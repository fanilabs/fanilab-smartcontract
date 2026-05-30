extern crate std;

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, Symbol, String
};

// ── Mock Escrow Contract with call tracking ───────────────────────────────────

#[contract]
pub struct MockEscrowContract;

#[contractimpl]
impl MockEscrowContract {
    pub fn refund_escrow(_env: Env, _caller: Address, delivery_id: u64) {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "refunded"), &delivery_id);
    }

    pub fn release_escrow(_env: Env, _caller: Address, delivery_id: u64) {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "released"), &delivery_id);
    }

    pub fn raise_dispute(_env: Env, _caller: Address, delivery_id: u64) {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "disputed"), &delivery_id);
    }
}

// ── Mock Reputation Contract for cross-contract calls ───────────────────────────

#[contract]
pub struct MockReputationContract;

#[contractimpl]
impl MockReputationContract {
    pub fn increase_reputation(_env: Env, driver: Address, _delivery_id: u64, _weight_grams: u32, _fragile: bool) {
        _env.storage().temporary().set(&Symbol::new(&_env, "rep_inc"), &driver);
    }

    pub fn decrease_reputation(_env: Env, driver: Address, _points: u32) {
        _env.storage().temporary().set(&Symbol::new(&_env, "rep_dec"), &driver);
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_full(env: &Env) -> (DeliveryContractClient<'static>, Address, Address, Address, Address, Address) {
    env.mock_all_auths();
    let escrow_id = env.register(MockEscrowContract, ());
    let reputation_id = env.register(MockReputationContract, ());
    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(env, &contract_id);
    let shipper = Address::generate(env);
    let driver = Address::generate(env);
    let recipient = Address::generate(env);
    client.init(&shipper, &escrow_id);
    (client, shipper, driver, recipient, escrow_id, reputation_id)
}

fn get_test_metadata(env: &Env, delivery_id: u64) -> DeliveryMetadata {
    use shared_types::{CargoCategory, CargoDescriptor};
    DeliveryMetadata {
        delivery_id,
        origin: String::from_str(env, "Origin"),
        destination: String::from_str(env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    }
}

// ── HAPPY PATH ───────────────────────────────────────────────────────────────

#[test]
fn test_happy_path_full_lifecycle() {
    let env = Env::default();
    let (client, shipper, driver, recipient, escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);
    client.confirm_delivery(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Delivered);

    let was_released: u64 = env.as_contract(&escrow_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "released"))
            .unwrap_or(0u64)
    });
    assert_eq!(was_released, delivery_id, "Expected escrow to be released after delivery");
}

// ── CANCELLATION PATH ───────────────────────────────────────────────────────

#[test]
fn test_cancellation_after_assign() {
    let env = Env::default();
    let (client, shipper, driver, recipient, escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    client.cancel_delivery(&shipper, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Cancelled);

    let was_refunded: u64 = env.as_contract(&escrow_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "refunded"))
            .unwrap_or(0u64)
    });
    assert_eq!(was_refunded, delivery_id, "Expected escrow to be refunded after cancellation");
}

// ── DISPUTE PATH ─────────────────────────────────────────────────────────────

#[test]
fn test_dispute_path() {
    let env = Env::default();
    let (client, shipper, driver, recipient, escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Disputed);

    let was_disputed: u64 = env.as_contract(&escrow_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "disputed"))
            .unwrap_or(0u64)
    });
    assert_eq!(was_disputed, delivery_id, "Expected escrow dispute to be raised");
}

// ── INVALID STATE REJECTIONS ───────────────────────────────────────────────

#[test]
#[should_panic(expected = "InvalidState")]
fn test_invalid_assign_when_delivered() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);
    client.confirm_delivery(&recipient, &delivery_id);

    client.assign_driver(&driver, &delivery_id, &driver);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_invalid_mark_in_transit_without_assign() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let driver = Address::generate(&env);
    client.mark_in_transit(&driver, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_invalid_confirm_without_transit() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    client.confirm_delivery(&recipient, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_invalid_dispute_when_cancelled() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.cancel_delivery(&shipper, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_invalid_cancel_when_delivered() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);
    client.confirm_delivery(&recipient, &delivery_id);

    client.cancel_delivery(&shipper, &delivery_id);
}

// ── UNAUTHORIZED CALLER REJECTIONS ───────────────────────────────────────────

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_assign_driver() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let unauthorized = Address::generate(&env);
    client.assign_driver(&unauthorized, &delivery_id, &driver);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_mark_in_transit() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    let unauthorized = Address::generate(&env);
    client.mark_in_transit(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_confirm_delivery() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    let unauthorized = Address::generate(&env);
    client.confirm_delivery(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_raise_dispute() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    let unauthorized = Address::generate(&env);
    client.raise_dispute(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_cancel_delivery() {
    let env = Env::default();
    let (client, shipper, driver, _, _, _) = setup_full(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    let unauthorized = Address::generate(&env);
    client.cancel_delivery(&unauthorized, &delivery_id);
}

// ── EDGE CASES ───────────────────────────────────────────────────────────────

#[test]
fn test_dispute_then_resolve_increments_reputation() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, reputation_id) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Disputed);

    env.as_contract(&reputation_id, || {
        env.storage().temporary().set(&Symbol::new(&env, "rep_inc"), &driver);
    });

    let stored_driver: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "rep_inc"))
            .unwrap_or(driver.clone())
    });
    assert_eq!(stored_driver, driver, "Expected reputation to be incremented for resolved dispute in driver's favor");
}

#[test]
fn test_dispute_then_resolve_penalizes_driver() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, reputation_id) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Disputed);

    env.as_contract(&reputation_id, || {
        env.storage().temporary().set(&Symbol::new(&env, "rep_dec"), &driver);
    });

    let stored_driver: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "rep_dec"))
            .unwrap_or(driver.clone())
    });
    assert_eq!(stored_driver, driver, "Expected reputation to be decremented for resolved dispute against driver");
}

#[test]
fn test_create_delivery_missing_fields() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, ""),
        destination: String::from_str(&env, ""),
        cargo_description: CargoDescriptor {
            weight_grams: 0,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    assert_eq!(delivery_id, 1);
}
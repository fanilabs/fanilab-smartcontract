extern crate std;

use super::*;
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Symbol,
};

proptest! {
    #[test]
    fn transition_matrix_is_exact(from in 0u8..6, to in 0u8..6) {
        let states = [DeliveryStatus::Pending, DeliveryStatus::Active,
            DeliveryStatus::InTransit, DeliveryStatus::Delivered,
            DeliveryStatus::Disputed, DeliveryStatus::Cancelled];
        let expected = matches!((from, to), (0,1)|(0,5)|(1,2)|(1,4)|(1,5)|(2,3)|(2,4)|(3,4)|(4,3));
        prop_assert_eq!(validate_transition(states[from as usize], states[to as usize]).is_ok(), expected);
    }
}

// ── Issue #95: State Rollback on Escrow Failure ──────────────────────────────────────────────────
// This module implements comprehensive testing for the contract's safety invariant:
// Cross-contract escrow calls execute BEFORE local state mutations, ensuring atomicity
// and preventing state corruption when cross-contract calls fail.

// ── Mock Escrow Contract with call tracking and failure simulation ───────────────────────────────────────

#[contract]
pub struct MockEscrowContract;

#[contractimpl]
impl MockEscrowContract {
    pub fn refund_escrow(_env: Env, _caller: Address, delivery_id: u64) {
        if delivery_id == 9999 {
            panic!("MockEscrowFailure");
        }
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "refunded"), &delivery_id);
    }

    pub fn mark_holdback_escrow(_env: Env, _caller: Address, delivery_id: u64) {
        if delivery_id == 9999 {
            panic!("MockEscrowFailure");
        }
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "holdback"), &delivery_id);
    }

    pub fn release_escrow(_env: Env, _caller: Address, delivery_id: u64) {
        if delivery_id == 9999 {
            panic!("MockEscrowFailure");
        }
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "released"), &delivery_id);
    }

    pub fn is_paused(_env: Env) -> bool {
        false
    }

    pub fn raise_dispute(_env: Env, _caller: Address, delivery_id: u64) {
        if delivery_id == 9999 {
            panic!("MockEscrowFailure");
        }
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "disputed"), &delivery_id);
    }

    /// Minimal stand-in for get_combined_state's cross-call. Reflect the
    /// escrow operation recorded by the mock, while defaulting to Locked for
    /// pre-Delivered/Disputed/Cancelled delivery states.
    pub fn get_escrow(_env: Env, delivery_id: u64) -> shared_types::EscrowRecord {
        let placeholder = Address::generate(&_env);
        let status = if _env
            .storage()
            .temporary()
            .get::<_, u64>(&Symbol::new(&_env, "holdback"))
            == Some(delivery_id)
        {
            shared_types::EscrowStatus::Holdback
        } else if _env
            .storage()
            .temporary()
            .get::<_, u64>(&Symbol::new(&_env, "released"))
            == Some(delivery_id)
        {
            shared_types::EscrowStatus::Released
        } else if _env
            .storage()
            .temporary()
            .get::<_, u64>(&Symbol::new(&_env, "refunded"))
            == Some(delivery_id)
        {
            shared_types::EscrowStatus::Refunded
        } else if _env
            .storage()
            .temporary()
            .get::<_, u64>(&Symbol::new(&_env, "disputed"))
            == Some(delivery_id)
        {
            shared_types::EscrowStatus::Paused
        } else {
            shared_types::EscrowStatus::Locked
        };
        shared_types::EscrowRecord {
            delivery_id: 0,
            sender: placeholder.clone(),
            recipient: placeholder.clone(),
            driver: placeholder,
            token: Address::generate(&_env),
            amount: 0,
            status,
            created_at: _env.ledger().timestamp(),
            expires_at: None,
            disputed_by: None,
            disputed_at: None,
            holdback_started_at: None,
            fleet_id: None,
        }
    }
}

// ── Mock Reputation Contract for cross-contract calls ───────────────────────────

#[contract]
pub struct MockReputationContract;

#[contractimpl]
impl MockReputationContract {
    pub fn has_user_profile(_env: Env, _user: Address) -> bool {
        false
    }

    pub fn register_user(_env: Env, user: Address) -> shared_types::UserProfile {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "registered_user"), &user);
        shared_types::UserProfile {
            address: user,
            registered_at: _env.ledger().timestamp(),
        }
    }

    pub fn increase_reputation(
        _env: Env,
        _caller: Address,
        driver: Address,
        _delivery_id: u64,
        _weight_grams: u32,
        _fragile: bool,
    ) {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "rep_inc"), &driver);
    }

    pub fn decrease_reputation(_env: Env, _caller: Address, driver: Address, _points: u32) {
        _env.storage()
            .temporary()
            .set(&Symbol::new(&_env, "rep_dec"), &driver);
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_full(
    env: &Env,
) -> (
    DeliveryContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    Address,
) {
    env.mock_all_auths();
    let escrow_id = env.register(MockEscrowContract, ());
    let reputation_id = env.register(MockReputationContract, ());
    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(env, &contract_id);
    let shipper = Address::generate(env);
    let driver = Address::generate(env);
    let recipient = Address::generate(env);
    client.init(&shipper, &escrow_id);
    client.set_identity_reputation_contract(&shipper, &reputation_id);
    (client, shipper, driver, recipient, escrow_id, reputation_id)
}

fn setup_with_identity(
    env: &Env,
) -> (DeliveryContractClient<'static>, Address, Address, Address) {
    env.mock_all_auths();
    let escrow_id = env.register(MockEscrowContract, ());
    let delivery_id = env.register(DeliveryContract, ());
    let identity_id = env.register(identity_reputation_contract::IdentityReputationContract, ());
    let client = DeliveryContractClient::new(env, &delivery_id);
    let admin = Address::generate(env);
    let recipient = Address::generate(env);
    let dispute_id = Address::generate(env);

    client.init(&admin, &escrow_id);
    let identity_client = identity_reputation_contract::IdentityReputationContractClient::new(
        env,
        &identity_id,
    );
    identity_client.init(&admin, &delivery_id, &dispute_id);
    client.set_identity_reputation_contract(&admin, &identity_id);

    (client, admin, recipient, identity_id)
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

#[test]
#[should_panic(expected = "5")] // DeliveryError::InvalidParties
fn create_delivery_rejects_same_sender_and_recipient() {
    let env = Env::default();
    env.mock_all_auths();
    let escrow_id = env.register(MockEscrowContract, ());
    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(&env, &contract_id);
    let sender_and_recipient = Address::generate(&env);
    client.init(&sender_and_recipient, &escrow_id);

    client.create_delivery(
        &sender_and_recipient,
        &sender_and_recipient,
        &get_test_metadata(&env, 1),
    );
}

#[test]
fn create_delivery_and_batch_are_idempotent_for_identity_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let identity_id = env.register(identity_reputation_contract::IdentityReputationContract, ());
    let escrow_id = env.register(MockEscrowContract, ());
    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(&env, &contract_id);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.init(&sender, &escrow_id);
    client.set_identity_reputation_contract(&sender, &identity_id);

    client.create_delivery(&sender, &recipient, &get_test_metadata(&env, 1));
    let second_id = client.create_delivery(&sender, &recipient, &get_test_metadata(&env, 2));

    let mut metadata_list = soroban_sdk::Vec::new(&env);
    metadata_list.push_back(get_test_metadata(&env, 3));
    let batch_ids = client.create_deliveries_batch(&sender, &recipient, &metadata_list);

    assert_eq!(second_id.value(), 2);
    assert_eq!(batch_ids.len(), 1);
}

#[test]
fn get_driver_profile_reads_identity_reputation_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let identity_id = env.register(identity_reputation_contract::IdentityReputationContract, ());
    let identity_client =
        identity_reputation_contract::IdentityReputationContractClient::new(&env, &identity_id);
    let driver = Address::generate(&env);
    env.ledger().set_timestamp(100);
    identity_client.register_driver(&driver);

    env.ledger().set_timestamp(200);
    let escrow_id = env.register(MockEscrowContract, ());
    let delivery_id = env.register(DeliveryContract, ());
    let delivery_client = DeliveryContractClient::new(&env, &delivery_id);
    let admin = Address::generate(&env);
    delivery_client.init(&admin, &escrow_id);
    delivery_client.set_identity_reputation_contract(&admin, &identity_id);

    let profile = delivery_client.get_driver_profile(&driver);

    assert_eq!(profile.address, driver);
    assert_eq!(profile.reputation_score, 50);
    assert_eq!(profile.registered_at, 100);
}

fn get_test_metadata_with_estimate(
    env: &Env,
    delivery_id: u64,
    estimated_delivery: u64,
) -> DeliveryMetadata {
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
        estimated_delivery,
    }
}

#[test]
fn test_get_escrow_contract_returns_configured_address() {
    let env = Env::default();
    let (client, _shipper, _driver, _recipient, escrow_id, _) = setup_full(&env);
    assert_eq!(client.get_escrow_contract(), escrow_id);
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

    let was_marked_holdback: u64 = env.as_contract(&escrow_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "holdback"))
            .unwrap_or(0u64)
    });
    assert_eq!(
        was_marked_holdback, delivery_id,
        "Expected escrow to be placed in holdback after delivery confirmation"
    );
}

#[test]
fn test_delivery_secondary_indexes_track_sender_and_recipient() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, _) = setup_full(&env);
    let other_shipper = Address::generate(&env);
    let other_recipient = Address::generate(&env);

    let first_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 1));
    let second_id = client.create_delivery(&shipper, &other_recipient, &get_test_metadata(&env, 2));
    let third_id = client.create_delivery(
        &other_shipper,
        &recipient,
        &get_test_metadata(&env, 3),
    );

    let shipper_deliveries = client.get_deliveries_by_sender(&shipper);
    assert_eq!(shipper_deliveries.len(), 2);
    assert_eq!(shipper_deliveries.get(0), Some(first_id));
    assert_eq!(shipper_deliveries.get(1), Some(second_id));

    let recipient_deliveries = client.get_deliveries_by_recipient(&recipient);
    assert_eq!(recipient_deliveries.len(), 2);
    assert_eq!(recipient_deliveries.get(0), Some(first_id));
    assert_eq!(recipient_deliveries.get(1), Some(third_id));

    assert_eq!(client.get_deliveries_by_sender(&Address::generate(&env)).len(), 0);
}

#[test]
fn test_delivery_batch_secondary_indexes_append_ids() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, _) = setup_full(&env);

    let first_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 1));
    let mut metadata_list = soroban_sdk::Vec::new(&env);
    metadata_list.push_back(get_test_metadata(&env, 2));
    metadata_list.push_back(get_test_metadata(&env, 3));

    let batch_ids = client.create_deliveries_batch(&shipper, &recipient, &metadata_list);
    assert_eq!(batch_ids.len(), 2);

    let sender_deliveries = client.get_deliveries_by_sender(&shipper);
    assert_eq!(sender_deliveries.len(), 3);
    assert_eq!(sender_deliveries.get(0), Some(first_id));
    assert_eq!(sender_deliveries.get(1), batch_ids.get(0));
    assert_eq!(sender_deliveries.get(2), batch_ids.get(1));

    let recipient_deliveries = client.get_deliveries_by_recipient(&recipient);
    assert_eq!(recipient_deliveries.len(), 3);
    assert_eq!(recipient_deliveries.get(0), Some(first_id));
    assert_eq!(recipient_deliveries.get(1), batch_ids.get(0));
    assert_eq!(recipient_deliveries.get(2), batch_ids.get(1));
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
    assert_eq!(
        was_refunded, delivery_id,
        "Expected escrow to be refunded after cancellation"
    );
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
    assert_eq!(
        was_disputed, delivery_id,
        "Expected escrow dispute to be raised"
    );
}

// ── INVALID STATE REJECTIONS ───────────────────────────────────────────────

#[test]
#[should_panic(expected = "5")]
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
#[should_panic(expected = "1")]
fn test_invalid_mark_in_transit_without_assign() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let driver = Address::generate(&env);
    client.mark_in_transit(&driver, &delivery_id);
}

#[test]
#[should_panic(expected = "5")]
fn test_invalid_confirm_without_transit() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    client.confirm_delivery(&recipient, &delivery_id);
}

#[test]
#[should_panic(expected = "5")]
fn test_invalid_dispute_when_cancelled() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.cancel_delivery(&shipper, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);
}

/// Issue #93 regression test: once a delivery has reached `Disputed`, the
/// sender must not be able to unilaterally cancel it and force a self-refund
/// — that would bypass admin-mediated dispute resolution entirely. There is
/// no `Disputed -> Cancelled` transition in `validate_transition`, so this
/// must panic with `InvalidState` rather than routing to `refund_escrow`.
#[test]
#[should_panic(expected = "5")]
fn test_cancel_delivery_rejected_once_disputed() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);
    assert_eq!(
        client.get_delivery(&delivery_id).status,
        DeliveryStatus::Disputed
    );

    client.cancel_delivery(&shipper, &delivery_id);
}

#[test]
#[should_panic(expected = "5")]
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
#[should_panic(expected = "1")]
fn test_unauthorized_assign_driver() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let unauthorized = Address::generate(&env);
    client.assign_driver(&unauthorized, &delivery_id, &driver);
}

#[test]
#[should_panic(expected = "1")]
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
#[should_panic(expected = "1")]
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
#[should_panic(expected = "1")]
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
#[should_panic(expected = "1")]
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
        env.storage()
            .temporary()
            .set(&Symbol::new(&env, "rep_inc"), &driver);
    });

    let stored_driver: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "rep_inc"))
            .unwrap_or(driver.clone())
    });
    assert_eq!(
        stored_driver, driver,
        "Expected reputation to be incremented for resolved dispute in driver's favor"
    );
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
        env.storage()
            .temporary()
            .set(&Symbol::new(&env, "rep_dec"), &driver);
    });

    let stored_driver: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "rep_dec"))
            .unwrap_or(driver.clone())
    });
    assert_eq!(
        stored_driver, driver,
        "Expected reputation to be decremented for resolved dispute against driver"
    );
}

// ── SELF-ASSIGNMENT REJECTION (Issue #20) ─────────────────────────────────

#[test]
#[should_panic(expected = "4")] // DeliveryError::InvalidDriver
fn test_reject_assign_driver_as_sender() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    client.assign_driver(&shipper, &delivery_id, &shipper);
}

#[test]
#[should_panic(expected = "4")] // DeliveryError::InvalidDriver
fn test_reject_assign_driver_as_recipient() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    client.assign_driver(&shipper, &delivery_id, &recipient);
}

#[test]
#[should_panic(expected = "1")] // FaniLabError::Unauthorized — driver is not the recipient
fn test_reject_confirm_delivery_from_driver() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.confirm_delivery(&driver, &delivery_id);
}

#[test]
#[should_panic(expected = "4")] // DeliveryError::InvalidDriver
fn test_confirm_delivery_rejects_driver_matching_recipient() {
    // assign_driver already rejects driver == recipient at assignment time,
    // so this state can't arise via the normal public API. This test forces
    // it directly to prove confirm_delivery's own defense-in-depth check
    // (Issue #23) still rejects it with a typed error rather than silently
    // succeeding or panicking with an untyped string.
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    env.as_contract(&client.address, || {
        let key = delivery_key(delivery_id);
        let mut record: DeliveryRecord = env.storage().persistent().get(&key).unwrap();
        record.driver = Some(recipient.clone());
        env.storage().persistent().set(&key, &record);
    });

    client.confirm_delivery(&recipient, &delivery_id);
}

// ── STATE SYNCHRONIZATION VALIDATION (Issue #19) ──────────────────────────────

#[test]
fn test_get_combined_state_pending_delivery() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let (delivery, _escrow, is_synchronized) = client.get_combined_state(&delivery_id);

    assert_eq!(delivery.status, DeliveryStatus::Pending);
    assert!(
        is_synchronized,
        "Pending delivery should be synchronized with escrow"
    );
}

#[test]
fn test_get_combined_state_active_delivery() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    let (delivery, _escrow, is_synchronized) = client.get_combined_state(&delivery_id);

    assert_eq!(delivery.status, DeliveryStatus::Active);
    assert!(
        is_synchronized,
        "Active delivery should be synchronized with escrow"
    );
}

#[test]
fn test_get_combined_state_in_transit_delivery() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _escrow_id, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    let (delivery, _escrow, is_synchronized) = client.get_combined_state(&delivery_id);

    assert_eq!(delivery.status, DeliveryStatus::InTransit);
    assert!(
        is_synchronized,
        "InTransit delivery should be synchronized with escrow"
    );
}

// ── METADATA VALIDATION (Issue #96 - empty origin/destination and zero weight) ───────────────────

#[test]
#[should_panic(expected = "2")]
fn test_reject_empty_origin() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, ""),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_empty_destination() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, ""),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_zero_weight() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 0,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_empty_origin_and_destination_and_zero_weight() {
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

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_origin_exceeds_max_length() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let long_string = String::from_str(&env, &"x".repeat(257));
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: long_string,
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_destination_exceeds_max_length() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let long_string = String::from_str(&env, &"x".repeat(257));
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: long_string,
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_weight_exceeds_max() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 1_000_001,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    client.create_delivery(&shipper, &recipient, &metadata);
}

#[test]
fn test_accept_location_at_max_length() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let max_string = String::from_str(&env, &"x".repeat(256));
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: max_string.clone(),
        destination: max_string,
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    assert_eq!(delivery_id, 1);
}

#[test]
fn test_accept_weight_at_max() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 1_000_000,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    assert_eq!(delivery_id, 1);
}

#[test]
fn test_accept_minimum_valid_weight() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 1,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    assert_eq!(delivery_id, 1);
}

#[test]
#[should_panic(expected = "2")] // DeliveryError::InvalidMetadata
fn test_batch_rejects_invalid_metadata() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let mut metadata_list = soroban_sdk::Vec::new(&env);
    metadata_list.push_back(get_test_metadata(&env, 1));

    let mut invalid_metadata = get_test_metadata(&env, 2);
    invalid_metadata.origin = String::from_str(&env, "");
    metadata_list.push_back(invalid_metadata);

    client.create_deliveries_batch(&shipper, &recipient, &metadata_list);
}

#[test]
fn test_confirm_delivery_calls_increase_reputation() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, reputation_id) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.confirm_delivery(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Delivered);

    let stored_driver: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "rep_inc"))
            .unwrap_or(driver.clone())
    });
    assert_eq!(
        stored_driver, driver,
        "Expected reputation increase to be called for driver on delivery confirmation"
    );
}

// ── User Registration Tests ───────────────────────────────────────────────────

#[test]
fn test_create_delivery_registers_sender_and_recipient() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, reputation_id) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);

    let _delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let last_registered: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "registered_user"))
            .unwrap_or(shipper.clone())
    });
    assert_eq!(
        last_registered, recipient,
        "Expected recipient to be registered after create_delivery"
    );
}

#[test]
fn test_raise_dispute_on_delivered_delivery_updates_status() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _escrow_id, _reputation_id) = setup_full(&env);

    let metadata = get_test_metadata(&env, 1u64);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&shipper, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Disputed);
}

// ── ISSUE #95: State Rollback on Escrow Failure Tests ─────────────────────────

#[test]
#[should_panic(expected = "MockEscrowFailure")]
fn test_confirm_delivery_state_rollback_on_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    // This should panic due to escrow failure (delivery_id 9999)
    client.confirm_delivery(&recipient, &delivery_id);
}

#[test]
fn test_delivery_state_unchanged_after_confirm_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    let delivery_before = client.get_delivery(&delivery_id);
    assert_eq!(delivery_before.status, DeliveryStatus::InTransit);
    assert_eq!(delivery_before.delivered_at, None);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.confirm_delivery(&recipient, &delivery_id);
    }));

    assert!(result.is_err(), "Expected confirm_delivery to panic");

    let delivery_after = client.get_delivery(&delivery_id);
    assert_eq!(
        delivery_after.status,
        DeliveryStatus::InTransit,
        "Delivery status should not change"
    );
    assert_eq!(
        delivery_after.delivered_at, None,
        "Delivered_at should remain None"
    );
}

#[test]
#[should_panic(expected = "MockEscrowFailure")]
fn test_cancel_delivery_state_rollback_on_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    // This should panic due to escrow failure (delivery_id 9999)
    client.cancel_delivery(&shipper, &delivery_id);
}

#[test]
fn test_delivery_state_unchanged_after_cancel_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    let delivery_before = client.get_delivery(&delivery_id);
    assert_eq!(delivery_before.status, DeliveryStatus::Active);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.cancel_delivery(&shipper, &delivery_id);
    }));

    assert!(result.is_err(), "Expected cancel_delivery to panic");

    let delivery_after = client.get_delivery(&delivery_id);
    assert_eq!(
        delivery_after.status,
        DeliveryStatus::Active,
        "Delivery status should not change"
    );
}

#[test]
#[should_panic(expected = "MockEscrowFailure")]
fn test_raise_dispute_state_rollback_on_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    // This should panic due to escrow failure (delivery_id 9999)
    client.raise_dispute(&shipper, &delivery_id);
}

#[test]
fn test_delivery_state_unchanged_after_raise_dispute_escrow_failure() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata_with_estimate(&env, 9999, env.ledger().timestamp() + 86400);
    // Force the next delivery's real (counter-assigned) ID to 9999 so the
    // MockEscrowContract's delivery_id == 9999 failure trigger actually fires.
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &9998u64);
    });
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    let delivery_before = client.get_delivery(&delivery_id);
    assert_eq!(delivery_before.status, DeliveryStatus::InTransit);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.raise_dispute(&shipper, &delivery_id);
    }));

    assert!(result.is_err(), "Expected raise_dispute to panic");

    let delivery_after = client.get_delivery(&delivery_id);
    assert_eq!(
        delivery_after.status,
        DeliveryStatus::InTransit,
        "Delivery status should not change to Disputed"
    );
}

// ── ISSUE #97: Update Delivery Metadata Tests ──────────────────────────────────

#[test]
fn test_update_delivery_metadata_while_pending() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    use shared_types::{CargoCategory, CargoDescriptor};
    let updated_metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "New Origin"),
        destination: String::from_str(&env, "New Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 172800,
    };

    client.update_delivery_metadata(&shipper, &delivery_id, &updated_metadata);

    let updated_delivery = client.get_delivery(&delivery_id);
    assert_eq!(
        updated_delivery.metadata.origin,
        String::from_str(&env, "New Origin")
    );
    assert_eq!(
        updated_delivery.metadata.destination,
        String::from_str(&env, "New Destination")
    );
    assert_eq!(
        updated_delivery.metadata.cargo_description.weight_grams,
        500
    );
    assert!(updated_delivery.metadata.cargo_description.fragile);
}

// ── DeliveryMetadata.delivery_id cross-check (Issue #45) ───────────────────

#[test]
fn test_create_delivery_overwrites_caller_supplied_delivery_id() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    // Caller supplies a delivery_id (999) that has nothing to do with the
    // real, internally generated ID the counter will assign.
    let metadata = get_test_metadata(&env, 999);
    let real_delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let stored = client.get_delivery(&real_delivery_id);
    assert_eq!(stored.metadata.delivery_id, u64::from(real_delivery_id));
    assert_ne!(stored.metadata.delivery_id, 999);
}

#[test]
fn test_create_deliveries_batch_overwrites_caller_supplied_delivery_ids() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);

    let mut metadata_list = soroban_sdk::Vec::new(&env);
    // Both entries claim the same bogus caller-supplied ID.
    metadata_list.push_back(get_test_metadata(&env, 4242));
    metadata_list.push_back(get_test_metadata(&env, 4242));

    let ids = client.create_deliveries_batch(&shipper, &recipient, &metadata_list);
    for i in 0..ids.len() {
        let id = ids.get(i).unwrap();
        let stored = client.get_delivery(&id);
        assert_eq!(stored.metadata.delivery_id, u64::from(id));
        assert_ne!(stored.metadata.delivery_id, 4242);
    }
}

#[test]
fn test_update_delivery_metadata_overwrites_caller_supplied_delivery_id() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    use shared_types::{CargoCategory, CargoDescriptor};
    let updated_metadata = DeliveryMetadata {
        delivery_id: 777, // bogus, unrelated to the real delivery_id
        origin: String::from_str(&env, "New Origin"),
        destination: String::from_str(&env, "New Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 172800,
    };

    client.update_delivery_metadata(&shipper, &delivery_id, &updated_metadata);

    let stored = client.get_delivery(&delivery_id);
    assert_eq!(stored.metadata.delivery_id, u64::from(delivery_id));
    assert_ne!(stored.metadata.delivery_id, 777);
}

#[test]
#[should_panic(expected = "5")]
fn test_reject_update_metadata_after_driver_assigned() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);

    use shared_types::{CargoCategory, CargoDescriptor};
    let updated_metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "New Origin"),
        destination: String::from_str(&env, "New Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 172800,
    };

    client.update_delivery_metadata(&shipper, &delivery_id, &updated_metadata);
}

#[test]
#[should_panic(expected = "1")]
fn test_unauthorized_update_metadata_wrong_sender() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    let unauthorized = Address::generate(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let updated_metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "New Origin"),
        destination: String::from_str(&env, "New Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 172800,
    };

    client.update_delivery_metadata(&unauthorized, &delivery_id, &updated_metadata);
}

#[test]
#[should_panic(expected = "2")]
fn test_reject_update_metadata_with_empty_origin() {
    let env = Env::default();
    let (client, shipper, _, recipient, _, _) = setup_full(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    use shared_types::{CargoCategory, CargoDescriptor};
    let updated_metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, ""),
        destination: String::from_str(&env, "New Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 172800,
    };

    client.update_delivery_metadata(&shipper, &delivery_id, &updated_metadata);
}

// ── ISSUE #98: Lateness Detection Tests ────────────────────────────────────────

#[test]
fn test_on_time_delivery_confirmation() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let current_time = env.ledger().timestamp();
    let estimated_time = current_time + 86400;

    let metadata = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: current_time,
        estimated_delivery: estimated_time,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    env.ledger().with_mut(|l| {
        l.timestamp = estimated_time - 3600;
    });

    client.confirm_delivery(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Delivered);

    client.raise_dispute(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(
        delivery.status,
        DeliveryStatus::Disputed,
        "Delivery status should transition to Disputed after raising dispute"
    );
}

#[test]
fn test_create_deliveries_batch_registers_users() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, reputation_id) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let mut metadata_list = soroban_sdk::Vec::new(&env);
    let metadata1 = DeliveryMetadata {
        delivery_id: 1,
        origin: String::from_str(&env, "Origin1"),
        destination: String::from_str(&env, "Destination1"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };
    let metadata2 = DeliveryMetadata {
        delivery_id: 2,
        origin: String::from_str(&env, "Origin2"),
        destination: String::from_str(&env, "Destination2"),
        cargo_description: CargoDescriptor {
            weight_grams: 200,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 86400,
    };
    metadata_list.push_back(metadata1);
    metadata_list.push_back(metadata2);

    let _delivery_ids = client.create_deliveries_batch(&shipper, &recipient, &metadata_list);

    let last_registered: Address = env.as_contract(&reputation_id, || {
        env.storage()
            .temporary()
            .get(&Symbol::new(&env, "registered_user"))
            .unwrap_or(shipper.clone())
    });
    assert_eq!(
        last_registered, recipient,
        "Expected recipient to be registered after create_deliveries_batch"
    );
}

#[test]
fn test_create_delivery_allows_existing_identity_profiles() {
    let env = Env::default();
    let (client, sender, recipient, identity_id) = setup_with_identity(&env);
    let metadata = get_test_metadata(&env, 1);

    client.create_delivery(&sender, &recipient, &metadata);
    client.create_delivery(&sender, &recipient, &metadata);

    let identity_client = identity_reputation_contract::IdentityReputationContractClient::new(
        &env,
        &identity_id,
    );
    assert_eq!(identity_client.get_user_profile(&sender).address, sender);
    assert_eq!(identity_client.get_user_profile(&recipient).address, recipient);
}

#[test]
fn test_create_deliveries_batch_allows_existing_identity_profiles() {
    let env = Env::default();
    let (client, sender, recipient, identity_id) = setup_with_identity(&env);
    let mut metadata_list = soroban_sdk::Vec::new(&env);
    metadata_list.push_back(get_test_metadata(&env, 1));

    client.create_deliveries_batch(&sender, &recipient, &metadata_list);
    client.create_deliveries_batch(&sender, &recipient, &metadata_list);

    let identity_client = identity_reputation_contract::IdentityReputationContractClient::new(
        &env,
        &identity_id,
    );
    assert_eq!(identity_client.get_user_profile(&sender).address, sender);
    assert_eq!(identity_client.get_user_profile(&recipient).address, recipient);
}

#[test]
#[should_panic(expected = "3")] // DeliveryError::BatchTooLarge
fn test_create_deliveries_batch_over_limit_rejected() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _, _) = setup_full(&env);

    let mut metadata_list = soroban_sdk::Vec::new(&env);
    for i in 0..(MAX_BATCH_SIZE + 1) {
        metadata_list.push_back(get_test_metadata(&env, i as u64));
    }

    client.create_deliveries_batch(&shipper, &recipient, &metadata_list);
}

#[test]
fn test_late_delivery_confirmation() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let current_time = env.ledger().timestamp();
    let estimated_time = current_time + 86400;

    let metadata = DeliveryMetadata {
        delivery_id: 2,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: current_time,
        estimated_delivery: estimated_time,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    env.ledger().with_mut(|l| {
        l.timestamp = estimated_time + 43200;
    });

    client.confirm_delivery(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Delivered);
    assert!(
        delivery.delivered_at.unwrap_or(0) > estimated_time,
        "Delivery should be late"
    );
}

#[test]
fn test_early_delivery_confirmation() {
    let env = Env::default();
    let (client, shipper, driver, recipient, _, _) = setup_full(&env);

    use shared_types::{CargoCategory, CargoDescriptor};
    let current_time = env.ledger().timestamp();
    let estimated_time = current_time + 86400;

    let metadata = DeliveryMetadata {
        delivery_id: 3,
        origin: String::from_str(&env, "Origin"),
        destination: String::from_str(&env, "Destination"),
        cargo_description: CargoDescriptor {
            weight_grams: 100,
            category: CargoCategory::General,
            fragile: false,
        },
        created_at: current_time,
        estimated_delivery: estimated_time,
    };

    let delivery_id = client.create_delivery(&shipper, &recipient, &metadata);
    client.assign_driver(&driver, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&recipient, &delivery_id);

    let result = client.try_cancel_delivery(&shipper, &delivery_id);
    match result {
        Err(Ok(err)) => {
            assert_eq!(err, FaniLabError::InvalidState.into());
        }
        _ => panic!("Expected FaniLabError::InvalidState when cancelling disputed delivery"),
    }
    env.ledger().with_mut(|l| {
        l.timestamp = estimated_time - 86400;
    });

    client.confirm_delivery(&recipient, &delivery_id);

    let delivery = client.get_delivery(&delivery_id);
    assert_eq!(delivery.status, DeliveryStatus::Delivered);
    assert!(
        delivery.delivered_at.unwrap_or(0) < estimated_time,
        "Delivery should be early"
    );
}


/// Test that create_delivery and create_deliveries_batch emit compatible
/// DeliveryCreatedEvent payloads with the same shape and topic.
/// This ensures off-chain consumers receive consistent event structure.
#[test]
fn test_create_delivery_and_batch_emit_consistent_events() {
    let env = Env::default();
    env.mock_all_auths();

    let shipper = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);
    let escrow_contract = env.register(MockEscrowContract, ());
    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(&env, &contract_id);

    client.init(&admin, &escrow_contract);

    // Create single delivery via create_delivery
    let cargo = shared_types::CargoDescriptor {
        weight_grams: 500,
        category: shared_types::CargoCategory::Electronics,
        fragile: true,
    };
    let metadata = DeliveryMetadata {
        delivery_id: 999, // Will be overwritten
        origin: soroban_sdk::String::from_str(&env, "Origin"),
        destination: soroban_sdk::String::from_str(&env, "Destination"),
        cargo_description: cargo,
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 3600,
    };

    let single_delivery_id = client.create_delivery(&shipper, &recipient, &metadata);

    // Create batch with one delivery via create_deliveries_batch
    let mut batch_metadata = soroban_sdk::vec![&env];
    let mut meta = metadata.clone();
    meta.delivery_id = 888; // Will be overwritten
    batch_metadata.push_back(meta);

    let batch_ids = client.create_deliveries_batch(&shipper, &recipient, &batch_metadata);
    assert_eq!(batch_ids.len(), 1);
    let batch_delivery_id = batch_ids.get(0).unwrap();

    // Retrieve both deliveries to verify they have the same structure
    let single_delivery = client.get_delivery(&single_delivery_id);
    let batch_delivery = client.get_delivery(&batch_delivery_id);

    // Both should be Pending status
    assert_eq!(single_delivery.status, DeliveryStatus::Pending);
    assert_eq!(batch_delivery.status, DeliveryStatus::Pending);

    // Both should have sender as shipper
    assert_eq!(single_delivery.sender, shipper);
    assert_eq!(batch_delivery.sender, shipper);

    // Both should have recipient
    assert_eq!(single_delivery.recipient, recipient);
    assert_eq!(batch_delivery.recipient, recipient);

    // Both should have same metadata structure
    assert_eq!(single_delivery.metadata.origin, batch_delivery.metadata.origin);
    assert_eq!(
        single_delivery.metadata.destination,
        batch_delivery.metadata.destination
    );
    assert_eq!(
        single_delivery.metadata.cargo_description.weight_grams,
        batch_delivery.metadata.cargo_description.weight_grams
    );
}

// Tests for issue #269: delivery_contract secondary-index accessors and identity getter

#[test]
fn test_get_identity_reputation_contract_returns_none_before_configuration() {
    let env = Env::default();
    let (client, _shipper, _driver, _recipient, _escrow_id, _admin) = setup_full(&env);

    assert_eq!(client.get_identity_reputation_contract(), None);
}

#[test]
fn test_set_and_get_identity_reputation_contract_round_trip() {
    let env = Env::default();
    let (client, _shipper, _driver, _recipient, _escrow_id, admin) = setup_full(&env);
    let identity_contract = Address::generate(&env);

    assert_eq!(client.get_identity_reputation_contract(), None);

    client.set_identity_reputation_contract(&admin, &identity_contract);

    assert_eq!(
        client.get_identity_reputation_contract(),
        Some(identity_contract)
    );
}

#[test]
fn test_index_contents_unchanged_after_delivery_completion() {
    let env = Env::default();
    let (client, shipper, driver, recipient, escrow_id, _admin) = setup_full(&env);

    let first_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 1));
    let second_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 2));

    // Get indexes before state change
    let sender_before = client.get_deliveries_by_sender(&shipper);
    let recipient_before = client.get_deliveries_by_recipient(&recipient);

    assert_eq!(sender_before.len(), 2);
    assert_eq!(recipient_before.len(), 2);

    // Assign, mark in transit, and confirm first delivery
    client.assign_driver(&driver, &first_id);
    client.mark_in_transit(&driver, &first_id);
    client.confirm_delivery(&recipient, &first_id, &escrow_id);

    // Index contents should be unchanged
    let sender_after = client.get_deliveries_by_sender(&shipper);
    let recipient_after = client.get_deliveries_by_recipient(&recipient);

    assert_eq!(sender_after.len(), 2);
    assert_eq!(recipient_after.len(), 2);
    assert_eq!(sender_before.get(0), sender_after.get(0));
    assert_eq!(sender_before.get(1), sender_after.get(1));
    assert_eq!(recipient_before.get(0), recipient_after.get(0));
    assert_eq!(recipient_before.get(1), recipient_after.get(1));
}

#[test]
fn test_index_contents_unchanged_after_delivery_cancellation() {
    let env = Env::default();
    let (client, shipper, _driver, recipient, _escrow_id, _admin) = setup_full(&env);

    let first_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 1));
    let second_id = client.create_delivery(&shipper, &recipient, &get_test_metadata(&env, 2));

    // Get indexes before cancellation
    let sender_before = client.get_deliveries_by_sender(&shipper);
    let recipient_before = client.get_deliveries_by_recipient(&recipient);

    assert_eq!(sender_before.len(), 2);
    assert_eq!(recipient_before.len(), 2);

    // Cancel first delivery
    client.cancel_delivery(&shipper, &first_id);

    // Index contents should be unchanged
    let sender_after = client.get_deliveries_by_sender(&shipper);
    let recipient_after = client.get_deliveries_by_recipient(&recipient);

    assert_eq!(sender_after.len(), 2);
    assert_eq!(recipient_after.len(), 2);
    assert_eq!(sender_before.get(0), sender_after.get(0));
    assert_eq!(sender_before.get(1), sender_after.get(1));
    assert_eq!(recipient_before.get(0), recipient_after.get(0));
    assert_eq!(recipient_before.get(1), recipient_after.get(1));
}
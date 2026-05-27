extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Symbol, TryFromVal, String
};
use shared_types::{DeliveryMetadata, CargoDescriptor, CargoCategory};

// ── Escrow mock ───────────────────────────────────────────────────────────────

#[contract]
pub struct MockEscrow;

#[contractimpl]
impl MockEscrow {
    pub fn refund_escrow(_env: Env, _caller: Address, delivery_id: DeliveryId) {
        if delivery_id == 999 {
            panic!("Escrow failure simulated");
        }
    }

    pub fn release_escrow(_env: Env, _caller: Address, _delivery_id: DeliveryId) {}

    pub fn raise_dispute(_env: Env, _caller: Address, delivery_id: DeliveryId) {
        if delivery_id == 777 {
            panic!("Escrow raise_dispute failure simulated");
        }
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

fn setup_test() -> (
    Env,
    DeliveryContractClient<'static>,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let driver = Address::generate(&env);
    let unauthorized = Address::generate(&env);

    let escrow_id = env.register(MockEscrow, ());
    client.init(&admin, &escrow_id);

    (env, client, admin, driver, unauthorized)
}

fn get_test_metadata(env: &Env, delivery_id: u64) -> DeliveryMetadata {
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

// ── Existing driver assignment tests ─────────────────────────────────────────

#[test]
fn test_successful_assignment_by_admin() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.assign_driver(&admin, &delivery_id, &driver);

    let events = env.events().all();
    std::println!("EVENTS LEN: {}", events.len());
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, client.address.clone());

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "driver_assigned"));

    let data: (DeliveryId, Address) =
        <(DeliveryId, Address)>::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, (delivery_id, driver.clone()));

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.driver, Some(driver.clone()));
    assert_eq!(delivery.status, DeliveryStatus::Active);
}

#[test]
fn test_successful_self_assignment_by_driver() {
    let (env, client, _, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.assign_driver(&driver, &delivery_id, &driver);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.driver, Some(driver));
    assert_eq!(delivery.status, DeliveryStatus::Active);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_unauthorized_caller_rejected() {
    let (env, client, _, driver, unauthorized) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.assign_driver(&unauthorized, &delivery_id, &driver);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_assignment_when_status_not_pending() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.assign_driver(&admin, &delivery_id, &driver);

    let another_driver = Address::generate(&env);
    client.assign_driver(&admin, &delivery_id, &another_driver);
}

// ── Existing cancel delivery tests ───────────────────────────────────────────

#[test]
fn test_cancel_delivery_pending() {
    let (env, client, _admin, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.cancel_delivery(&sender, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.status, DeliveryStatus::Cancelled);
}

#[test]
fn test_cancel_delivery_active() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.cancel_delivery(&sender, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.status, DeliveryStatus::Cancelled);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_cancel_delivery_unauthorized() {
    let (env, client, _admin, _, unauthorized) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.cancel_delivery(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_cancel_delivery_invalid_state() {
    let (env, client, _admin, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    client.cancel_delivery(&sender, &delivery_id);
    client.cancel_delivery(&sender, &delivery_id);
}

#[test]
#[should_panic(expected = "Escrow failure simulated")]
fn test_cancel_delivery_escrow_failure() {
    let (env, client, _admin, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 999);
    
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &998u64);
    });

    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    assert_eq!(delivery_id, 999);

    client.cancel_delivery(&sender, &delivery_id);
}

// ── Existing create delivery tests ───────────────────────────────────────────

#[test]
fn test_create_delivery_success_and_storage() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);

    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    assert_eq!(delivery_id, 1);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.delivery_id, delivery_id);
    assert_eq!(delivery.sender, sender);
    assert_eq!(delivery.driver, None);
    assert_eq!(delivery.status, DeliveryStatus::Pending);
    assert_eq!(delivery.recipient, recipient);
    assert_eq!(delivery.transit_started_at, None);
}

#[test]
fn test_create_delivery_incrementing_ids_and_persistence() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);

    let id1 = client.create_delivery(&sender, &recipient, &metadata);
    let id2 = client.create_delivery(&sender, &recipient, &metadata);
    let id3 = client.create_delivery(&sender, &recipient, &metadata);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
#[should_panic]
fn test_double_init() {
    let (env, client, admin, _, _) = setup_test();
    let escrow = Address::generate(&env);
    client.init(&admin, &escrow);
}

#[test]
fn test_init_state_and_event() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(DeliveryContract, ());
    let client = DeliveryContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let escrow = Address::generate(&env);

    client.init(&admin, &escrow);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, contract_id);

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "DeliveryContractInitialized"));

    let data: (Address, Address) = <(Address, Address)>::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, (admin, escrow));

    let counter: u64 = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get(&DataKey::DeliveryCounter)
            .unwrap()
    });
    assert_eq!(counter, 0);
}

// ── Issue #25: State machine validate_transition tests ───────────────────────

#[test]
fn test_validate_transition_all_valid_transitions() {
    assert!(validate_transition(DeliveryStatus::Pending, DeliveryStatus::Active).is_ok());
    assert!(validate_transition(DeliveryStatus::Pending, DeliveryStatus::Cancelled).is_ok());
    assert!(validate_transition(DeliveryStatus::Active, DeliveryStatus::InTransit).is_ok());
    assert!(validate_transition(DeliveryStatus::Active, DeliveryStatus::Disputed).is_ok());
    assert!(validate_transition(DeliveryStatus::Active, DeliveryStatus::Cancelled).is_ok());
    assert!(validate_transition(DeliveryStatus::InTransit, DeliveryStatus::Delivered).is_ok());
    assert!(validate_transition(DeliveryStatus::InTransit, DeliveryStatus::Disputed).is_ok());
    assert!(validate_transition(DeliveryStatus::Disputed, DeliveryStatus::Delivered).is_ok());
    assert!(validate_transition(DeliveryStatus::Disputed, DeliveryStatus::Cancelled).is_ok());
}

#[test]
fn test_validate_transition_all_invalid_transitions() {
    // Pending cannot skip ahead
    assert!(validate_transition(DeliveryStatus::Pending, DeliveryStatus::InTransit).is_err());
    assert!(validate_transition(DeliveryStatus::Pending, DeliveryStatus::Delivered).is_err());
    assert!(validate_transition(DeliveryStatus::Pending, DeliveryStatus::Disputed).is_err());

    // Active cannot go backward or skip
    assert!(validate_transition(DeliveryStatus::Active, DeliveryStatus::Pending).is_err());
    assert!(validate_transition(DeliveryStatus::Active, DeliveryStatus::Delivered).is_err());

    // InTransit cannot go backward or cancel
    assert!(validate_transition(DeliveryStatus::InTransit, DeliveryStatus::Pending).is_err());
    assert!(validate_transition(DeliveryStatus::InTransit, DeliveryStatus::Active).is_err());
    assert!(validate_transition(DeliveryStatus::InTransit, DeliveryStatus::Cancelled).is_err());

    // Disputed cannot transition back to transit states
    assert!(validate_transition(DeliveryStatus::Disputed, DeliveryStatus::Pending).is_err());
    assert!(validate_transition(DeliveryStatus::Disputed, DeliveryStatus::Active).is_err());
    assert!(validate_transition(DeliveryStatus::Disputed, DeliveryStatus::InTransit).is_err());

    // Terminal states — no transitions allowed
    assert!(validate_transition(DeliveryStatus::Delivered, DeliveryStatus::Active).is_err());
    assert!(validate_transition(DeliveryStatus::Delivered, DeliveryStatus::Disputed).is_err());
    assert!(validate_transition(DeliveryStatus::Delivered, DeliveryStatus::Cancelled).is_err());
    assert!(validate_transition(DeliveryStatus::Delivered, DeliveryStatus::InTransit).is_err());
    assert!(validate_transition(DeliveryStatus::Cancelled, DeliveryStatus::Active).is_err());
    assert!(validate_transition(DeliveryStatus::Cancelled, DeliveryStatus::Delivered).is_err());
    assert!(validate_transition(DeliveryStatus::Cancelled, DeliveryStatus::InTransit).is_err());
    assert!(validate_transition(DeliveryStatus::Cancelled, DeliveryStatus::Disputed).is_err());
}

#[test]
fn test_validate_transition_returns_invalid_state_error() {
    let result = validate_transition(DeliveryStatus::Delivered, DeliveryStatus::Active);
    assert_eq!(result, Err(DeliveryError::InvalidState));
}

// ── Issue #22: mark_in_transit tests ─────────────────────────────────────────

#[test]
fn test_mark_in_transit_success() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.mark_in_transit(&driver, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    assert_eq!(delivery.status, DeliveryStatus::InTransit);
    assert!(delivery.transit_started_at.is_some());
}

#[test]
fn test_mark_in_transit_records_timestamp() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    let ts_before = env.ledger().timestamp();
    client.mark_in_transit(&driver, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });

    let recorded_ts = delivery.transit_started_at.unwrap();
    assert!(recorded_ts >= ts_before);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_mark_in_transit_wrong_driver_rejected() {
    let (env, client, admin, driver, unauthorized) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.mark_in_transit(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_mark_in_transit_unassigned_driver_rejected() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    let random_driver = Address::generate(&env);
    // No driver assigned — driver field is None → NotAuthorized
    client.mark_in_transit(&random_driver, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_mark_in_transit_from_pending_rejected() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    let driver = Address::generate(&env);
    // Assign driver manually so the driver field matches but status is still Pending
    env.as_contract(&client.address, || {
        let key = DataKey::Delivery(delivery_id);
        let mut d: DeliveryRecord = env.storage().persistent().get(&key).unwrap();
        d.driver = Some(driver.clone());
        env.storage().persistent().set(&key, &d);
    });

    client.mark_in_transit(&driver, &delivery_id);
}

#[test]
fn test_mark_in_transit_emits_event() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.mark_in_transit(&driver, &delivery_id);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "DeliveryInTransit"));
}

// ── Issue #27: raise_dispute tests ───────────────────────────────────────────

#[test]
fn test_raise_dispute_from_active_by_sender() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.raise_dispute(&sender, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });
    assert_eq!(delivery.status, DeliveryStatus::Disputed);
}

#[test]
fn test_raise_dispute_from_in_transit() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);

    client.raise_dispute(&sender, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });
    assert_eq!(delivery.status, DeliveryStatus::Disputed);
}

#[test]
fn test_raise_dispute_by_recipient() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.raise_dispute(&recipient, &delivery_id);

    let delivery: DeliveryRecord = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Delivery(delivery_id))
            .unwrap()
    });
    assert_eq!(delivery.status, DeliveryStatus::Disputed);
}

#[test]
#[should_panic(expected = "NotAuthorized")]
fn test_raise_dispute_non_participant_rejected() {
    let (env, client, admin, driver, unauthorized) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.raise_dispute(&unauthorized, &delivery_id);
}

#[test]
#[should_panic(expected = "InvalidState")]
fn test_raise_dispute_from_pending_rejected() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    // Delivery is still Pending — invalid transition
    client.raise_dispute(&sender, &delivery_id);
}

#[test]
#[should_panic(expected = "Escrow raise_dispute failure simulated")]
fn test_raise_dispute_escrow_failure_reverts_delivery_state() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 777);
    
    env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &776u64);
    });

    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    assert_eq!(delivery_id, 777);
    client.assign_driver(&admin, &delivery_id, &driver);

    // Escrow mock panics for delivery_id 777 — delivery state is never mutated
    client.raise_dispute(&sender, &delivery_id);
}

#[test]
fn test_raise_dispute_emits_event() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);

    client.raise_dispute(&sender, &delivery_id);

    let events = env.events().all();
    let last_event = events.last().unwrap();
    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "delivery_disputed"));
}

// ── Driver Profile Reputation Tests ─────────────────────────────────────────

#[test]
fn test_driver_profile_starts_empty() {
    let (env, client, _, driver, _) = setup_test();

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.address, driver);
    assert_eq!(profile.deliveries_completed, 0);
    assert_eq!(profile.reputation_score, 0);
}

#[test]
fn test_driver_profile_increments_on_delivery() {
    let (env, client, admin, driver, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    // First delivery
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);
    client.assign_driver(&admin, &delivery_id, &driver);
    client.mark_in_transit(&driver, &delivery_id);
    client.confirm_delivery(&recipient, &delivery_id);

    let profile1 = client.get_driver_profile(&driver);
    assert_eq!(profile1.deliveries_completed, 1);
    assert_eq!(profile1.reputation_score, 1);

    // Second delivery
    let metadata2 = get_test_metadata(&env, 2);
    let delivery_id2 = client.create_delivery(&sender, &recipient, &metadata2);
    client.assign_driver(&admin, &delivery_id2, &driver);
    client.mark_in_transit(&driver, &delivery_id2);
    client.confirm_delivery(&recipient, &delivery_id2);

    let profile2 = client.get_driver_profile(&driver);
    assert_eq!(profile2.deliveries_completed, 2);
    assert_eq!(profile2.reputation_score, 2);
}

// ── Get Delivery Query Tests ────────────────────────────────────────────────

#[test]
fn test_get_delivery_success() {
    let (env, client, _, _, _) = setup_test();
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let metadata = get_test_metadata(&env, 1);
    let delivery_id = client.create_delivery(&sender, &recipient, &metadata);

    let delivery = client.get_delivery(&delivery_id);

    assert_eq!(delivery.delivery_id, delivery_id);
    assert_eq!(delivery.sender, sender);
    assert_eq!(delivery.status, DeliveryStatus::Pending);
    assert_eq!(delivery.recipient, recipient);
}

#[test]
#[should_panic(expected = "DeliveryNotFound")]
fn test_get_delivery_not_found() {
    let (_env, client, _, _, _) = setup_test();
    client.get_delivery(&999);
}

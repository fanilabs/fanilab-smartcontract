extern crate std;

use super::*;
use shared_types::{DeliveryId, DeliveryRecord, DeliveryStatus, FaniLabError};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    xdr, Address, Env, String, Symbol, TryFromVal, Val,
};

fn did(value: u64) -> DeliveryId {
    DeliveryId::from(value)
}

/// Decode the most recently published event into a (topics, data) pair. SDK 27's
/// `ContractEvents` only exposes the raw XDR form, so it has to be converted back
/// to host values before assertions can be made against it.
fn last_event(env: &Env) -> (soroban_sdk::Vec<Val>, Val) {
    let events = env.events().all();
    let raw = events.events().last().expect("no events emitted").clone();
    let xdr::ContractEventBody::V0(body) = raw.body;
    let mut topics = soroban_sdk::Vec::new(env);
    for topic in body.topics.iter() {
        topics.push_back(Val::try_from_val(env, topic).expect("failed to decode topic"));
    }
    let data = Val::try_from_val(env, &body.data).expect("failed to decode event data");
    (topics, data)
}

#[contract]
pub struct MockDeliveryContract;

#[contractimpl]
impl MockDeliveryContract {
    pub fn get_delivery(env: Env, delivery_id: DeliveryId) -> DeliveryRecord {
        env.storage()
            .instance()
            .get::<_, DeliveryRecord>(&u64::from(delivery_id))
            .unwrap_or_else(|| panic!("DeliveryNotFound"))
    }

    pub fn raise_dispute(env: Env, _caller: Address, delivery_id: DeliveryId) {
        let storage_key = u64::from(delivery_id);
        if env.storage().instance().has(&storage_key) {
            let mut record: DeliveryRecord = env.storage().instance().get(&storage_key).unwrap();
            record.status = shared_types::DeliveryStatus::Disputed;
            env.storage().instance().set(&storage_key, &record);
        }
    }
}

#[contract]
pub struct MockEscrowContract;

#[contractimpl]
impl MockEscrowContract {
    pub fn get_escrow(env: Env, delivery_id: u64) -> shared_types::EscrowRecord {
        env.storage()
            .instance()
            .get(&delivery_id)
            .unwrap_or_else(|| panic!("EscrowNotFound"))
    }

    pub fn resolve_dispute(env: Env, _caller: Address, delivery_id: u64, release_to_driver: bool) {
        if env.storage().instance().has(&delivery_id) {
            let mut record: shared_types::EscrowRecord =
                env.storage().instance().get(&delivery_id).unwrap();
            if release_to_driver {
                record.status = shared_types::EscrowStatus::Released;
            } else {
                record.status = shared_types::EscrowStatus::Refunded;
            }
            env.storage().instance().set(&delivery_id, &record);
        }
    }

    pub fn resolve_dispute_split(
        env: Env,
        _caller: Address,
        delivery_id: u64,
        _sender_share_bps: u32,
    ) {
        if env.storage().instance().has(&delivery_id) {
            let mut record: shared_types::EscrowRecord =
                env.storage().instance().get(&delivery_id).unwrap();
            record.status = shared_types::EscrowStatus::Refunded;
            env.storage().instance().set(&delivery_id, &record);
        }
    }

    pub fn freeze_funds(env: Env, _caller: Address, delivery_id: u64) {
        if env.storage().instance().has(&delivery_id) {
            let mut record: shared_types::EscrowRecord =
                env.storage().instance().get(&delivery_id).unwrap();
            record.status = shared_types::EscrowStatus::Paused;
            env.storage().instance().set(&delivery_id, &record);
        }
    }
}

fn setup_test() -> (
    Env,
    Address, // admin
    Address, // sender
    Address, // recipient
    Address, // driver
    Address, // delivery contract ID
    Address, // escrow contract ID
    DisputeResolutionContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let delivery_id = env.register(MockDeliveryContract, ());
    let escrow_id = env.register(MockEscrowContract, ());
    let dispute_id = env.register(DisputeResolutionContract, ());

    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    // Time limit: 1 day (86400 seconds) — MIN_DISPUTE_TIME_LIMIT
    dispute_client.init(&admin, &delivery_id, &escrow_id, &86400, &604800);

    (
        env,
        admin,
        sender,
        recipient,
        driver,
        delivery_id,
        escrow_id,
        dispute_client,
    )
}

fn set_mock_delivery(
    env: &Env,
    delivery_contract_id: &Address,
    delivery_id: DeliveryId,
    record: &DeliveryRecord,
) {
    env.as_contract(delivery_contract_id, || {
        env.storage()
            .instance()
            .set(&u64::from(delivery_id), record);
    });
}

fn set_mock_escrow(
    env: &Env,
    escrow_contract_id: &Address,
    delivery_id: u64,
    record: &shared_types::EscrowRecord,
) {
    env.as_contract(escrow_contract_id, || {
        env.storage().instance().set(&delivery_id, record);
    });
}

fn create_mock_delivery_record(
    env: &Env,
    delivery_id: DeliveryId,
    sender: Address,
    recipient: Address,
    status: DeliveryStatus,
    delivered_at: Option<u64>,
) -> DeliveryRecord {
    let cargo = shared_types::CargoDescriptor {
        weight_grams: 500,
        category: shared_types::CargoCategory::Electronics,
        fragile: true,
    };
    let metadata = shared_types::DeliveryMetadata {
        delivery_id: u64::from(delivery_id),
        origin: String::from_str(env, "Origin"),
        destination: String::from_str(env, "Destination"),
        cargo_description: cargo,
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 3600,
    };
    DeliveryRecord {
        delivery_id,
        sender,
        recipient,
        driver: None,
        status,
        metadata,
        created_at: env.ledger().timestamp(),
        delivered_at,
        transit_started_at: None,
    }
}

fn create_mock_escrow_record(
    sender: Address,
    recipient: Address,
    driver: Address,
    token: Address,
    status: shared_types::EscrowStatus,
) -> shared_types::EscrowRecord {
    shared_types::EscrowRecord {
        delivery_id: 0,
        sender,
        recipient,
        driver,
        token,
        amount: 500,
        status,
        created_at: 0,
        expires_at: None,
        disputed_by: None,
        disputed_at: None,
        holdback_started_at: None,
        fleet_id: None,
    }
}

#[test]
fn test_init_and_setup() {
    let (_env, admin, _, _, _, delivery_id, escrow_id, dispute_client) = setup_test();

    assert_eq!(dispute_client.get_delivery_contract(), delivery_id);
    assert_eq!(dispute_client.get_escrow_contract(), escrow_id);
    assert_eq!(dispute_client.get_dispute_time_limit(), 86400);
    assert!(dispute_client.is_admin(&admin));
}

#[test]
fn test_admin_whitelist_management() {
    let (env, admin, _, _, _, _, _, dispute_client) = setup_test();

    let new_admin = Address::generate(&env);
    assert!(!dispute_client.is_admin(&new_admin));

    // Admin adds new_admin
    dispute_client.add_admin(&admin, &new_admin);
    assert!(dispute_client.is_admin(&new_admin));

    // Original admin steps down, leaving new_admin as the sole admin. A
    // self-removal is the sanctioned way to reduce the roster to one admin
    // (Issue #212); an admin removing a *different* admin may never leave
    // itself alone.
    dispute_client.remove_admin(&admin, &admin);
    assert!(!dispute_client.is_admin(&admin));
    assert!(dispute_client.is_admin(&new_admin));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // FaniLabError::InvalidState
fn test_admin_cannot_consolidate_roster_to_self() {
    // Issue #212: an admin must not be able to reduce the roster to only
    // itself by removing the others.
    let (env, admin, _, _, _, _, _, dispute_client) = setup_test();

    let admin2 = Address::generate(&env);
    dispute_client.add_admin(&admin, &admin2);

    // `admin` removing `admin2` would leave `admin` as the sole admin — the
    // self-service consolidation this guard blocks.
    dispute_client.remove_admin(&admin, &admin2);
}

#[test]
fn test_admin_removal_still_works_while_another_admin_remains() {
    // Issue #212 regression: legitimate removals through the intended process
    // still succeed as long as at least one other admin remains.
    let (env, admin, _, _, _, _, _, dispute_client) = setup_test();

    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    dispute_client.add_admin(&admin, &admin2);
    dispute_client.add_admin(&admin, &admin3);

    // Roster is [admin, admin2, admin3]; removing admin3 leaves two admins.
    dispute_client.remove_admin(&admin, &admin3);
    assert!(!dispute_client.is_admin(&admin3));
    assert_eq!(dispute_client.list_admins().len(), 2);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // FaniLabError::InvalidState
fn test_remove_last_admin_rejected() {
    let (_env, admin, _, _, _, _, _, dispute_client) = setup_test();

    // `admin` is the only admin left after init — removing it must be
    // rejected, since it would permanently brick governance (no one left
    // who could call add_admin to recover).
    dispute_client.remove_admin(&admin, &admin);
}

#[test]
fn test_remove_admin_allowed_when_multiple_admins_remain() {
    let (env, admin, _, _, _, _, _, dispute_client) = setup_test();

    let second_admin = Address::generate(&env);
    dispute_client.add_admin(&admin, &second_admin);

    // With two admins present, removing one must still succeed.
    dispute_client.remove_admin(&admin, &admin);
    assert!(!dispute_client.is_admin(&admin));
    assert!(dispute_client.is_admin(&second_admin));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_unauthorized_add_admin_fails() {
    let (env, _, sender, _, _, _, _, dispute_client) = setup_test();
    let attacker = sender;
    let target = Address::generate(&env);

    dispute_client.add_admin(&attacker, &target);
}

#[test]
fn test_raise_dispute_active_delivery() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery status: Active
    let delivery_record = create_mock_delivery_record(
        &env,
        did(1),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(1), &delivery_record);

    // Setup mock escrow status: Locked
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 1, &escrow_record);

    // Raise dispute
    dispute_client.raise_dispute(&sender, &did(1));

    // Verify delivery status changed to Disputed in MockDeliveryContract
    let delivery = MockDeliveryContractClient::new(&env, &delivery_id).get_delivery(&did(1));
    assert_eq!(delivery.status, DeliveryStatus::Disputed);

    // Verify escrow status changed to Paused in MockEscrowContract
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&1);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Paused);

    // Verify local dispute case in DisputeResolutionContract
    let case = dispute_client.get_dispute(&did(1));
    assert_eq!(case.delivery_id, did(1));
    assert_eq!(case.status, DisputeStatus::Open);
    assert_eq!(case.raised_by, sender);
    assert_eq!(case.evidence_hashes.len(), 0);
}

#[test]
fn test_raise_dispute_delivered_within_time_limit() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery status: Delivered with timestamp
    let delivered_at = env.ledger().timestamp();
    let delivery_record = create_mock_delivery_record(
        &env,
        did(2),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Delivered,
        Some(delivered_at),
    );
    set_mock_delivery(&env, &delivery_id, did(2), &delivery_record);

    // Setup mock escrow status: Released
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Released,
    );
    set_mock_escrow(&env, &escrow_id, 2, &escrow_record);

    // Set time forward by 1800 seconds (30 mins)
    env.ledger().set_timestamp(delivered_at + 1800);

    // Raise dispute
    dispute_client.raise_dispute(&recipient, &did(2));

    // Verify local dispute case is created
    let case = dispute_client.get_dispute(&did(2));
    assert_eq!(case.status, DisputeStatus::Open);
    assert_eq!(case.raised_by, recipient);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // FaniLabError::InvalidState
fn test_raise_dispute_delivered_exceeds_time_limit() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery status: Delivered
    let delivered_at = env.ledger().timestamp();
    let delivery_record = create_mock_delivery_record(
        &env,
        did(3),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Delivered,
        Some(delivered_at),
    );
    set_mock_delivery(&env, &delivery_id, did(3), &delivery_record);

    // Setup mock escrow status: Released
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Released,
    );
    set_mock_escrow(&env, &escrow_id, 3, &escrow_record);

    // Set time forward past the 86400s (MIN_DISPUTE_TIME_LIMIT) configured in setup_test
    env.ledger().set_timestamp(delivered_at + 86401);

    // Attempt to raise dispute (should fail due to time limit exceeded)
    dispute_client.raise_dispute(&recipient, &did(3));
}

#[test]
fn test_update_dispute_time_limit_allows_below_minimum_and_getter_returns_value() {
    let (_env, admin, _, _, _, _, _, dispute_client) = setup_test();

    dispute_client.update_dispute_time_limit(&admin, &1000);

    assert_eq!(dispute_client.get_dispute_time_limit(), 1000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // FaniLabError::InvalidState
fn test_updated_dispute_time_limit_shortens_delivered_dispute_window() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();
    dispute_client.update_dispute_time_limit(&admin, &1000);

    let delivered_at = env.ledger().timestamp();
    let delivery_record = create_mock_delivery_record(
        &env,
        did(11),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Delivered,
        Some(delivered_at),
    );
    set_mock_delivery(&env, &delivery_id, did(11), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender,
        recipient.clone(),
        driver,
        token,
        shared_types::EscrowStatus::Released,
    );
    set_mock_escrow(&env, &escrow_id, 11, &escrow_record);

    env.ledger().set_timestamp(delivered_at + 1001);
    dispute_client.raise_dispute(&recipient, &did(11));
}

#[test]
fn test_set_dispute_resolution_limit_getter_and_force_resolution_window() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();
    dispute_client.set_dispute_resolution_limit(&admin, &1000);
    assert_eq!(dispute_client.get_dispute_resolution_limit(), 1000);

    let delivery_record = create_mock_delivery_record(
        &env,
        did(12),
        sender.clone(),
        recipient,
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(12), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        delivery_record.sender.clone(),
        driver,
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 12, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(12));
    env.ledger().set_timestamp(1001);
    dispute_client.force_resolve_dispute(&sender, &did(12));

    assert_eq!(
        dispute_client.get_dispute(&did(12)).status,
        DisputeStatus::Split
    );
}

#[test]
fn test_resolve_dispute_refund_sender_by_admin() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery with driver assigned (required for reputation penalty on resolve)
    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(4),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(4), &delivery_record);

    // Setup mock escrow as Paused (representing escrow paused after dispute raised)
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 4, &escrow_record);

    // Raise dispute to initialize local dispute case
    dispute_client.raise_dispute(&sender, &did(4));

    // Resolve dispute
    dispute_client.resolve_dispute_refund_sender(&admin, &did(4));

    // Verify local dispute status is ResolvedRefund
    let case = dispute_client.get_dispute(&did(4));
    assert_eq!(case.status, DisputeStatus::ResolvedRefund);

    // Verify mock escrow status updated to Refunded
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&4);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Refunded);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_unauthorized_resolve_dispute_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(5),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(5), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 5, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(5));

    // Attacker (sender) tries to resolve dispute
    dispute_client.resolve_dispute_refund_sender(&sender, &did(5));
}

#[test]
fn test_add_evidence_hash_success() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(6),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(6), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(6));

    let evidence_hash1 = soroban_sdk::BytesN::from_array(&env, &[1; 32]);
    let evidence_hash2 = soroban_sdk::BytesN::from_array(&env, &[2; 32]);

    // Sender adds evidence
    dispute_client.add_evidence_hash(&sender, &did(6), &evidence_hash1);
    // Recipient adds evidence
    dispute_client.add_evidence_hash(&recipient, &did(6), &evidence_hash2);

    let case = dispute_client.get_dispute(&did(6));
    assert_eq!(case.evidence_hashes.len(), 2);
    assert_eq!(case.evidence_hashes.get(0).unwrap().hash, evidence_hash1);
    assert_eq!(case.evidence_hashes.get(0).unwrap().submitter, sender);
    assert_eq!(case.evidence_hashes.get(1).unwrap().hash, evidence_hash2);
    assert_eq!(case.evidence_hashes.get(1).unwrap().submitter, recipient);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_add_evidence_unauthorized_fails() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(7),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(7), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(7));

    let attacker = Address::generate(&env);
    let evidence_hash = soroban_sdk::BytesN::from_array(&env, &[3; 32]);

    dispute_client.add_evidence_hash(&attacker, &did(7), &evidence_hash);
}

#[test]
fn test_add_evidence_hash_up_to_cap_succeeds() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(8),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(8), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(8));

    for i in 0..MAX_EVIDENCE_HASHES_PER_PARTY {
        let hash = soroban_sdk::BytesN::from_array(&env, &[i as u8; 32]);
        dispute_client.add_evidence_hash(&sender, &did(8), &hash);
    }

    let case = dispute_client.get_dispute(&did(8));
    assert_eq!(case.evidence_hashes.len(), MAX_EVIDENCE_HASHES_PER_PARTY);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")] // FaniLabError::LimitExceeded
fn test_add_evidence_hash_beyond_cap_rejected() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(9),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(9), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(9));

    for i in 0..MAX_EVIDENCE_HASHES_PER_PARTY {
        let hash = soroban_sdk::BytesN::from_array(&env, &[i as u8; 32]);
        dispute_client.add_evidence_hash(&sender, &did(9), &hash);
    }

    // One past the per-party cap must be rejected.
    let one_too_many = soroban_sdk::BytesN::from_array(&env, &[0xFF; 32]);
    dispute_client.add_evidence_hash(&sender, &did(9), &one_too_many);
}

// ── PER-PARTY EVIDENCE QUOTA (Issue #209) ────────────────────────────────────

/// Regression: one party exhausting its evidence quota must not stop another
/// party from submitting. Before the per-party quota, a shared cap let the
/// first party lock the counterparty out for the life of the dispute.
#[test]
fn test_evidence_quota_is_per_party_no_lockout() {
    let (env, _admin, sender, recipient, driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(20),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(20), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(20));

    // Sender fills its entire quota.
    for i in 0..MAX_EVIDENCE_HASHES_PER_PARTY {
        let hash = soroban_sdk::BytesN::from_array(&env, &[i as u8; 32]);
        dispute_client.add_evidence_hash(&sender, &did(20), &hash);
    }

    // Recipient and driver can still submit their own evidence.
    let r_hash = soroban_sdk::BytesN::from_array(&env, &[0xAA; 32]);
    let d_hash = soroban_sdk::BytesN::from_array(&env, &[0xBB; 32]);
    dispute_client.add_evidence_hash(&recipient, &did(20), &r_hash);
    dispute_client.add_evidence_hash(&driver, &did(20), &d_hash);

    let case = dispute_client.get_dispute(&did(20));
    assert_eq!(
        case.evidence_hashes.len(),
        MAX_EVIDENCE_HASHES_PER_PARTY + 2
    );
}

/// A party cannot submit the same hash twice; a different party still can.
#[test]
fn test_evidence_duplicate_from_same_party_rejected() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(21),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(21), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(21));

    let hash = soroban_sdk::BytesN::from_array(&env, &[7u8; 32]);
    dispute_client.add_evidence_hash(&sender, &did(21), &hash);

    // Same party, same hash → rejected.
    let res = dispute_client.try_add_evidence_hash(&sender, &did(21), &hash);
    assert!(res.is_err());

    // Different party referencing the same document is still allowed.
    dispute_client.add_evidence_hash(&recipient, &did(21), &hash);

    let case = dispute_client.get_dispute(&did(21));
    assert_eq!(case.evidence_hashes.len(), 2);
}

/// Quota exhaustion returns `LimitExceeded` only for the party that is full;
/// other parties are unaffected.
#[test]
fn test_evidence_quota_exhaustion_is_scoped_to_offending_party() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(22),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(22), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(22));

    for i in 0..MAX_EVIDENCE_HASHES_PER_PARTY {
        let hash = soroban_sdk::BytesN::from_array(&env, &[i as u8; 32]);
        dispute_client.add_evidence_hash(&sender, &did(22), &hash);
    }

    // Sender is full — its next submission fails with LimitExceeded.
    let extra = soroban_sdk::BytesN::from_array(&env, &[0xF0; 32]);
    match dispute_client.try_add_evidence_hash(&sender, &did(22), &extra) {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::LimitExceeded.into()),
        other => panic!("expected LimitExceeded for the full party, got {other:?}"),
    }

    // Recipient is not affected by the sender's exhausted quota.
    dispute_client.add_evidence_hash(&recipient, &did(22), &extra);
}

/// Total per-dispute evidence storage stays bounded: with three authorized
/// parties the hard ceiling is `3 * MAX_EVIDENCE_HASHES_PER_PARTY`, and any
/// submission past that is rejected regardless of caller.
#[test]
fn test_evidence_total_storage_bounded_across_all_parties() {
    let (env, _admin, sender, recipient, driver, delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(23),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(23), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(23));

    for (party_idx, party) in [sender.clone(), recipient.clone(), driver.clone()]
        .iter()
        .enumerate()
    {
        for i in 0..MAX_EVIDENCE_HASHES_PER_PARTY {
            let hash =
                soroban_sdk::BytesN::from_array(&env, &[(party_idx as u8) * 100 + i as u8; 32]);
            dispute_client.add_evidence_hash(party, &did(23), &hash);
        }
    }

    let case = dispute_client.get_dispute(&did(23));
    assert_eq!(
        case.evidence_hashes.len(),
        3 * MAX_EVIDENCE_HASHES_PER_PARTY
    );

    // Every party is now at its quota — no one can add more.
    let extra = soroban_sdk::BytesN::from_array(&env, &[0xEE; 32]);
    assert!(dispute_client
        .try_add_evidence_hash(&recipient, &did(23), &extra)
        .is_err());
}

/// Existing behavior: evidence cannot be added once a dispute leaves `Open`.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // FaniLabError::InvalidState
fn test_evidence_cannot_be_added_to_resolved_dispute() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(24),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(24), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 24, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(24));
    dispute_client.resolve_dispute_refund_sender(&admin, &did(24));

    let hash = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    dispute_client.add_evidence_hash(&sender, &did(24), &hash);
}

#[test]
fn test_integration_resolve_dispute_split_funds() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    // Register real contracts
    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(escrow_contract::EscrowContract, ());
    let dispute_resolution_id = env.register(DisputeResolutionContract, ());

    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = escrow_contract::EscrowContractClient::new(&env, &escrow_contract_id);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_resolution_id);

    // Register stellar asset contract for token
    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

    // Init contracts
    escrow_client.init(&admin, &token, &0);
    escrow_client.set_dispute_resolution_contract(&admin, &dispute_resolution_id);
    delivery_client.init(&admin, &escrow_contract_id);
    dispute_client.init(
        &admin,
        &delivery_contract_id,
        &escrow_contract_id,
        &86400,
        &604800,
    );

    // Mint tokens to sender
    StellarAssetClient::new(&env, &token).mint(&sender, &1000);

    // Create delivery
    let metadata = {
        let cargo = shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: true,
        };
        shared_types::DeliveryMetadata {
            delivery_id: 0,
            origin: String::from_str(&env, "Origin"),
            destination: String::from_str(&env, "Destination"),
            cargo_description: cargo,
            created_at: env.ledger().timestamp(),
            estimated_delivery: env.ledger().timestamp() + 3600,
        }
    };
    let delivery_id_val = delivery_client.create_delivery(&sender, &recipient, &metadata);

    // Create escrow
    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id_val),
        &token,
        &1000,
        &None,
    );

    // Assign driver to make delivery Active
    delivery_client.assign_driver(&admin, &delivery_id_val, &driver);

    // Raise dispute
    dispute_client.raise_dispute(&sender, &delivery_id_val);

    // Verify escrow is paused
    let escrow = escrow_client.get_escrow(&u64::from(delivery_id_val));
    assert_eq!(escrow.status, shared_types::EscrowStatus::Paused);

    // Resolve split (60% sender, 40% driver)
    dispute_client.resolve_dispute_split_funds(&admin, &delivery_id_val, &6000);

    // Verify local dispute is Split
    let case = dispute_client.get_dispute(&delivery_id_val);
    assert_eq!(case.status, DisputeStatus::Split);

    // Verify token balances
    let sender_balance = TokenClient::new(&env, &token).balance(&sender);
    let driver_balance = TokenClient::new(&env, &token).balance(&driver);
    assert_eq!(sender_balance, 600); // 60% of 1000 refunded
    assert_eq!(driver_balance, 400); // 40% of 1000 paid to driver
}

/// Issue #51 regression test: `resolve_dispute_refund_sender` is the one path
/// in the protocol that cross-calls `identity_reputation_contract::
/// decrease_reputation`, but until now nothing exercised it end-to-end
/// through real contracts — a full delivery -> escrow -> dispute_resolution
/// -> identity_reputation chain. This wires all four real contracts together
/// and asserts the driver's on-chain reputation score actually drops.
#[test]
fn test_integration_resolve_dispute_refund_sender_decreases_reputation() {
    use identity_reputation_contract::{
        IdentityReputationContract, IdentityReputationContractClient,
    };

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    // Register real contracts.
    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(escrow_contract::EscrowContract, ());
    let dispute_resolution_id = env.register(DisputeResolutionContract, ());
    let identity_contract_id = env.register(IdentityReputationContract, ());

    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = escrow_contract::EscrowContractClient::new(&env, &escrow_contract_id);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_resolution_id);
    let identity_client = IdentityReputationContractClient::new(&env, &identity_contract_id);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

    escrow_client.init(&admin, &token, &0);
    escrow_client.set_dispute_resolution_contract(&admin, &dispute_resolution_id);
    delivery_client.init(&admin, &escrow_contract_id);
    dispute_client.init(
        &admin,
        &delivery_contract_id,
        &escrow_contract_id,
        &86400,
        &604800,
    );
    // Authorizes both delivery_contract and dispute_resolution_contract to
    // call increase_reputation/decrease_reputation.
    identity_client.init(&admin, &delivery_contract_id, &dispute_resolution_id);
    dispute_client.set_identity_reputation_contract(&admin, &identity_contract_id);

    identity_client.register_driver(&driver);
    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        50
    );

    StellarAssetClient::new(&env, &token).mint(&sender, &1000);

    let metadata = {
        let cargo = shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: false,
        };
        shared_types::DeliveryMetadata {
            delivery_id: 0,
            origin: String::from_str(&env, "Origin"),
            destination: String::from_str(&env, "Destination"),
            cargo_description: cargo,
            created_at: env.ledger().timestamp(),
            estimated_delivery: env.ledger().timestamp() + 3600,
        }
    };
    let delivery_id_val = delivery_client.create_delivery(&sender, &recipient, &metadata);

    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id_val),
        &token,
        &1000,
        &None,
    );

    delivery_client.assign_driver(&admin, &delivery_id_val, &driver);
    dispute_client.raise_dispute(&sender, &delivery_id_val);

    dispute_client.resolve_dispute_refund_sender(&admin, &delivery_id_val);

    let case = dispute_client.get_dispute(&delivery_id_val);
    assert_eq!(case.status, DisputeStatus::ResolvedRefund);

    let penalty = dispute_client.get_dispute_reputation_penalty();
    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        50 - penalty
    );

    // The sender got their funds back; the driver's reputation is the only
    // thing that moved.
    assert_eq!(TokenClient::new(&env, &token).balance(&sender), 1000);
}

#[test]
fn test_resolve_dispute_pay_driver_by_admin() {
    use identity_reputation_contract::{
        IdentityReputationContract, IdentityReputationContractClient,
    };

    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Wire a real identity-reputation contract so the reputation-award branch
    // of resolve_dispute_pay_driver is actually executed (previously this test
    // ran with no reputation contract configured, so the malformed cross-call
    // was never reached — see issue #207).
    let identity_id = env.register(IdentityReputationContract, ());
    let identity_client = IdentityReputationContractClient::new(&env, &identity_id);
    identity_client.init(&admin, &delivery_id, &dispute_client.address);
    dispute_client.set_identity_reputation_contract(&admin, &identity_id);
    identity_client.register_driver(&driver);
    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        50
    );

    // Setup mock delivery with the driver assigned.
    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(8),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(8), &delivery_record);

    // Setup mock escrow as Paused (representing escrow paused after dispute raised)
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 8, &escrow_record);

    // Raise dispute to initialize local dispute case
    dispute_client.raise_dispute(&sender, &did(8));

    // Resolve dispute
    dispute_client.resolve_dispute_pay_driver(&admin, &did(8));

    // Verify local dispute status is ResolvedPayout
    let case = dispute_client.get_dispute(&did(8));
    assert_eq!(case.status, DisputeStatus::ResolvedPayout);

    // Verify mock escrow status updated to Released
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&8);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Released);

    // The driver's reputation went up by the flat dispute reward, and the
    // award did NOT count as a delivery completion.
    let profile = identity_client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);
    assert_eq!(profile.deliveries_completed, 0);
}

/// Issue #207 regression: `resolve_dispute_pay_driver` cross-calls the identity
/// reputation contract with a malformed arity (three args where five are
/// expected), so on any deployment with reputation wired the whole transaction
/// reverted and a dispute could never be resolved in the driver's favour. This
/// wires all four real contracts and asserts the ruling succeeds end to end and
/// the driver's on-chain reputation actually rises by the flat dispute reward
/// without counting as a delivery completion.
#[test]
fn test_integration_resolve_dispute_pay_driver_increases_reputation() {
    use identity_reputation_contract::{
        IdentityReputationContract, IdentityReputationContractClient,
    };

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(escrow_contract::EscrowContract, ());
    let dispute_resolution_id = env.register(DisputeResolutionContract, ());
    let identity_contract_id = env.register(IdentityReputationContract, ());

    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = escrow_contract::EscrowContractClient::new(&env, &escrow_contract_id);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_resolution_id);
    let identity_client = IdentityReputationContractClient::new(&env, &identity_contract_id);

    let token = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();

    escrow_client.init(&admin, &token, &0);
    escrow_client.set_dispute_resolution_contract(&admin, &dispute_resolution_id);
    delivery_client.init(&admin, &escrow_contract_id);
    dispute_client.init(
        &admin,
        &delivery_contract_id,
        &escrow_contract_id,
        &86400,
        &604800,
    );
    identity_client.init(&admin, &delivery_contract_id, &dispute_resolution_id);
    dispute_client.set_identity_reputation_contract(&admin, &identity_contract_id);

    identity_client.register_driver(&driver);
    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        50
    );

    StellarAssetClient::new(&env, &token).mint(&sender, &1000);

    let metadata = {
        let cargo = shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: false,
        };
        shared_types::DeliveryMetadata {
            delivery_id: 0,
            origin: String::from_str(&env, "Origin"),
            destination: String::from_str(&env, "Destination"),
            cargo_description: cargo,
            created_at: env.ledger().timestamp(),
            estimated_delivery: env.ledger().timestamp() + 3600,
        }
    };
    let delivery_id_val = delivery_client.create_delivery(&sender, &recipient, &metadata);

    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id_val),
        &token,
        &1000,
        &None,
    );

    delivery_client.assign_driver(&admin, &delivery_id_val, &driver);
    dispute_client.raise_dispute(&sender, &delivery_id_val);

    // The ruling must succeed — before the fix this call reverted.
    dispute_client.resolve_dispute_pay_driver(&admin, &delivery_id_val);

    let case = dispute_client.get_dispute(&delivery_id_val);
    assert_eq!(case.status, DisputeStatus::ResolvedPayout);

    // Driver reputation rose by the flat reward; it is not a delivery completion.
    let profile = identity_client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);
    assert_eq!(profile.deliveries_completed, 0);

    // Funds were released to the driver.
    assert_eq!(TokenClient::new(&env, &token).balance(&driver), 1000);
    assert_eq!(TokenClient::new(&env, &token).balance(&sender), 0);
}

/// Issue #207: a flat dispute award must never push a driver past the
/// reputation ceiling.
#[test]
fn test_dispute_reward_respects_max_reputation() {
    use identity_reputation_contract::{
        IdentityReputationContract, IdentityReputationContractClient,
    };

    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let identity_id = env.register(IdentityReputationContract, ());
    let identity_client = IdentityReputationContractClient::new(&env, &identity_id);
    identity_client.init(&admin, &delivery_id, &dispute_client.address);
    dispute_client.set_identity_reputation_contract(&admin, &identity_id);
    identity_client.register_driver(&driver);

    // Push the driver to the ceiling first via authorized flat awards.
    for _ in 0..20 {
        identity_client.award_reputation(&dispute_client.address, &driver, &10u32);
    }
    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        100
    );

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(25),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(25), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 25, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(25));
    dispute_client.resolve_dispute_pay_driver(&admin, &did(25));

    assert_eq!(
        identity_client.get_driver_profile(&driver).reputation_score,
        100
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_unauthorized_resolve_pay_driver_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(9),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(9), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 9, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(9));

    // Attacker (sender) tries to resolve dispute pay driver
    dispute_client.resolve_dispute_pay_driver(&sender, &did(9));
}

#[test]
fn test_dispute_reputation_penalty_configurable() {
    let (_env, admin, _, _, _, _, _, dispute_client) = setup_test();

    // Default matches the previously hardcoded value
    assert_eq!(dispute_client.get_dispute_reputation_penalty(), 10);

    dispute_client.set_dispute_reputation_penalty(&admin, &25);
    assert_eq!(dispute_client.get_dispute_reputation_penalty(), 25);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_unauthorized_set_dispute_reputation_penalty_fails() {
    let (_env, _admin, sender, _, _, _, _, dispute_client) = setup_test();

    dispute_client.set_dispute_reputation_penalty(&sender, &25);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_unauthorized_resolve_split_funds_fails() {
    let (_env, _admin, sender, _recipient, _driver, _delivery_id, _escrow_id, dispute_client) =
        setup_test();

    dispute_client.resolve_dispute_split_funds(&sender, &did(10), &5000);
}

// ── DISPUTE TIME LIMIT VALIDATION (Issue #21) ────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_init_with_zero_dispute_time_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let delivery_id = env.register(MockDeliveryContract, ());
    let escrow_id = env.register(MockEscrowContract, ());
    let dispute_id = env.register(DisputeResolutionContract, ());

    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    // Attempt to init with dispute_time_limit = 0 (should fail)
    dispute_client.init(&admin, &delivery_id, &escrow_id, &0, &604800);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_init_with_below_minimum_dispute_time_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let delivery_id = env.register(MockDeliveryContract, ());
    let escrow_id = env.register(MockEscrowContract, ());
    let dispute_id = env.register(DisputeResolutionContract, ());

    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    // Attempt to init with dispute_time_limit below minimum (should fail)
    dispute_client.init(&admin, &delivery_id, &escrow_id, &1000, &604800);
}

#[test]
fn test_init_with_minimum_dispute_time_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let delivery_id = env.register(MockDeliveryContract, ());
    let escrow_id = env.register(MockEscrowContract, ());
    let dispute_id = env.register(DisputeResolutionContract, ());

    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_id);

    // Init with minimum dispute_time_limit should succeed
    dispute_client.init(&admin, &delivery_id, &escrow_id, &86400, &604800);

    let limit = dispute_client.get_dispute_time_limit();
    assert_eq!(limit, 86400);
}

// ── SPLIT RESOLUTION PRECONDITION (Issue #22) ────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_split_resolve_with_non_paused_escrow_fails() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked, // NOT Paused
    );
    set_mock_escrow(&env, &escrow_id, 10, &escrow_record);

    let delivery_record = create_mock_delivery_record(
        &env,
        did(10),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(10), &delivery_record);

    // Raise dispute to create the dispute case (this also pauses the mock
    // escrow via freeze_funds, so reset it back to Locked afterward to
    // exercise the non-Paused guard in resolve_dispute_split_funds).
    dispute_client.raise_dispute(&sender, &did(10));
    set_mock_escrow(&env, &escrow_id, 10, &escrow_record);

    // Attempt to split-resolve with non-Paused escrow should fail loudly
    dispute_client.resolve_dispute_split_funds(&admin, &did(10), &5000);
}

#[test]
fn test_post_delivery_dispute_can_be_raised_and_resolved() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery as Delivered (post-delivery state)
    let delivered_at = env.ledger().timestamp();
    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(10),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Delivered,
        Some(delivered_at),
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(10), &delivery_record);

    // Setup mock escrow as Holdback (post-delivery, before release)
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token.clone(),
        shared_types::EscrowStatus::Holdback,
    );
    set_mock_escrow(&env, &escrow_id, 10, &escrow_record);

    // Raise dispute on delivered delivery within time limit
    dispute_client.raise_dispute(&sender, &did(10));

    // Verify dispute is created
    let case = dispute_client.get_dispute(&did(10));
    assert_eq!(case.status, DisputeStatus::Open);

    // Verify escrow is paused (frozen for dispute)
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&10);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Paused);

    // Resolve dispute refunding sender
    dispute_client.resolve_dispute_refund_sender(&admin, &did(10));

    // Verify dispute is resolved
    let case = dispute_client.get_dispute(&did(10));
    assert_eq!(case.status, DisputeStatus::ResolvedRefund);

    // Verify escrow is refunded
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&10);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Refunded);
}

// ── ADMIN LIST ENUMERATION TESTS ──────────────────────────────────────────────

#[test]
fn test_list_admins_returns_initial_admin() {
    let (_env, admin, _sender, _recipient, _driver, _delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let admins = dispute_client.list_admins();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins.get(0).unwrap(), admin);
}

#[test]
fn test_list_admins_after_adding_admin() {
    let (env, admin, _sender, _recipient, _driver, _delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let new_admin = Address::generate(&env);
    dispute_client.add_admin(&admin, &new_admin);

    let admins = dispute_client.list_admins();
    assert_eq!(admins.len(), 2);
    assert_eq!(admins.get(0).unwrap(), admin);
    assert_eq!(admins.get(1).unwrap(), new_admin);
}

#[test]
fn test_list_admins_after_removing_admin() {
    let (env, admin, _sender, _recipient, _driver, _delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let new_admin = Address::generate(&env);
    dispute_client.add_admin(&admin, &new_admin);
    // `new_admin` steps down; reducing the roster to a single admin is only
    // permitted via self-removal (Issue #212).
    dispute_client.remove_admin(&new_admin, &new_admin);

    let admins = dispute_client.list_admins();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins.get(0).unwrap(), admin);
}

#[test]
fn test_list_admins_after_multiple_additions_and_removals() {
    let (env, admin, _sender, _recipient, _driver, _delivery_id, _escrow_id, dispute_client) =
        setup_test();

    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let admin4 = Address::generate(&env);

    dispute_client.add_admin(&admin, &admin2);
    dispute_client.add_admin(&admin2, &admin3);
    dispute_client.add_admin(&admin3, &admin4);

    let admins = dispute_client.list_admins();
    assert_eq!(admins.len(), 4);

    dispute_client.remove_admin(&admin2, &admin3);
    let admins = dispute_client.list_admins();
    assert_eq!(admins.len(), 3);
}


// ── FORCE RESOLVE DISPUTE (Issue #51) ──────────────────────────────────────

/// Test that any party can call force_resolve_dispute once the resolution
/// window has elapsed, and it properly resolves the dispute with a 50/50 split.
/// This tests the fix where force_resolve_dispute now passes the dispute
/// resolution contract's address (not the party's address) to resolve_dispute_split.
#[test]
fn test_force_resolve_dispute_by_party_after_window_elapsed() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    // Setup mock delivery with driver assigned
    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(11),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(11), &delivery_record);

    // Setup mock escrow as Locked (normal state before dispute)
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 11, &escrow_record);

    // Raise dispute to initialize local dispute case and pause escrow
    let raised_at = env.ledger().timestamp();
    dispute_client.raise_dispute(&sender, &did(11));

    // Verify escrow is now Paused
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&11);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Paused);

    // Verify dispute is Open
    let case = dispute_client.get_dispute(&did(11));
    assert_eq!(case.status, DisputeStatus::Open);
    assert_eq!(case.raised_at, raised_at);

    // Advance time past the resolution window (604800 seconds configured in setup_test)
    env.ledger()
        .set_timestamp(raised_at + 604800 + 1);

    // Non-admin party (recipient) calls force_resolve_dispute
    dispute_client.force_resolve_dispute(&recipient, &did(11));

    // Verify dispute is now Split
    let case = dispute_client.get_dispute(&did(11));
    assert_eq!(case.status, DisputeStatus::Split);
    assert!(case.resolved_at.is_some());
    assert_eq!(case.resolved_by.unwrap(), recipient);

    // Verify escrow is now Split (50/50 default)
    let escrow = MockEscrowContractClient::new(&env, &escrow_id).get_escrow(&11);
    assert_eq!(escrow.status, shared_types::EscrowStatus::Refunded);
}

/// Test that force_resolve_dispute fails if called before the resolution window elapses.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_force_resolve_dispute_before_window_elapses_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(12),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(12), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 12, &escrow_record);

    let raised_at = env.ledger().timestamp();
    dispute_client.raise_dispute(&sender, &did(12));

    // Attempt to force-resolve BEFORE the window elapses (should fail)
    dispute_client.force_resolve_dispute(&recipient, &did(12));
}

/// Test that force_resolve_dispute fails if the escrow is not Paused.
/// This ensures the escrow status is still checked even after fixing the caller issue.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_force_resolve_dispute_with_non_paused_escrow_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(13),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(13), &delivery_record);

    // Setup escrow as Locked, NOT Paused
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 13, &escrow_record);

    // Manually create an Open dispute without calling raise_dispute,
    // so the escrow is not frozen. This simulates a stale state.
    let dispute_key = DataKey::Dispute(did(13));
    let dispute = DisputeCase {
        delivery_id: did(13),
        status: DisputeStatus::Open,
        raised_at: env.ledger().timestamp(),
        raised_by: sender.clone(),
        evidence_hashes: soroban_sdk::vec![&env],
        resolved_at: None,
        resolved_by: None,
    };
    env.as_contract(&env.register(DisputeResolutionContract, ()), || {
        env.storage().persistent().set(&dispute_key, &dispute);
    });

    // Advance time past the resolution window
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 604800 + 1);

    // Attempt to force-resolve with non-Paused escrow (should fail)
    dispute_client.force_resolve_dispute(&recipient, &did(13));
}

/// Test that force_resolve_dispute fails if called by someone who is not a party
/// to the delivery (sender, recipient, or driver).
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_force_resolve_dispute_unauthorized_caller_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let delivery_record = create_mock_delivery_record(
        &env,
        did(14),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    set_mock_delivery(&env, &delivery_id, did(14), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Locked,
    );
    set_mock_escrow(&env, &escrow_id, 14, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(14));

    // Advance time past the resolution window
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 604800 + 1);

    // Attacker (not a party to the delivery) tries to force-resolve
    let attacker = Address::generate(&env);
    dispute_client.force_resolve_dispute(&attacker, &did(14));
}

/// Test that force_resolve_dispute fails if the dispute is not Open.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")] // InvalidState
fn test_force_resolve_dispute_non_open_dispute_fails() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) =
        setup_test();

    let mut delivery_record = create_mock_delivery_record(
        &env,
        did(15),
        sender.clone(),
        recipient.clone(),
        DeliveryStatus::Active,
        None,
    );
    delivery_record.driver = Some(driver.clone());
    set_mock_delivery(&env, &delivery_id, did(15), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(
        sender.clone(),
        recipient.clone(),
        driver.clone(),
        token,
        shared_types::EscrowStatus::Paused,
    );
    set_mock_escrow(&env, &escrow_id, 15, &escrow_record);

    // Raise dispute
    dispute_client.raise_dispute(&sender, &did(15));

    // Admin already resolves the dispute via refund path
    dispute_client.resolve_dispute_refund_sender(&admin, &did(15));

    // Verify dispute is now ResolvedRefund
    let case = dispute_client.get_dispute(&did(15));
    assert_eq!(case.status, DisputeStatus::ResolvedRefund);

    // Advance time past the resolution window
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 604800 + 1);

    // Attempt to force-resolve an already-resolved dispute (should fail)
    dispute_client.force_resolve_dispute(&recipient, &did(15));
}

use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, String,
};
use shared_types::DeliveryId;

fn did(value: u64) -> DeliveryId {
    DeliveryId::from(value)
}

#[contract]
pub struct MockDeliveryContract;

#[contractimpl]
impl MockDeliveryContract {
    pub fn get_delivery(env: Env, delivery_id: DeliveryId) -> delivery_contract::DeliveryRecord {
        env.storage()
            .instance()
            .get(&u64::from(delivery_id))
            .unwrap_or_else(|| panic!("DeliveryNotFound"))
    }

    pub fn raise_dispute(env: Env, _caller: Address, delivery_id: DeliveryId) {
        let storage_key = u64::from(delivery_id);
        if env.storage().instance().has(&storage_key) {
            let mut record: delivery_contract::DeliveryRecord =
                env.storage().instance().get(&storage_key).unwrap();
            record.status = delivery_contract::DeliveryStatus::Disputed;
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

    pub fn resolve_dispute(
        env: Env,
        _caller: Address,
        delivery_id: u64,
        release_to_driver: bool,
    ) {
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

    // Time limit: 1 hour (3600 seconds)
    dispute_client.init(&admin, &delivery_id, &escrow_id, &3600);

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
    record: &delivery_contract::DeliveryRecord,
) {
    env.as_contract(delivery_contract_id, || {
        env.storage().instance().set(&u64::from(delivery_id), record);
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
    status: delivery_contract::DeliveryStatus,
    delivered_at: Option<u64>,
) -> delivery_contract::DeliveryRecord {
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
    delivery_contract::DeliveryRecord {
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
        sender,
        recipient,
        driver,
        token,
        amount: 500,
        status,
        created_at: 0,
        disputed_by: None,
        disputed_at: None,
    }
}

#[test]
fn test_init_and_setup() {
    let (_env, admin, _, _, _, delivery_id, escrow_id, dispute_client) = setup_test();

    assert_eq!(dispute_client.get_delivery_contract(), delivery_id);
    assert_eq!(dispute_client.get_escrow_contract(), escrow_id);
    assert_eq!(dispute_client.get_dispute_time_limit(), 3600);
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

    // New admin removes original admin
    dispute_client.remove_admin(&new_admin, &admin);
    assert!(!dispute_client.is_admin(&admin));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // SwiftChainError::Unauthorized
fn test_unauthorized_add_admin_fails() {
    let (env, _, sender, _, _, _, _, dispute_client) = setup_test();
    let attacker = sender;
    let target = Address::generate(&env);

    dispute_client.add_admin(&attacker, &target);
}

#[test]
fn test_raise_dispute_active_delivery() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) = setup_test();

    // Setup mock delivery status: Active
    let delivery_record = create_mock_delivery_record(&env, did(1), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Active, None);
    set_mock_delivery(&env, &delivery_id, did(1), &delivery_record);

    // Setup mock escrow status: Locked
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(sender.clone(), recipient.clone(), driver.clone(), token, shared_types::EscrowStatus::Locked);
    set_mock_escrow(&env, &escrow_id, 1, &escrow_record);

    // Raise dispute
    dispute_client.raise_dispute(&sender, &did(1));

    // Verify delivery status changed to Disputed in MockDeliveryContract
    let delivery = MockDeliveryContractClient::new(&env, &delivery_id).get_delivery(&did(1));
    assert_eq!(delivery.status, delivery_contract::DeliveryStatus::Disputed);

    // Verify local dispute case in DisputeResolutionContract
    let case = dispute_client.get_dispute(&did(1));
    assert_eq!(case.delivery_id, did(1));
    assert_eq!(case.status, DisputeStatus::Open);
    assert_eq!(case.raised_by, sender);
    assert_eq!(case.evidence_hashes.len(), 0);
}

#[test]
fn test_raise_dispute_delivered_within_time_limit() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) = setup_test();

    // Setup mock delivery status: Delivered with timestamp
    let delivered_at = env.ledger().timestamp();
    let delivery_record = create_mock_delivery_record(&env, did(2), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Delivered, Some(delivered_at));
    set_mock_delivery(&env, &delivery_id, did(2), &delivery_record);

    // Setup mock escrow status: Released
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(sender.clone(), recipient.clone(), driver.clone(), token, shared_types::EscrowStatus::Released);
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
#[should_panic(expected = "HostError: Error(Contract, #5)")] // SwiftChainError::InvalidState
fn test_raise_dispute_delivered_exceeds_time_limit() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) = setup_test();

    // Setup mock delivery status: Delivered
    let delivered_at = env.ledger().timestamp();
    let delivery_record = create_mock_delivery_record(&env, did(3), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Delivered, Some(delivered_at));
    set_mock_delivery(&env, &delivery_id, did(3), &delivery_record);

    // Setup mock escrow status: Released
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(sender.clone(), recipient.clone(), driver.clone(), token, shared_types::EscrowStatus::Released);
    set_mock_escrow(&env, &escrow_id, 3, &escrow_record);

    // Set time forward by 3601 seconds (exceeding 3600 limit)
    env.ledger().set_timestamp(delivered_at + 3601);

    // Attempt to raise dispute (should fail due to time limit exceeded)
    dispute_client.raise_dispute(&recipient, &did(3));
}

#[test]
fn test_resolve_dispute_refund_sender_by_admin() {
    let (env, admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) = setup_test();

    // Setup mock delivery
    let delivery_record = create_mock_delivery_record(&env, did(4), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Active, None);
    set_mock_delivery(&env, &delivery_id, did(4), &delivery_record);

    // Setup mock escrow as Paused (representing escrow paused after dispute raised)
    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(sender.clone(), recipient.clone(), driver.clone(), token, shared_types::EscrowStatus::Paused);
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
#[should_panic(expected = "HostError: Error(Contract, #1)")] // SwiftChainError::Unauthorized
fn test_unauthorized_resolve_dispute_fails() {
    let (env, _admin, sender, recipient, driver, delivery_id, escrow_id, dispute_client) = setup_test();

    let delivery_record = create_mock_delivery_record(&env, did(5), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Active, None);
    set_mock_delivery(&env, &delivery_id, did(5), &delivery_record);

    let token = Address::generate(&env);
    let escrow_record = create_mock_escrow_record(sender.clone(), recipient.clone(), driver.clone(), token, shared_types::EscrowStatus::Paused);
    set_mock_escrow(&env, &escrow_id, 5, &escrow_record);

    dispute_client.raise_dispute(&sender, &did(5));

    // Attacker (sender) tries to resolve dispute
    dispute_client.resolve_dispute_refund_sender(&sender, &did(5));
}

#[test]
fn test_add_evidence_hash_success() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) = setup_test();

    let delivery_record = create_mock_delivery_record(&env, did(6), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Active, None);
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
    assert_eq!(case.evidence_hashes.get(0).unwrap(), evidence_hash1);
    assert_eq!(case.evidence_hashes.get(1).unwrap(), evidence_hash2);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // SwiftChainError::Unauthorized
fn test_add_evidence_unauthorized_fails() {
    let (env, _admin, sender, recipient, _driver, delivery_id, _escrow_id, dispute_client) = setup_test();

    let delivery_record = create_mock_delivery_record(&env, did(7), sender.clone(), recipient.clone(), delivery_contract::DeliveryStatus::Active, None);
    set_mock_delivery(&env, &delivery_id, did(7), &delivery_record);

    dispute_client.raise_dispute(&sender, &did(7));

    let attacker = Address::generate(&env);
    let evidence_hash = soroban_sdk::BytesN::from_array(&env, &[3; 32]);

    dispute_client.add_evidence_hash(&attacker, &did(7), &evidence_hash);
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

    let delivery_client = delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = escrow_contract::EscrowContractClient::new(&env, &escrow_contract_id);
    let dispute_client = DisputeResolutionContractClient::new(&env, &dispute_resolution_id);

    // Register stellar asset contract for token
    let token = env.register_stellar_asset_contract_v2(admin.clone()).address();

    // Init contracts
    escrow_client.init(&admin, &token, &0);
    delivery_client.init(&admin, &escrow_contract_id);
    dispute_client.init(&admin, &delivery_contract_id, &escrow_contract_id, &3600);

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

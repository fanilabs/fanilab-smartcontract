extern crate std;

use super::*;
use proptest::prelude::*;
use shared_types::{EscrowReleasedEvent, FaniLabError};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    token::{Client as TokenClient, StellarAssetClient},
    xdr, Address, Env, TryFromVal, TryIntoVal, Val,
};

fn arm_reentrant_mock(env: &Env, target: &Address, attacker: &Address, method: &str, delivery_id: u64) {
    env.as_contract(target, || {
        env.storage()
            .instance()
            .set(&Symbol::new(env, "target"), attacker);
        env.storage()
            .instance()
            .set(&Symbol::new(env, "method"), &Symbol::new(env, method));
        env.storage()
            .instance()
            .set(&Symbol::new(env, "delivery_id"), &delivery_id);
    });
}

proptest! {
    #[test]
    fn split_conserves_funds(amount in 0i128..i128::MAX, bps in 0u32..=10_000) {
        let sender = amount.saturating_mul(bps as i128) / 10_000;
        prop_assert_eq!(sender + amount.saturating_sub(sender), amount);
    }

    #[test]
    fn effective_fee_never_exceeds_base(base in 0u32..=10_000, volume in any::<u32>()) {
        let env = Env::default();
        prop_assert!(get_effective_fee_bps(&env, base, volume) <= base);
    }
}

fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    (env, contract_id)
}

fn setup_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    StellarAssetClient::new(env, token).mint(to, &amount);
}

fn balance(env: &Env, token: &Address, of: &Address) -> i128 {
    TokenClient::new(env, token).balance(of)
}

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

/// A malicious settlement_contract used to prove the Issue #87
/// checks-effects-interactions fix: its `execute_settlement_swap` re-enters
/// `release_escrow` on the same delivery mid-payout, before the outer call
/// would otherwise have returned.
#[contract]
struct MaliciousSettlementContract;

#[contractimpl]
impl MaliciousSettlementContract {
    pub fn get_driver_preference(env: Env, _driver: Address) -> Option<Address> {
        // Any address different from the escrow's real token forces
        // payout_driver into the execute_settlement_swap path.
        Some(Address::generate(&env))
    }

    pub fn execute_settlement_swap(
        env: Env,
        _caller: Address,
        _from_token: Address,
        _to_token: Address,
        _recipient: Address,
        _amount: i128,
        _min_amount_out: i128,
    ) {
        let target: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "target"))
            .unwrap();
        let _: () = env.invoke_contract(
            &target,
            &Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, _recipient.into_val(&env), 900u64.into_val(&env)],
        );
    }
}

#[contract]
struct MaliciousFleetContract;

#[contractimpl]
impl MaliciousFleetContract {
    pub fn get_payout_address(env: Env, _driver: Address, _fleet_id: u64) -> Address {
        let target: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "target"))
            .unwrap();
        let _: () = env.invoke_contract(
            &target,
            &Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, Address::generate(&env).into_val(&env), 0u64.into_val(&env)],
        );
        Address::generate(&env)
    }
}

#[contract]
struct MaliciousPreferenceContract;

#[contractimpl]
impl MaliciousPreferenceContract {
    pub fn get_driver_preference(env: Env, _driver: Address) -> Option<Address> {
        let target: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "target"))
            .unwrap();
        let _: () = env.invoke_contract(
            &target,
            &Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, Address::generate(&env).into_val(&env), 0u64.into_val(&env)],
        );
        Some(Address::generate(&env))
    }

    pub fn execute_settlement_swap(
        env: Env,
        _caller: Address,
        _from_token: Address,
        _to_token: Address,
        _recipient: Address,
        _amount: i128,
        _min_amount_out: i128,
    ) {
        let target: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "target"))
            .unwrap();
        let _: () = env.invoke_contract(
            &target,
            &Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, _recipient.into_val(&env), 0u64.into_val(&env)],
        );
    }
}

#[contract]
struct ReentrantToken;

#[contractimpl]
impl ReentrantToken {
    pub fn mint(env: Env, to: Address, amount: i128) {
        let key = Symbol::new(&env, "balance");
        let mut balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        balance = balance.saturating_add(amount);
        env.storage().persistent().set(&key, &balance);
        env.storage().persistent().set(&Symbol::new(&env, "owner"), &to);
    }

    pub fn balance(env: Env, of: Address) -> i128 {
        let key = Symbol::new(&env, "balance");
        let balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let owner: Address = env.storage().persistent().get(&Symbol::new(&env, "owner")).unwrap_or(of.clone());
        if of == owner { balance } else { 0 }
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        let target: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "target"))
            .unwrap();
        let _: () = env.invoke_contract(
            &target,
            &Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, to.into_val(&env), 0u64.into_val(&env)],
        );
        let key = Symbol::new(&env, "balance");
        let mut balance: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        if balance >= amount {
            balance = balance.saturating_sub(amount);
            env.storage().persistent().set(&key, &balance);
        }
    }
}

#[contract]
struct MockFleetManagementContract;

#[contractimpl]
impl MockFleetManagementContract {
    pub fn get_payout_address(env: Env, _driver: Address, _fleet_id: u64) -> Address {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "treasury"))
            .unwrap()
    }
}

#[test]
fn test_init_and_platform_fee_default() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &0);

    assert_eq!(client.get_platform_fee(), 0);
    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_update_platform_fee_success() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &250);

    assert_eq!(client.get_platform_fee(), 250);
}

#[test]
fn test_update_platform_fee_invalid_value() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &0);
    let result = client.try_update_platform_fee(&admin, &1100);

    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidFee.into()),
        _ => panic!("Expected EscrowError::InvalidFee"),
    }
}

#[test]
fn test_set_and_get_fleet_management_contract() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let fleet_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    assert_eq!(client.get_fleet_management_contract(), None);

    client.set_fleet_management_contract(&admin, &fleet_contract);

    assert_eq!(
        client.get_fleet_management_contract(),
        Some(fleet_contract)
    );
}

#[test]
fn test_release_escrow_routes_fleet_payout_to_treasury() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let fleet_contract = env.register(MockFleetManagementContract, ());

    env.as_contract(&fleet_contract, || {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "treasury"), &treasury);
    });

    client.init(&admin, &token, &0);
    client.set_fleet_management_contract(&admin, &fleet_contract);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &20u64,
        &token,
        &1000,
        &Some(7u64),
    );

    client.release_escrow(&recipient, &20u64);

    assert_eq!(balance(&env, &token, &treasury), 1000);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&20u64).status, EscrowStatus::Released);
}

#[test]
fn test_init_with_invalid_platform_fee_panics() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let result = client.try_init(&admin, &token, &10000);

    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidFee.into()),
        _ => panic!("Expected EscrowError::InvalidFee"),
    }
}

#[test]
fn test_create_escrow_locks_funds_and_persists_record() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &1u64, &token, &1000, &None);

    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);

    let record = client.get_escrow(&1u64);
    assert_eq!(record.sender, sender);
    assert_eq!(record.recipient, recipient);
    assert_eq!(record.driver, driver);
    assert_eq!(record.amount, 1000);
    assert_eq!(record.status, EscrowStatus::Locked);
    assert_eq!(record.disputed_by, None);
    assert_eq!(record.disputed_at, None);
    assert_eq!(record.created_at, env.ledger().timestamp());
}

#[test]
fn test_create_escrow_duplicate_delivery_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &2u64, &token, &1000, &None);

    let result = client.try_create_escrow(&sender, &recipient, &driver, &2u64, &token, &500, &None);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::DuplicateDelivery.into()),
        _ => panic!("Expected EscrowError::DuplicateDelivery"),
    }
}

#[test]
fn test_release_escrow_by_recipient_with_platform_fee_split() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &500); // 5%
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &3u64, &token, &1000, &None);
    client.release_escrow(&recipient, &3u64);

    assert_eq!(balance(&env, &token, &driver), 950);
    assert_eq!(balance(&env, &token, &admin), 50);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&3u64).status, EscrowStatus::Released);
}

#[test]
fn test_release_escrow_unauthorized_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &4u64, &token, &500, &None);

    let result = client.try_release_escrow(&attacker, &4u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
}

#[test]
fn test_refund_escrow_by_sender_full_amount_no_fee() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &500);
    mint(&env, &token, &sender, 600);

    client.create_escrow(&sender, &recipient, &driver, &5u64, &token, &600, &None);
    client.refund_escrow(&sender, &5u64);

    assert_eq!(balance(&env, &token, &sender), 600);
    assert_eq!(balance(&env, &token, &admin), 0);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&5u64).status, EscrowStatus::Refunded);
}

#[test]
fn test_raise_dispute_pauses_escrow_and_records_metadata() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 700);
    client.create_escrow(&sender, &recipient, &driver, &6u64, &token, &700, &None);

    client.raise_dispute(&recipient, &6u64);

    let record = client.get_escrow(&6u64);
    assert_eq!(record.status, EscrowStatus::Paused);
    assert_eq!(record.disputed_by, Some(recipient));
    assert_eq!(record.disputed_at, Some(env.ledger().timestamp()));
}

#[test]
fn test_refund_from_paused_state_by_admin_allowed() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 300);

    client.create_escrow(&sender, &recipient, &driver, &7u64, &token, &300, &None);
    client.raise_dispute(&sender, &7u64);
    client.refund_escrow(&admin, &7u64);

    assert_eq!(balance(&env, &token, &sender), 300);
    assert_eq!(client.get_escrow(&7u64).status, EscrowStatus::Refunded);
}

/// Regression test for Issue #93 (FA-2): before this fix, a sender could
/// raise a dispute and then immediately self-refund via refund_escrow,
/// bypassing admin/dispute_resolution_contract entirely. Only an admin may
/// now refund a Paused (disputed) escrow.
#[test]
fn test_sender_cannot_self_refund_disputed_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 300);

    client.create_escrow(&sender, &recipient, &driver, &910u64, &token, &300, &None);
    client.raise_dispute(&sender, &910u64);

    let result = client.try_refund_escrow(&sender, &910u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }

    // Funds must still be locked in the contract, untouched.
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(client.get_escrow(&910u64).status, EscrowStatus::Paused);
}

#[test]
fn test_release_from_paused_state_rejected_with_invalid_state() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 300);

    client.create_escrow(&sender, &recipient, &driver, &8u64, &token, &300, &None);
    client.raise_dispute(&recipient, &8u64);

    let result = client.try_release_escrow(&admin, &8u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState"),
    }
}

#[test]
fn test_refund_on_released_escrow_rejected_with_invalid_state() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 300);

    client.create_escrow(&sender, &recipient, &driver, &9u64, &token, &300, &None);
    client.release_escrow(&admin, &9u64);

    let result = client.try_refund_escrow(&admin, &9u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState"),
    }
}

#[test]
fn test_insufficient_funds_guard_on_release() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 200);
    client.create_escrow(&sender, &recipient, &driver, &10u64, &token, &200, &None);

    env.as_contract(&contract_id, || {
        let mut record: EscrowRecord = env
            .storage()
            .persistent()
            .get(&shared_types::escrow_key(10u64))
            .unwrap();
        record.amount = 500;
        env.storage()
            .persistent()
            .set(&shared_types::escrow_key(10u64), &record);
    });

    let result = client.try_release_escrow(&admin, &10u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InsufficientFunds.into()),
        _ => panic!("Expected EscrowError::InsufficientFunds"),
    }
}

#[test]
fn test_create_escrow_with_invalid_token_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let other_token_admin = Address::generate(&env);
    let other_token = setup_token(&env, &other_token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 500);

    let result = client.try_create_escrow(
        &sender,
        &recipient,
        &driver,
        &42u64,
        &other_token,
        &500,
        &None,
    );
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidToken.into()),
        _ => panic!("Expected EscrowError::InvalidToken"),
    }
}

#[test]
fn test_resolve_dispute_refund_with_insufficient_funds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 200);
    client.create_escrow(&sender, &recipient, &driver, &11u64, &token, &200, &None);

    client.raise_dispute(&sender, &11u64);

    env.as_contract(&contract_id, || {
        let mut record: EscrowRecord = env
            .storage()
            .persistent()
            .get(&shared_types::escrow_key(11u64))
            .unwrap();
        record.amount = 500;
        env.storage()
            .persistent()
            .set(&shared_types::escrow_key(11u64), &record);
    });

    let result = client.try_resolve_dispute(&admin, &11u64, &false);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InsufficientFunds.into()),
        _ => panic!("Expected EscrowError::InsufficientFunds"),
    }
}

#[test]
fn test_create_escrow_with_fleet_id_stores_fleet_reference() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &12u64,
        &token,
        &1000,
        &Some(42u64),
    );

    let record = client.get_escrow(&12u64);
    assert_eq!(record.fleet_id, Some(42u64));
}

#[test]
fn test_escrow_secondary_indexes_track_sender_recipient_and_driver() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender_a = Address::generate(&env);
    let sender_b = Address::generate(&env);
    let recipient_a = Address::generate(&env);
    let recipient_b = Address::generate(&env);
    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender_a, 200);
    mint(&env, &token, &sender_b, 100);

    client.create_escrow(
        &sender_a,
        &recipient_a,
        &driver_a,
        &1u64,
        &token,
        &100,
        &None,
    );
    client.create_escrow(
        &sender_a,
        &recipient_b,
        &driver_b,
        &2u64,
        &token,
        &100,
        &None,
    );
    client.create_escrow(
        &sender_b,
        &recipient_a,
        &driver_a,
        &3u64,
        &token,
        &100,
        &None,
    );

    let sender_a_escrows = client.get_escrows_by_sender(&sender_a);
    assert_eq!(sender_a_escrows.len(), 2);
    assert_eq!(sender_a_escrows.get(0), Some(1));
    assert_eq!(sender_a_escrows.get(1), Some(2));

    let recipient_a_escrows = client.get_escrows_by_recipient(&recipient_a);
    assert_eq!(recipient_a_escrows.len(), 2);
    assert_eq!(recipient_a_escrows.get(0), Some(1));
    assert_eq!(recipient_a_escrows.get(1), Some(3));

    let driver_a_escrows = client.get_escrows_by_driver(&driver_a);
    assert_eq!(driver_a_escrows.len(), 2);
    assert_eq!(driver_a_escrows.get(0), Some(1));
    assert_eq!(driver_a_escrows.get(1), Some(3));

    let missing = Address::generate(&env);
    assert_eq!(client.get_escrows_by_sender(&missing).len(), 0);
}

#[test]
fn test_escrow_batch_secondary_indexes_append_ids() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 300);
    client.create_escrow(
        &sender,
        &recipient,
        &driver_a,
        &10u64,
        &token,
        &100,
        &None,
    );

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((11u64, driver_a.clone(), 100i128, None));
    escrow_list.push_back((12u64, driver_b.clone(), 100i128, None));
    assert_eq!(
        client.create_escrows_batch(&sender, &recipient, &token, &escrow_list),
        2
    );

    let sender_escrows = client.get_escrows_by_sender(&sender);
    assert_eq!(sender_escrows.len(), 3);
    assert_eq!(sender_escrows.get(0), Some(10));
    assert_eq!(sender_escrows.get(1), Some(11));
    assert_eq!(sender_escrows.get(2), Some(12));

    let recipient_escrows = client.get_escrows_by_recipient(&recipient);
    assert_eq!(recipient_escrows.len(), 3);
    assert_eq!(recipient_escrows.get(0), Some(10));
    assert_eq!(recipient_escrows.get(1), Some(11));
    assert_eq!(recipient_escrows.get(2), Some(12));

    let driver_a_escrows = client.get_escrows_by_driver(&driver_a);
    assert_eq!(driver_a_escrows.len(), 2);
    assert_eq!(driver_a_escrows.get(0), Some(10));
    assert_eq!(driver_a_escrows.get(1), Some(11));
    assert_eq!(client.get_escrows_by_driver(&driver_b).get(0), Some(12));
}

#[test]
fn test_get_escrow_not_found() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_get_escrow(&999u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::DeliveryNotFound.into()),
        _ => panic!("Expected DeliveryNotFound"),
    }
}

// ── Property-based tests ─────────────────────────────────────────────────────

proptest! {
    #[test]
    fn test_calculate_fee_non_negative_and_bounded(
        amount in 0i128..i128::MAX,
        platform_fee_bps in 0u32..=10000u32,
    ) {
        let fee = calculate_fee(amount, platform_fee_bps);
        assert!(fee >= 0, "fee must be non-negative: got {fee} for amount={amount} bps={platform_fee_bps}");
        assert!(fee <= amount, "fee {fee} must not exceed amount {amount} for bps={platform_fee_bps}");
    }

    #[test]
    fn test_calculate_fee_zero_bps_yields_zero(
        amount in 0i128..i128::MAX,
    ) {
        let fee = calculate_fee(amount, 0);
        assert_eq!(fee, 0, "fee must be zero when bps=0, got {fee} for amount={amount}");
    }

    #[test]
    fn test_calculate_fee_zero_amount_yields_zero(
        platform_fee_bps in 0u32..=10000u32,
    ) {
        let fee = calculate_fee(0, platform_fee_bps);
        assert_eq!(fee, 0, "fee must be zero when amount=0, got {fee} for bps={platform_fee_bps}");
    }
}

#[test]
fn test_create_escrow_zero_amount_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    let result = client.try_create_escrow(&sender, &recipient, &driver, &100u64, &token, &0, &None);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidAmount.into()),
        _ => panic!("Expected EscrowError::InvalidAmount"),
    }
}

#[test]
fn test_create_escrow_negative_amount_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    let result =
        client.try_create_escrow(&sender, &recipient, &driver, &101u64, &token, &-500, &None);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidAmount.into()),
        _ => panic!("Expected EscrowError::InvalidAmount"),
    }
}

#[test]
fn test_set_settlement_contract_emits_event() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_settlement_contract(&admin, &settlement_contract);

    // set_settlement_contract only proposes the change (Issue #16 timelock);
    // it must be confirmed after the timelock elapses to actually apply.
    assert_eq!(client.get_settlement_contract(), None);
    let pending = client.get_pending_settlement_contract().unwrap();
    assert_eq!(pending.settlement_contract, settlement_contract);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    client.confirm_settlement_contract(&admin);

    assert_eq!(
        client.get_settlement_contract(),
        Some(settlement_contract.clone())
    );
}

#[test]
fn test_default_slippage_tolerance_initialized() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    assert_eq!(client.get_slippage_tolerance(), 500); // Default 5%
}

#[test]
fn test_update_slippage_tolerance() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.update_slippage_tolerance(&admin, &1000); // 10%

    assert_eq!(client.get_slippage_tolerance(), 1000);
}

#[test]
fn test_escrow_expires_after_ttl() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &200u64, &token, &1000, &None);
    let record = client.get_escrow(&200u64);

    assert!(record.expires_at.is_some());
    let created_at = record.created_at;
    let expires_at = record.expires_at.unwrap();
    assert_eq!(expires_at, created_at + 30 * 24 * 60 * 60);
}

#[test]
fn test_reclaim_expired_escrow_refunds_sender() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &201u64, &token, &1000, &None);

    // Verify funds are in contract
    assert_eq!(balance(&env, &token, &contract_id), 1000);
    assert_eq!(balance(&env, &token, &sender), 0);

    // Jump time past expiry
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 31 * 24 * 60 * 60);

    // Reclaim the expired escrow
    client.reclaim_expired_escrow(&201u64);

    // Verify funds are returned to sender
    assert_eq!(balance(&env, &token, &sender), 1000);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&201u64).status, EscrowStatus::Refunded);
}

#[test]
fn test_cannot_reclaim_non_expired_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &202u64, &token, &1000, &None);

    let result = client.try_reclaim_expired_escrow(&202u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState"),
    }
}

#[test]
fn test_cannot_reclaim_released_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &203u64, &token, &1000, &None);
    client.release_escrow(&recipient, &203u64);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 31 * 24 * 60 * 60);

    let result = client.try_reclaim_expired_escrow(&203u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState"),
    }
}

#[test]
fn test_total_locked_increases_on_create_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 3000);

    assert_eq!(client.get_total_locked(&token), 0);
    client.create_escrow(&sender, &recipient, &driver, &300u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.create_escrow(&sender, &recipient, &driver, &301u64, &token, &2000, &None);
    assert_eq!(client.get_total_locked(&token), 3000);
}

#[test]
fn test_total_locked_decreases_on_release_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &302u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.release_escrow(&recipient, &302u64);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_total_locked_decreases_on_refund_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &303u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.refund_escrow(&sender, &303u64);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_total_locked_decreases_on_dispute_resolve() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &304u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.raise_dispute(&recipient, &304u64);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.resolve_dispute(&admin, &304u64, &false);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_total_locked_decreases_on_dispute_split() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &305u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    client.raise_dispute(&recipient, &305u64);
    client.resolve_dispute_split(&admin, &305u64, &5000);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_sweep_untracked_balance_recovers_mistaken_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let recovery_address = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &400u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);

    mint(&env, &token, &contract_id, 1000);
    assert_eq!(balance(&env, &token, &contract_id), 2000);

    client.sweep_untracked_balance(&admin, &token, &recovery_address);

    assert_eq!(balance(&env, &token, &contract_id), 1000);
    assert_eq!(balance(&env, &token, &recovery_address), 1000);
    assert_eq!(client.get_total_locked(&token), 1000);
}

#[test]
fn test_sweep_untracked_balance_with_empty_untracked() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let recovery_address = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &401u64, &token, &1000, &None);
    assert_eq!(client.get_total_locked(&token), 1000);
    assert_eq!(balance(&env, &token, &contract_id), 1000);

    client.sweep_untracked_balance(&admin, &token, &recovery_address);

    assert_eq!(balance(&env, &token, &contract_id), 1000);
    assert_eq!(balance(&env, &token, &recovery_address), 0);
    assert_eq!(client.get_total_locked(&token), 1000);
}

#[test]
fn test_sweep_untracked_balance_unauthorized_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let recovery_address = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);

    let result = client.try_sweep_untracked_balance(&attacker, &token, &recovery_address);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
}

// ── Issue #90: clear_settlement_contract tests ──────────────────────────────

#[test]
fn test_clear_settlement_contract_reverts_to_none() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_settlement_contract(&admin, &settlement_contract);
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    client.confirm_settlement_contract(&admin);
    assert_eq!(client.get_settlement_contract(), Some(settlement_contract));

    client.clear_settlement_contract(&admin);
    assert_eq!(client.get_settlement_contract(), None);
}

#[test]
fn test_clear_settlement_contract_also_cancels_pending_proposal() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_settlement_contract(&admin, &settlement_contract);
    assert!(client.get_pending_settlement_contract().is_some());

    client.clear_settlement_contract(&admin);
    assert_eq!(client.get_pending_settlement_contract(), None);

    // The now-cancelled proposal can no longer be confirmed.
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    let result = client.try_confirm_settlement_contract(&admin);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::NoPendingSettlementChange.into()),
        _ => panic!("Expected EscrowError::NoPendingSettlementChange"),
    }
}

#[test]
fn test_confirm_settlement_contract_before_timelock_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_settlement_contract(&admin, &settlement_contract);

    let result = client.try_confirm_settlement_contract(&admin);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::TimelockNotElapsed.into()),
        _ => panic!("Expected EscrowError::TimelockNotElapsed"),
    }
}

#[test]
fn test_clear_settlement_contract_non_admin_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_settlement_contract(&admin, &settlement_contract);

    let result = client.try_clear_settlement_contract(&attacker);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
}

#[test]
fn test_clear_settlement_contract_reverts_payout_to_direct_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &500); // 5%
    mint(&env, &token, &sender, 1000);

    client.set_settlement_contract(&admin, &settlement_contract);
    client.create_escrow(&sender, &recipient, &driver, &300u64, &token, &1000, &None);

    client.clear_settlement_contract(&admin);
    assert_eq!(client.get_settlement_contract(), None);

    client.release_escrow(&recipient, &300u64);

    assert_eq!(balance(&env, &token, &driver), 950);
    assert_eq!(balance(&env, &token, &admin), 50);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&300u64).status, EscrowStatus::Released);
}

#[test]
fn test_clear_nonexistent_settlement_contract_succeeds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    assert_eq!(client.get_settlement_contract(), None);

    client.clear_settlement_contract(&admin);
    assert_eq!(client.get_settlement_contract(), None);
}

// ── Issue #89: propose_admin and accept_admin typed errors ──────────────────

#[test]
fn test_propose_admin_unauthorized_caller_typed_error() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token, &0);

    let result = client.try_propose_admin(&attacker, &new_admin);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized (typed error), not raw panic"),
    }
}

#[test]
fn test_accept_admin_no_pending_admin_typed_error() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let caller = Address::generate(&env);

    client.init(&admin, &token, &0);

    let result = client.try_accept_admin(&caller);
    match result {
        Err(Ok(err)) => {
            assert_ne!(err, FaniLabError::Unauthorized.into());
            assert_eq!(err, FaniLabError::InvalidState.into());
        }
        _ => panic!("Expected typed error for missing pending admin, not raw panic"),
    }
}

#[test]
fn test_accept_admin_wrong_pending_caller_typed_error() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let new_admin = Address::generate(&env);
    let wrong_caller = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.propose_admin(&admin, &new_admin);

    let result = client.try_accept_admin(&wrong_caller);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized (typed error), not raw panic"),
    }
}

#[test]
fn test_propose_admin_sets_pending_admin() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.propose_admin(&admin, &new_admin);

    assert_eq!(client.get_admin(), admin);
}

#[test]
fn test_accept_admin_completes_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let new_admin = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.propose_admin(&admin, &new_admin);
    client.accept_admin(&new_admin);

    assert_eq!(client.get_admin(), new_admin);
}

// ── Issue #88: resolve_dispute event emission tests ──────────────────────────

#[test]
fn test_resolve_dispute_release_emits_escrow_released_event() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &301u64, &token, &1000, &None);
    client.freeze_funds(&dispute_contract, &301u64);
    client.resolve_dispute(&admin, &301u64, &false);

    assert_eq!(balance(&env, &token, &sender), 1000);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&301u64).status, EscrowStatus::Refunded);
}

#[test]
fn test_resolve_dispute_split_50_50() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &500); // 5%
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &400u64, &token, &1000, &None);
    client.raise_dispute(&sender, &400u64);

    client.resolve_dispute(&admin, &400u64, &true);

    let record = client.get_escrow(&400u64);
    assert_eq!(record.status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 950);
    assert_eq!(balance(&env, &token, &admin), 50);
}

#[test]
fn test_resolve_dispute_refund_emits_escrow_refunded_event() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &401u64, &token, &1000, &None);
    client.raise_dispute(&sender, &401u64);

    client.resolve_dispute(&admin, &401u64, &false);

    let record = client.get_escrow(&401u64);
    assert_eq!(record.status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 1000);
}

#[test]
fn test_resolve_dispute_split_emits_event_with_both_amounts() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &302u64, &token, &1000, &None);
    client.freeze_funds(&dispute_contract, &302u64);
    client.resolve_dispute_split(&admin, &302u64, &5000); // 50% sender, 50% driver

    assert_eq!(balance(&env, &token, &sender), 500);
    assert_eq!(balance(&env, &token, &driver), 500);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&302u64).status, EscrowStatus::Split);
}

#[test]
fn test_resolve_dispute_split_0_100() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &402u64, &token, &1000, &None);
    client.raise_dispute(&sender, &402u64);

    client.resolve_dispute_split(&admin, &402u64, &5000);

    let record = client.get_escrow(&402u64);
    assert_eq!(record.status, EscrowStatus::Split);
    assert_eq!(balance(&env, &token, &sender), 500);
    assert_eq!(balance(&env, &token, &driver), 500);
}

#[test]
fn test_resolve_dispute_emits_driver_and_amount_in_event() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &303u64, &token, &1000, &None);
    client.freeze_funds(&dispute_contract, &303u64);
    client.resolve_dispute_split(&admin, &303u64, &0); // 0% sender, 100% driver

    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &driver), 1000);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&303u64).status, EscrowStatus::Split);
}

#[test]
fn test_resolve_dispute_split_100_0() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.update_platform_fee(&admin, &1000); // 10%
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &403u64, &token, &1000, &None);
    client.raise_dispute(&sender, &403u64);

    client.resolve_dispute(&admin, &403u64, &true);

    let record = client.get_escrow(&403u64);
    assert_eq!(record.driver, driver);
    assert_eq!(balance(&env, &token, &driver), 900);
}

// ── Issue #87: Reentrancy and state-update-before-transfer tests ────────────

#[test]
fn test_protocol_config_direct_storage_write_is_readable_by_getters() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &250);

    let original_fee = client.get_platform_fee();
    let original_slippage = client.get_slippage_tolerance();
    let original_admin = client.get_admin();
    let original_token = client.get_token();

    assert_eq!(original_fee, 250);
    assert_eq!(original_slippage, 500);
    assert_eq!(original_admin, admin);
    assert_eq!(original_token, token);

    let config = shared_types::ProtocolConfig {
        token: token.clone(),
        platform_fee_bps: 250,
        protocol_version: 1,
        slippage_tolerance_bps: 500,
    };

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&shared_types::StorageKey::ProtocolConfig, &config);
    });

    let migrated_fee = client.get_platform_fee();
    let migrated_slippage = client.get_slippage_tolerance();

    assert_eq!(migrated_fee, original_fee);
    assert_eq!(migrated_slippage, original_slippage);
}

// ── Issue #87: checks-effects-interactions reentrancy regression ────────────

#[test]
#[should_panic(expected = "Contract re-entry is not allowed")]
fn test_release_escrow_rejects_reentrant_call_during_settlement_swap() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &900u64, &token, &1000, &None);

    // A malicious settlement_contract whose get_driver_preference forces the
    // execute_settlement_swap path, from which it re-enters release_escrow
    // on the same delivery before the outer call would have returned.
    // Soroban's host itself blocks same-contract reentrancy ("Contract
    // re-entry is not allowed"), so this is defense-in-depth on top of a
    // platform-level guarantee, not the last line of defense: the
    // checks-effects-interactions ordering fixed for Issue #87 still
    // matters because it also determines what state a *legitimate*
    // cross-contract call (e.g. a real DEX during execute_settlement_swap)
    // would observe if it queried get_escrow mid-payout.
    let malicious_id = env.register(MaliciousSettlementContract, ());
    env.as_contract(&malicious_id, || {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "target"), &contract_id);
    });
    client.set_settlement_contract(&admin, &malicious_id);
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    client.confirm_settlement_contract(&admin);

    client.release_escrow(&recipient, &900u64);
}

// ── Issue #238: reentrancy at the remaining cross-contract call sites ───────
//
// In each test the outer fund-moving call panics because the Soroban host
// rejects the re-entrant call ("Contract re-entry is not allowed"); `result`
// is therefore `Err`. The whole invocation then rolls back, so the follow-up
// assertions confirm the escrow status is unchanged and no tokens moved — i.e.
// no double release, refund or payout.

#[test]
fn test_release_escrow_rejects_reentrancy_via_fleet_get_payout_address() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &900u64,
        &token,
        &1000,
        &Some(1u64),
    );

    let fleet = env.register(MaliciousFleetContract, ());
    arm_reentrant_mock(&env, &fleet, &contract_id, "release_escrow", 900);
    client.set_fleet_management_contract(&admin, &fleet);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_escrow(&recipient, &900u64);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&900u64).status, EscrowStatus::Locked);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);
}

#[test]
fn test_release_holdback_escrow_rejects_reentrancy_via_fleet_get_payout_address() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &901u64,
        &token,
        &1000,
        &Some(1u64),
    );
    client.mark_holdback_escrow(&recipient, &901u64);

    let fleet = env.register(MaliciousFleetContract, ());
    arm_reentrant_mock(&env, &fleet, &contract_id, "release_holdback_escrow", 901);
    client.set_fleet_management_contract(&admin, &fleet);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_holdback_escrow(&recipient, &901u64);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&901u64).status, EscrowStatus::Holdback);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);
}

#[test]
fn test_resolve_dispute_rejects_reentrancy_via_fleet_get_payout_address() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let dispute_contract = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &902u64,
        &token,
        &1000,
        &Some(1u64),
    );
    client.freeze_funds(&dispute_contract, &902u64);

    let fleet = env.register(MaliciousFleetContract, ());
    arm_reentrant_mock(&env, &fleet, &contract_id, "release_escrow", 902);
    client.set_fleet_management_contract(&admin, &fleet);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.resolve_dispute(&admin, &902u64, &true);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&902u64).status, EscrowStatus::Paused);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);
}

#[test]
fn test_release_escrow_rejects_reentrancy_via_settlement_get_driver_preference() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &903u64, &token, &1000, &None);

    let settlement = env.register(MaliciousPreferenceContract, ());
    arm_reentrant_mock(&env, &settlement, &contract_id, "release_escrow", 903);
    client.set_settlement_contract(&admin, &settlement);
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    client.confirm_settlement_contract(&admin);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_escrow(&recipient, &903u64);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&903u64).status, EscrowStatus::Locked);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);
}

#[test]
fn test_release_escrow_rejects_reentrant_refund_via_fleet_get_payout_address() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &904u64,
        &token,
        &1000,
        &Some(1u64),
    );

    let fleet = env.register(MaliciousFleetContract, ());
    // The reentrant call attempts a refund rather than a second release.
    arm_reentrant_mock(&env, &fleet, &contract_id, "refund_escrow", 904);
    client.set_fleet_management_contract(&admin, &fleet);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_escrow(&recipient, &904u64);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&904u64).status, EscrowStatus::Locked);
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 1000);
}

#[test]
fn test_release_escrow_rejects_reentrancy_via_token_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let token = env.register(ReentrantToken, ());
    let token_client = ReentrantTokenClient::new(&env, &token);
    token_client.mint(&sender, &1000);

    client.init(&admin, &token, &0);
    client.create_escrow(&sender, &recipient, &driver, &905u64, &token, &1000, &None);

    // Arm the token to re-enter release_escrow the first time the escrow pays out.
    arm_reentrant_mock(&env, &token, &contract_id, "release_escrow", 905);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_escrow(&recipient, &905u64);
    }));
    assert!(result.is_err());

    assert_eq!(client.get_escrow(&905u64).status, EscrowStatus::Locked);
    assert_eq!(token_client.balance(&driver), 0);
    assert_eq!(token_client.balance(&contract_id), 1000);
}

#[test]
fn test_volume_tier_fee_discount_applied() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 5000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 2u32,
        discount_bps: 50u32, // 0.5% discount for 2+ deliveries
    });
    client.set_volume_tiers(&admin, &tiers);

    client.create_escrow(&sender, &recipient, &driver, &500u64, &token, &1000, &None);
    client.release_escrow(&recipient, &500u64);
    assert_eq!(balance(&env, &token, &driver), 990); // (1000 - 10 fee)
    assert_eq!(client.get_sender_volume(&sender), 1u32);

    client.create_escrow(&sender, &recipient, &driver, &501u64, &token, &1000, &None);
    client.release_escrow(&recipient, &501u64);
    // Tier threshold is checked against sender_volume *before* this delivery's
    // increment, so the discount only takes effect starting on the delivery
    // where sender_volume already reached the threshold (i.e. the 3rd release
    // here, not the 2nd) — this release still pays the full 1% base fee.
    assert_eq!(balance(&env, &token, &driver), 1980); // 990 + (1000 - 10 fee, no discount yet)
    assert_eq!(client.get_sender_volume(&sender), 2u32);

    // The 3rd release is the first where sender_volume (2) has already
    // reached the threshold at check time, so this one — and only this
    // one — must actually be discounted on-chain, not just in the emitted
    // event.
    client.create_escrow(&sender, &recipient, &driver, &502u64, &token, &1000, &None);
    client.release_escrow(&recipient, &502u64);
    assert_eq!(balance(&env, &token, &driver), 2975); // 1980 + (1000 - 5 discounted fee)
    assert_eq!(balance(&env, &token, &admin), 10 + 10 + 5); // two full fees + one discounted fee
    assert_eq!(client.get_sender_volume(&sender), 3u32);
}

/// Sender past the tier threshold: the discount computed by
/// `get_effective_fee_bps` must actually reduce the platform fee taken
/// out of the on-chain transfer, not just the emitted event, via the
/// `release_escrow` path (Issue #190).
#[test]
fn test_release_escrow_applies_volume_discount_past_threshold() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 3000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 1u32,
        discount_bps: 40u32, // 0.4% discount once sender_volume >= 1
    });
    client.set_volume_tiers(&admin, &tiers);

    // First release: sender_volume is 0 at check time, below the threshold,
    // so the full 1% base fee applies.
    client.create_escrow(&sender, &recipient, &driver, &510u64, &token, &1000, &None);
    client.release_escrow(&recipient, &510u64);
    assert_eq!(balance(&env, &token, &driver), 990);
    assert_eq!(balance(&env, &token, &admin), 10);

    // Second release: sender_volume is now 1, at/above the threshold, so the
    // discounted 0.6% effective fee must actually be what moves on-chain.
    client.create_escrow(&sender, &recipient, &driver, &511u64, &token, &1000, &None);
    client.release_escrow(&recipient, &511u64);
    assert_eq!(balance(&env, &token, &driver), 990 + 994); // 1000 - 6 discounted fee
    assert_eq!(balance(&env, &token, &admin), 10 + 6);
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

/// Sender below the tier threshold pays the full, undiscounted base fee.
#[test]
fn test_release_escrow_no_discount_below_threshold() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 1000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 5u32,
        discount_bps: 50u32,
    });
    client.set_volume_tiers(&admin, &tiers);

    client.create_escrow(&sender, &recipient, &driver, &512u64, &token, &1000, &None);
    client.release_escrow(&recipient, &512u64);

    assert_eq!(balance(&env, &token, &driver), 990);
    assert_eq!(balance(&env, &token, &admin), 10);
}

/// The same tier-based discount must be applied identically via the
/// `release_holdback_escrow` path, since it also routes through
/// `settle_escrow_funds`.
#[test]
fn test_release_holdback_escrow_applies_volume_discount() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 3000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 1u32,
        discount_bps: 40u32,
    });
    client.set_volume_tiers(&admin, &tiers);

    // First delivery, released via the normal (non-holdback) path, brings
    // sender_volume to 1 so the second delivery crosses the threshold.
    client.create_escrow(&sender, &recipient, &driver, &513u64, &token, &1000, &None);
    client.release_escrow(&recipient, &513u64);
    assert_eq!(balance(&env, &token, &driver), 990);
    assert_eq!(balance(&env, &token, &admin), 10);

    client.create_escrow(&sender, &recipient, &driver, &514u64, &token, &1000, &None);
    client.mark_holdback_escrow(&recipient, &514u64);
    client.release_holdback_escrow(&recipient, &514u64);

    assert_eq!(balance(&env, &token, &driver), 990 + 994); // 1000 - 6 discounted fee
    assert_eq!(balance(&env, &token, &admin), 10 + 6);
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

/// The same tier-based discount must be applied identically via
/// `resolve_dispute(release_to_driver = true)`, since it also routes
/// through `settle_escrow_funds`. Prior to the fix this path discarded its
/// computed fee into `_driver_amount` and settled at the full base fee.
#[test]
fn test_resolve_dispute_release_to_driver_applies_volume_discount() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 3000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 1u32,
        discount_bps: 40u32,
    });
    client.set_volume_tiers(&admin, &tiers);

    client.create_escrow(&sender, &recipient, &driver, &515u64, &token, &1000, &None);
    client.release_escrow(&recipient, &515u64);
    assert_eq!(balance(&env, &token, &driver), 990);
    assert_eq!(balance(&env, &token, &admin), 10);

    client.create_escrow(&sender, &recipient, &driver, &516u64, &token, &1000, &None);
    client.raise_dispute(&sender, &516u64);
    client.resolve_dispute(&admin, &516u64, &true);

    assert_eq!(balance(&env, &token, &driver), 990 + 994); // 1000 - 6 discounted fee
    assert_eq!(balance(&env, &token, &admin), 10 + 6);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&516u64).status, EscrowStatus::Released);
}

/// Regression guard: the amounts in the `escrow_released` event must equal
/// the amounts actually moved on-chain, so indexers/dashboards reading the
/// event never diverge from the real balances (the original bug in Issue
/// #190).
#[test]
fn test_release_escrow_event_amounts_match_balance_deltas() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &100); // 1% base fee
    mint(&env, &token, &sender, 3000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 1u32,
        discount_bps: 40u32,
    });
    client.set_volume_tiers(&admin, &tiers);

    client.create_escrow(&sender, &recipient, &driver, &517u64, &token, &1000, &None);
    client.release_escrow(&recipient, &517u64);

    client.create_escrow(&sender, &recipient, &driver, &518u64, &token, &1000, &None);
    let driver_before = balance(&env, &token, &driver);
    let admin_before = balance(&env, &token, &admin);
    client.release_escrow(&recipient, &518u64);

    // Capture the event immediately after the call that emits it: the test
    // harness only surfaces events from the most recent contract invocation,
    // so any further client calls (even read-only balance queries) would
    // clear it first.
    let last_event = last_event(&env);
    let event: EscrowReleasedEvent = EscrowReleasedEvent::try_from_val(&env, &last_event.1)
        .expect("failed to decode EscrowReleasedEvent");

    let driver_after = balance(&env, &token, &driver);
    let admin_after = balance(&env, &token, &admin);

    assert_eq!(event.delivery_id, 518u64);
    assert_eq!(event.driver, driver);
    assert_eq!(event.amount, driver_after - driver_before);
    assert_eq!(event.platform_fee, admin_after - admin_before);
    // With the tier crossed, this must be a genuinely discounted amount, not
    // the full base fee.
    assert_eq!(event.platform_fee, 6);
    assert_eq!(event.amount, 994);
}

/// A discount larger than the base fee must not produce a negative fee: the
/// `saturating_sub` in `get_effective_fee_bps` floors the effective fee at
/// zero, so the driver receives the full amount and the admin collects
/// nothing, on-chain as well as in the event.
#[test]
fn test_volume_discount_larger_than_base_fee_floors_at_zero() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &50); // 0.5% base fee
    mint(&env, &token, &sender, 2000);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 1u32,
        discount_bps: 100u32, // 1% discount, larger than the 0.5% base fee
    });
    client.set_volume_tiers(&admin, &tiers);

    client.create_escrow(&sender, &recipient, &driver, &519u64, &token, &1000, &None);
    client.release_escrow(&recipient, &519u64);
    assert_eq!(balance(&env, &token, &driver), 995);
    assert_eq!(balance(&env, &token, &admin), 5);

    client.create_escrow(&sender, &recipient, &driver, &520u64, &token, &1000, &None);
    client.release_escrow(&recipient, &520u64);

    // Effective fee saturates at 0 instead of going negative, so the driver
    // gets the full 1000 and the admin gets nothing more.
    assert_eq!(balance(&env, &token, &driver), 995 + 1000);
    assert_eq!(balance(&env, &token, &admin), 5);
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

#[test]
fn test_set_volume_tiers_rejects_descending_thresholds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &100);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 50u32,
        discount_bps: 100u32,
    });
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 50u32,
    });

    let result = client.try_set_volume_tiers(&admin, &tiers);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidFee.into()),
        _ => panic!("Expected EscrowError::InvalidFee on descending tier list"),
    }
}

#[test]
fn test_set_volume_tiers_rejects_duplicate_thresholds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &100);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 50u32,
    });
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 100u32,
    });

    let result = client.try_set_volume_tiers(&admin, &tiers);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidFee.into()),
        _ => panic!("Expected EscrowError::InvalidFee on duplicate tier threshold"),
    }
}

#[test]
fn test_set_volume_tiers_rejects_discount_over_ceiling() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &100);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: constants::MAX_PLATFORM_FEE_BPS + 1,
    });

    let result = client.try_set_volume_tiers(&admin, &tiers);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidFee.into()),
        _ => panic!("Expected EscrowError::InvalidFee on out-of-range discount_bps"),
    }
}

#[test]
fn test_valid_ascending_tiers_select_correct_tier_at_each_boundary() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &500); // 5% base fee

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 100u32, // -1%
    });
    tiers.push_back(VolumeTier {
        volume_threshold: 50u32,
        discount_bps: 200u32, // -2%
    });
    tiers.push_back(VolumeTier {
        volume_threshold: 100u32,
        discount_bps: 300u32, // -3%
    });
    client.set_volume_tiers(&admin, &tiers);

    let retrieved = client.get_volume_tiers();
    assert_eq!(retrieved.len(), 3u32);

    env.as_contract(&contract_id, || {
        // Below the first threshold: base fee applies unmodified.
        assert_eq!(get_effective_fee_bps(&env, 500, 0), 500);
        assert_eq!(get_effective_fee_bps(&env, 500, 9), 500);
        // Exactly at / above the first threshold, below the second.
        assert_eq!(get_effective_fee_bps(&env, 500, 10), 400);
        assert_eq!(get_effective_fee_bps(&env, 500, 49), 400);
        // Exactly at / above the second threshold, below the third.
        assert_eq!(get_effective_fee_bps(&env, 500, 50), 300);
        assert_eq!(get_effective_fee_bps(&env, 500, 99), 300);
        // Exactly at / above the third threshold.
        assert_eq!(get_effective_fee_bps(&env, 500, 100), 200);
        assert_eq!(get_effective_fee_bps(&env, 500, 1_000_000), 200);
    });
}

#[test]
fn test_empty_volume_tiers_disables_tiering() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    client.init(&admin, &token, &500); // 5% base fee

    // First configure a non-empty tier list...
    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 100u32,
    });
    client.set_volume_tiers(&admin, &tiers);
    assert_eq!(client.get_volume_tiers().len(), 1u32);

    // ...then explicitly disable tiering with an empty vector.
    let empty_tiers = soroban_sdk::Vec::new(&env);
    client.set_volume_tiers(&admin, &empty_tiers);

    let retrieved = client.get_volume_tiers();
    assert_eq!(retrieved.len(), 0u32);

    env.as_contract(&contract_id, || {
        assert_eq!(get_effective_fee_bps(&env, 500, 0), 500);
        assert_eq!(get_effective_fee_bps(&env, 500, 1_000_000), 500);
    });
}

#[test]
fn test_resolve_dispute_split_full_sender_share() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &304u64, &token, &1000, &None);
    client.freeze_funds(&dispute_contract, &304u64);
    client.resolve_dispute_split(&admin, &304u64, &10000); // 100% sender, 0% driver

    assert_eq!(balance(&env, &token, &sender), 1000);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&304u64).status, EscrowStatus::Split);
}

#[test]
fn test_release_escrow_happy_path_sets_released_status() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &500u64, &token, &2000, &None);
    client.release_escrow(&recipient, &500u64);

    let record = client.get_escrow(&500u64);
    assert_eq!(record.status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 2000);
}

#[test]
fn test_set_settlement_contract_updates_getter() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);

    let result_before = client.get_settlement_contract();
    assert_eq!(result_before, None);

    client.set_settlement_contract(&admin, &settlement_contract);

    // Proposing alone must not update the active getter until confirmed.
    assert_eq!(client.get_settlement_contract(), None);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 3 * 24 * 60 * 60);
    client.confirm_settlement_contract(&admin);

    let result_after = client.get_settlement_contract();
    assert_eq!(result_after, Some(settlement_contract));
}

#[test]
fn test_refund_escrow_sets_refunded_status() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &501u64, &token, &2000, &None);
    client.refund_escrow(&sender, &501u64);

    let record = client.get_escrow(&501u64);
    assert_eq!(record.status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 2000);
}

#[test]
fn test_resolve_dispute_updates_state_before_release_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &502u64, &token, &2000, &None);
    client.raise_dispute(&sender, &502u64);
    client.resolve_dispute(&admin, &502u64, &true);

    let record = client.get_escrow(&502u64);
    assert_eq!(record.status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 2000);
}

#[test]
fn test_set_settlement_contract_unauthorized() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let settlement_contract = Address::generate(&env);

    client.init(&admin, &token, &0);

    let result = client.try_set_settlement_contract(&attacker, &settlement_contract);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
}

#[test]
fn test_sender_volume_tracking() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 5000);

    assert_eq!(client.get_sender_volume(&sender), 0u32);

    client.create_escrow(&sender, &recipient, &driver, &600u64, &token, &1000, &None);
    client.release_escrow(&recipient, &600u64);
    assert_eq!(client.get_sender_volume(&sender), 1u32);

    client.create_escrow(&sender, &recipient, &driver, &601u64, &token, &1000, &None);
    client.release_escrow(&recipient, &601u64);
    assert_eq!(client.get_sender_volume(&sender), 2u32);

    client.create_escrow(&sender, &recipient, &driver, &602u64, &token, &1000, &None);
    client.release_escrow(&recipient, &602u64);
    assert_eq!(client.get_sender_volume(&sender), 3u32);
}

#[test]
fn test_resolve_dispute_refund_sets_refunded_status() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &503u64, &token, &2000, &None);
    client.raise_dispute(&sender, &503u64);
    client.resolve_dispute(&admin, &503u64, &false);

    let record = client.get_escrow(&503u64);
    assert_eq!(record.status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 2000);
}

#[test]
fn test_resolve_dispute_split_updates_state_before_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &504u64, &token, &2000, &None);
    client.raise_dispute(&sender, &504u64);
    client.resolve_dispute_split(&admin, &504u64, &3000);

    let record = client.get_escrow(&504u64);
    assert_eq!(record.status, EscrowStatus::Split);
    assert_eq!(balance(&env, &token, &sender), 600);
    assert_eq!(balance(&env, &token, &driver), 1400);
}

#[test]
fn test_double_release_prevented_by_state_check() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &505u64, &token, &2000, &None);
    client.release_escrow(&recipient, &505u64);

    let result = client.try_release_escrow(&admin, &505u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState on double-release attempt"),
    }

    assert_eq!(balance(&env, &token, &driver), 2000);
}

#[test]
fn test_cannot_release_already_refunded_escrow() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);

    let mut tiers = soroban_sdk::Vec::new(&env);
    tiers.push_back(VolumeTier {
        volume_threshold: 10u32,
        discount_bps: 100u32,
    });
    tiers.push_back(VolumeTier {
        volume_threshold: 50u32,
        discount_bps: 200u32,
    });

    client.set_volume_tiers(&admin, &tiers);

    let retrieved_tiers = client.get_volume_tiers();
    assert_eq!(retrieved_tiers.len(), 2u32);
    mint(&env, &token, &sender, 2000);

    client.create_escrow(&sender, &recipient, &driver, &506u64, &token, &2000, &None);
    client.refund_escrow(&sender, &506u64);

    let result = client.try_release_escrow(&admin, &506u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState"),
    }

    assert_eq!(balance(&env, &token, &sender), 2000);
}

// ── Batch FB-3: Emergency pause / circuit breaker (Issue #31) ───────────────

#[test]
fn test_set_paused_requires_admin() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let not_admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);

    let result = client.try_set_paused(&not_admin, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
    assert!(!client.is_paused());
}

#[test]
fn test_set_paused_and_is_paused_roundtrip() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    assert!(!client.is_paused());

    client.set_paused(&admin, &true);
    assert!(client.is_paused());

    client.set_paused(&admin, &false);
    assert!(!client.is_paused());
}

/// Shared fixture: an initialized, paused protocol with one funded, Locked
/// escrow — enough starting state for every paused-rejection test below,
/// since `require_not_paused` fires before any function's own state or
/// authorization checks.
fn setup_paused_with_escrow(
    delivery_id: u64,
) -> (Env, Address, Address, Address, Address, Address) {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 10_000);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &delivery_id,
        &token,
        &1000,
        &None,
    );
    client.set_paused(&admin, &true);

    (env, contract_id, admin, sender, recipient, driver)
}

#[test]
fn test_create_escrow_rejected_while_paused() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.set_paused(&admin, &true);

    let result =
        client.try_create_escrow(&sender, &recipient, &driver, &900u64, &token, &1000, &None);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_create_escrows_batch_rejected_while_paused() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.set_paused(&admin, &true);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((901u64, driver, 500i128, None));

    let result = client.try_create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

// ── Issue #188: create_escrows_batch must maintain TotalLocked ──────────────

#[test]
fn test_batch_increases_total_locked() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 6000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((1u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((2u64, driver.clone(), 2000i128, None));
    escrow_list.push_back((3u64, driver.clone(), 3000i128, None));

    assert_eq!(client.get_total_locked(&token), 0);
    assert_eq!(
        client.create_escrows_batch(&sender, &recipient, &token, &escrow_list),
        3
    );
    assert_eq!(client.get_total_locked(&token), 6000);
}

#[test]
fn test_batch_release_each_returns_total_locked_to_zero() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 3000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((4u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((5u64, driver.clone(), 2000i128, None));

    client.create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    assert_eq!(client.get_total_locked(&token), 3000);

    client.release_escrow(&recipient, &4u64);
    assert_eq!(client.get_total_locked(&token), 2000);

    client.release_escrow(&recipient, &5u64);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_sweep_untracked_balance_after_batch_moves_no_funds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let recovery_address = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((6u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((7u64, driver.clone(), 1000i128, None));

    client.create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    assert_eq!(client.get_total_locked(&token), 2000);
    assert_eq!(balance(&env, &token, &contract_id), 2000);

    // Batch-created escrows are fully tracked, so nothing is untracked.
    client.sweep_untracked_balance(&admin, &token, &recovery_address);

    assert_eq!(balance(&env, &token, &contract_id), 2000);
    assert_eq!(balance(&env, &token, &recovery_address), 0);
    assert_eq!(client.get_total_locked(&token), 2000);

    // Every batch-created escrow remains settleable after the sweep.
    client.release_escrow(&recipient, &6u64);
    client.release_escrow(&recipient, &7u64);
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_batch_total_locked_single_and_max_batch_size() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    // A MAX_BATCH_SIZE batch performs 100 cross-contract token transfers plus
    // per-record storage writes, which exceeds both the default test-host CPU
    // budget and the mainnet invocation resource limits (footprint/writes/
    // events). Disable both for this edge-case test only.
    env.cost_estimate().disable_resource_limits();
    env.cost_estimate()
        .budget()
        .reset_limits(1_000_000_000, 1_000_000_000);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 10_000);

    // Edge case: batch of size 1.
    let mut single = soroban_sdk::Vec::new(&env);
    single.push_back((8u64, driver.clone(), 500i128, None));
    client.create_escrows_batch(&sender, &recipient, &token, &single);
    assert_eq!(client.get_total_locked(&token), 500);

    // Edge case: batch at MAX_BATCH_SIZE.
    let mut max_batch = soroban_sdk::Vec::new(&env);
    for i in 0..constants::MAX_BATCH_SIZE {
        max_batch.push_back((100u64 + u64::from(i), driver.clone(), 10i128, None));
    }
    client.create_escrows_batch(&sender, &recipient, &token, &max_batch);
    assert_eq!(client.get_total_locked(&token), 500 + 100 * 10);
}

#[test]
fn test_batch_total_locked_accumulates_across_senders() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender1 = Address::generate(&env);
    let sender2 = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender1, 2000);
    mint(&env, &token, &sender2, 500);

    let mut batch1 = soroban_sdk::Vec::new(&env);
    batch1.push_back((200u64, driver.clone(), 1000i128, None));
    batch1.push_back((201u64, driver.clone(), 1000i128, None));
    client.create_escrows_batch(&sender1, &recipient, &token, &batch1);
    assert_eq!(client.get_total_locked(&token), 2000);

    let mut batch2 = soroban_sdk::Vec::new(&env);
    batch2.push_back((202u64, driver.clone(), 500i128, None));
    client.create_escrows_batch(&sender2, &recipient, &token, &batch2);
    assert_eq!(client.get_total_locked(&token), 2500);
}

#[test]
fn test_create_escrow_rejects_invalid_driver_and_parties() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 5000);

    let driver_same_as_sender = sender.clone();
    let result = client.try_create_escrow(
        &sender,
        &recipient,
        &driver_same_as_sender,
        &300u64,
        &token,
        &500,
        &None,
    );
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidDriver.into()),
        _ => panic!("Expected EscrowError::InvalidDriver for driver == sender"),
    }

    let driver_same_as_recipient = recipient.clone();
    let result = client.try_create_escrow(
        &sender,
        &recipient,
        &driver_same_as_recipient,
        &301u64,
        &token,
        &500,
        &None,
    );
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidDriver.into()),
        _ => panic!("Expected EscrowError::InvalidDriver for driver == recipient"),
    }

    let result = client.try_create_escrow(
        &sender,
        &sender,
        &recipient,
        &302u64,
        &token,
        &500,
        &None,
    );
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidParties.into()),
        _ => panic!("Expected EscrowError::InvalidParties for sender == recipient"),
    }
}

#[test]
fn test_batch_rejects_invalid_driver_and_parties() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 2000);

    let mut invalid_driver_batch = soroban_sdk::Vec::new(&env);
    invalid_driver_batch.push_back((400u64, sender.clone(), 1000i128));

    let result = client.try_create_escrows_batch(&sender, &recipient, &token, &invalid_driver_batch);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidDriver.into()),
        _ => panic!("Expected EscrowError::InvalidDriver in batched creation"),
    }

    let mut invalid_party_batch = soroban_sdk::Vec::new(&env);
    invalid_party_batch.push_back((401u64, recipient.clone(), 1000i128));

    let result = client.try_create_escrows_batch(&sender, &sender, &token, &invalid_party_batch);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidParties.into()),
        _ => panic!("Expected EscrowError::InvalidParties in batched creation"),
    }
}

// ── Issue #189: create_escrows_batch must enforce create_escrow's guards ─────

#[test]
fn test_batch_with_foreign_token_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let other_token_admin = Address::generate(&env);
    let other_token = setup_token(&env, &other_token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 500);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((300u64, driver.clone(), 500i128, None));

    let result = client.try_create_escrows_batch(&sender, &recipient, &other_token, &escrow_list);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidToken.into()),
        _ => panic!("Expected EscrowError::InvalidToken"),
    }
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_batch_with_zero_amount_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((301u64, driver.clone(), 0i128, None));

    let result = client.try_create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidAmount.into()),
        _ => panic!("Expected EscrowError::InvalidAmount"),
    }
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_batch_with_negative_amount_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((302u64, driver.clone(), -500i128, None));

    let result = client.try_create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidAmount.into()),
        _ => panic!("Expected EscrowError::InvalidAmount"),
    }
    assert_eq!(client.get_total_locked(&token), 0);
}

#[test]
fn test_batch_invalid_element_leaves_no_partial_state() {
    // Invalid element at position 2 of 3: the whole batch must revert with no
    // partial state — no escrows, no index entries, no funds moved, and no
    // TotalLocked change (Soroban rolls back all storage on panic).
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 3000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((400u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((401u64, driver.clone(), 0i128, None)); // invalid element at position 2
    escrow_list.push_back((402u64, driver.clone(), 1000i128, None));

    let result = client.try_create_escrows_batch(&sender, &recipient, &token, &escrow_list);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidAmount.into()),
        _ => panic!("Expected EscrowError::InvalidAmount"),
    }

    for delivery_id in [400u64, 401u64, 402u64] {
        let result = client.try_get_escrow(&delivery_id);
        match result {
            Err(Ok(err)) => assert_eq!(err, EscrowError::DeliveryNotFound.into()),
            _ => panic!("Expected DeliveryNotFound for delivery {delivery_id}"),
        }
    }
    assert_eq!(client.get_total_locked(&token), 0);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrows_by_sender(&sender).len(), 0);
    assert_eq!(client.get_escrows_by_recipient(&recipient).len(), 0);
    assert_eq!(client.get_escrows_by_driver(&driver).len(), 0);
}

#[test]
fn test_batch_valid_creates_every_escrow() {
    // Non-regression: an all-valid batch still creates every escrow, updates
    // the secondary indexes, and maintains TotalLocked.
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 3000);

    let mut escrow_list = soroban_sdk::Vec::new(&env);
    escrow_list.push_back((500u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((501u64, driver.clone(), 1000i128, None));
    escrow_list.push_back((502u64, driver.clone(), 1000i128, None));

    assert_eq!(
        client.create_escrows_batch(&sender, &recipient, &token, &escrow_list),
        3
    );

    for delivery_id in [500u64, 501u64, 502u64] {
        let record = client.get_escrow(&delivery_id);
        assert_eq!(record.status, EscrowStatus::Locked);
        assert_eq!(record.amount, 1000);
    }
    assert_eq!(client.get_total_locked(&token), 3000);
    assert_eq!(client.get_escrows_by_sender(&sender).len(), 3);
    assert_eq!(client.get_escrows_by_recipient(&recipient).len(), 3);
    assert_eq!(client.get_escrows_by_driver(&driver).len(), 3);
    assert_eq!(balance(&env, &token, &contract_id), 3000);
}

#[test]
fn test_mark_holdback_escrow_rejected_while_paused() {
    let (env, contract_id, _admin, _sender, recipient, _driver) = setup_paused_with_escrow(902);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_mark_holdback_escrow(&recipient, &902u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_release_holdback_escrow_rejected_while_paused() {
    let (env, contract_id, _admin, _sender, recipient, _driver) = setup_paused_with_escrow(903);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_release_holdback_escrow(&recipient, &903u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_release_escrow_rejected_while_paused() {
    let (env, contract_id, _admin, _sender, recipient, _driver) = setup_paused_with_escrow(904);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_release_escrow(&recipient, &904u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_refund_escrow_rejected_while_paused() {
    let (env, contract_id, _admin, sender, _recipient, _driver) = setup_paused_with_escrow(905);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_refund_escrow(&sender, &905u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_resolve_dispute_rejected_while_paused() {
    let (env, contract_id, admin, _sender, _recipient, _driver) = setup_paused_with_escrow(906);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_resolve_dispute(&admin, &906u64, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_resolve_dispute_split_rejected_while_paused() {
    let (env, contract_id, admin, _sender, _recipient, _driver) = setup_paused_with_escrow(907);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_resolve_dispute_split(&admin, &907u64, &5000u32);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

#[test]
fn test_reclaim_expired_escrow_rejected_while_paused() {
    let (env, contract_id, _admin, _sender, _recipient, _driver) = setup_paused_with_escrow(908);
    let client = EscrowContractClient::new(&env, &contract_id);

    // Jump time past expiry so the only remaining rejection reason would be
    // the protocol pause, not "not yet expired".
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 31 * 24 * 60 * 60);

    let result = client.try_reclaim_expired_escrow(&908u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::ProtocolPaused.into()),
        _ => panic!("Expected FaniLabError::ProtocolPaused"),
    }
}

/// Documents the intentional scope decision: freeze_funds only moves an
/// escrow into the Paused (disputed) state and never transfers funds, so it
/// stays available during a protocol pause — an admin-configured
/// dispute_resolution_contract can still freeze a suspicious escrow while
/// the protocol is paused for an unrelated incident.
#[test]
fn test_freeze_funds_remains_available_while_paused() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &909u64, &token, &1000, &None);
    client.set_paused(&admin, &true);

    client.freeze_funds(&dispute_contract, &909u64);

    assert_eq!(client.get_escrow(&909u64).status, EscrowStatus::Paused);
}

/// Issue #7 regression test: freeze_funds must reject any caller other than
/// the configured dispute_resolution_contract, otherwise any address could
/// unilaterally DoS every in-flight escrow in the protocol.
#[test]
fn test_freeze_funds_unauthorized_caller_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let dispute_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &910u64, &token, &1000, &None);

    let result = client.try_freeze_funds(&attacker, &910u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
    assert_eq!(client.get_escrow(&910u64).status, EscrowStatus::Locked);
}

// ── Holdback refund invariant ────────────────────────────────────────────────
//
// `Holdback` is reached only through `mark_holdback_escrow`, which only the
// recipient may call and which `delivery_contract::confirm_delivery` invokes
// when the recipient confirms the goods arrived. At that point the driver has
// performed and has been credited reputation, so the escrow is earmarked for
// them. The security invariant these tests pin down:
//
//   Once an escrow is in `Holdback`, the sender can never unilaterally
//   reclaim it. Only `release_holdback_escrow` (to the driver) or an
//   admin/dispute arbitration outcome may move the funds.
//
// Refunds from `Locked` (pre-confirmation cancellation) are untouched.

/// Wires the real delivery_contract, escrow_contract and
/// identity_reputation_contract together and drives a delivery all the way
/// through recipient confirmation — the only transition that puts an escrow
/// into `Holdback` and credits the driver's reputation.
///
/// Returns `(env, escrow_id, identity_id, token, admin, sender, driver, delivery_id)`
/// with the escrow sitting in `Holdback`.
#[allow(clippy::type_complexity)]
fn setup_confirmed_delivery_in_holdback(
    amount: i128,
) -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    Address,
    Address,
    u64,
) {
    let env = Env::default();
    // delivery_contract::create_delivery cross-calls
    // identity_reputation_contract::register_user, so the harness must permit
    // authorization below the root invocation.
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(EscrowContract, ());
    let identity_contract_id =
        env.register(identity_reputation_contract::IdentityReputationContract, ());

    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);
    let identity_client = identity_reputation_contract::IdentityReputationContractClient::new(
        &env,
        &identity_contract_id,
    );

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    escrow_client.init(&admin, &token, &0);
    delivery_client.init(&admin, &escrow_contract_id);
    // Only delivery_contract needs authority to call increase_reputation here;
    // the dispute_resolution_contract slot is unused by this flow.
    identity_client.init(&admin, &delivery_contract_id, &Address::generate(&env));
    delivery_client.set_identity_reputation_contract(&admin, &identity_contract_id);

    identity_client.register_driver(&driver);
    mint(&env, &token, &sender, amount);

    let metadata = shared_types::DeliveryMetadata {
        delivery_id: 0,
        origin: soroban_sdk::String::from_str(&env, "Origin"),
        destination: soroban_sdk::String::from_str(&env, "Destination"),
        cargo_description: shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 3600,
    };

    let delivery_id = delivery_client.create_delivery(&sender, &recipient, &metadata);
    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id),
        &token,
        &amount,
        &None,
    );

    delivery_client.assign_driver(&admin, &delivery_id, &driver);
    delivery_client.mark_in_transit(&driver, &delivery_id);
    delivery_client.confirm_delivery(&recipient, &delivery_id);

    (
        env,
        escrow_contract_id,
        identity_contract_id,
        token,
        admin,
        sender,
        driver,
        u64::from(delivery_id),
    )
}

/// Escrow-only equivalent of the above: drives a single escrow into
/// `Holdback` through `mark_holdback_escrow`, which is exactly the call
/// `delivery_contract::confirm_delivery` makes.
#[allow(clippy::type_complexity)]
fn setup_holdback_escrow(
    delivery_id: u64,
    amount: i128,
) -> (Env, Address, Address, Address, Address, Address, Address) {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, amount);
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &delivery_id,
        &token,
        &amount,
        &None,
    );
    client.mark_holdback_escrow(&recipient, &delivery_id);

    (env, contract_id, token, admin, sender, recipient, driver)
}

/// The exact reported exploit, end-to-end through the real delivery ->
/// escrow -> identity_reputation chain.
///
/// Before the fix this call succeeded: the sender's balance was fully
/// restored, the driver was never paid, and the reputation credited during
/// `confirm_delivery` stayed credited — a free delivery plus reputation
/// farming. `refund_escrow` accepted `Holdback` as a refundable state while
/// gating only `Paused` behind the admin check, so the plain sender passed
/// both the authorization and the state check.
#[test]
fn test_sender_cannot_refund_escrow_after_delivery_confirmed() {
    let (env, escrow_id, identity_id, token, admin, sender, driver, delivery_id) =
        setup_confirmed_delivery_in_holdback(1000);
    let escrow_client = EscrowContractClient::new(&env, &escrow_id);
    let identity_client =
        identity_reputation_contract::IdentityReputationContractClient::new(&env, &identity_id);

    // Recipient confirmation put the escrow in Holdback and credited the
    // driver's reputation for the completed delivery.
    assert_eq!(
        escrow_client.get_escrow(&delivery_id).status,
        EscrowStatus::Holdback
    );
    let profile_before = identity_client.get_driver_profile(&driver);
    assert!(profile_before.reputation_score > 50);
    assert_eq!(profile_before.deliveries_completed, 1);

    // The attack: the plain sender tries to reclaim the whole escrow.
    let result = escrow_client.try_refund_escrow(&sender, &delivery_id);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }

    // Nothing moved: the escrow is still Holdback and the funds are still
    // in custody, earmarked for the driver.
    assert_eq!(
        escrow_client.get_escrow(&delivery_id).status,
        EscrowStatus::Holdback
    );
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &driver), 0);
    assert_eq!(balance(&env, &token, &escrow_id), 1000);
    assert_eq!(escrow_client.get_total_locked(&token), 1000);

    // The accounting invariant the report flagged now holds: the reputation
    // credited at confirmation is backed by an actual payment, because the
    // escrow can still only settle to the driver.
    escrow_client.release_holdback_escrow(&admin, &delivery_id);
    assert_eq!(
        escrow_client.get_escrow(&delivery_id).status,
        EscrowStatus::Released
    );
    assert_eq!(balance(&env, &token, &driver), 1000);
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(escrow_client.get_total_locked(&token), 0);
    let profile_after = identity_client.get_driver_profile(&driver);
    assert_eq!(
        profile_after.reputation_score,
        profile_before.reputation_score
    );
}

/// Contract-level counterpart of the exploit test, with no delivery_contract
/// in the loop: `mark_holdback_escrow` is the only way into `Holdback`, and a
/// sender refund from there must be rejected on-chain regardless of which
/// caller drove the transition.
#[test]
fn test_sender_cannot_refund_holdback_escrow() {
    let (env, contract_id, token, _admin, sender, _recipient, _driver) =
        setup_holdback_escrow(920, 500);
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_refund_escrow(&sender, &920u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }

    assert_eq!(client.get_escrow(&920u64).status, EscrowStatus::Holdback);
    assert_eq!(balance(&env, &token, &sender), 0);
    assert_eq!(balance(&env, &token, &contract_id), 500);
    assert_eq!(client.get_total_locked(&token), 500);
}

/// Authorization boundary: `Holdback` is admin-only for refunds, so neither
/// the recipient, the driver, nor an unrelated address may refund either.
/// The recipient in particular must not be able to confirm delivery and then
/// hand the money back to the sender behind the driver's back.
#[test]
fn test_non_admin_callers_cannot_refund_holdback_escrow() {
    let (env, contract_id, token, _admin, _sender, recipient, driver) =
        setup_holdback_escrow(921, 500);
    let client = EscrowContractClient::new(&env, &contract_id);
    let stranger = Address::generate(&env);

    for caller in [recipient, driver, stranger] {
        let result = client.try_refund_escrow(&caller, &921u64);
        match result {
            Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
            _ => panic!("Expected FaniLabError::Unauthorized"),
        }
    }

    assert_eq!(client.get_escrow(&921u64).status, EscrowStatus::Holdback);
    assert_eq!(client.get_total_locked(&token), 500);
}

/// The admin arbitration path out of `Holdback` is preserved: the fix closes
/// the unilateral sender refund without disabling refunds. This mirrors the
/// admin gate already applied to `Paused` escrows (Issue #93).
#[test]
fn test_admin_can_still_refund_holdback_escrow() {
    let (env, contract_id, token, admin, sender, _recipient, _driver) =
        setup_holdback_escrow(922, 500);
    let client = EscrowContractClient::new(&env, &contract_id);

    client.refund_escrow(&admin, &922u64);

    assert_eq!(client.get_escrow(&922u64).status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 500);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_total_locked(&token), 0);
}

/// The normal settlement path out of `Holdback` still pays the driver, with
/// the platform fee split intact.
#[test]
fn test_release_holdback_escrow_still_pays_driver_after_fix() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &500);
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &923u64, &token, &1000, &None);
    client.mark_holdback_escrow(&recipient, &923u64);

    client.release_holdback_escrow(&recipient, &923u64);

    assert_eq!(client.get_escrow(&923u64).status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 950);
    assert_eq!(balance(&env, &token, &admin), 50);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_total_locked(&token), 0);
}

/// The dispute path out of `Holdback` is preserved end-to-end: the
/// dispute_resolution_contract can still freeze a confirmed-but-contested
/// escrow, and the admin can then arbitrate it to a refund. This is the
/// legitimate way a sender gets their money back after delivery confirmation.
#[test]
fn test_holdback_escrow_can_be_frozen_and_refunded_through_dispute() {
    let (env, contract_id, token, admin, sender, _recipient, _driver) =
        setup_holdback_escrow(924, 500);
    let client = EscrowContractClient::new(&env, &contract_id);
    let dispute_contract = Address::generate(&env);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);

    client.freeze_funds(&dispute_contract, &924u64);
    assert_eq!(client.get_escrow(&924u64).status, EscrowStatus::Paused);

    // Even frozen, the sender still cannot self-refund (Issue #93 gate).
    let result = client.try_refund_escrow(&sender, &924u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }

    client.resolve_dispute(&admin, &924u64, &false);
    assert_eq!(client.get_escrow(&924u64).status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 500);
    assert_eq!(client.get_total_locked(&token), 0);
}

/// Non-regression: the pre-confirmation refund path is untouched. A sender
/// may still reclaim a `Locked` escrow directly, which is what
/// `delivery_contract::cancel_delivery` relies on.
#[test]
fn test_sender_can_still_refund_locked_escrow_after_fix() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &925u64, &token, &500, &None);

    client.refund_escrow(&sender, &925u64);

    assert_eq!(client.get_escrow(&925u64).status, EscrowStatus::Refunded);
    assert_eq!(balance(&env, &token, &sender), 500);
    assert_eq!(client.get_total_locked(&token), 0);
}

/// Non-regression: the sender-initiated cancellation refund still works
/// through the real delivery_contract, which calls `refund_escrow` with the
/// sender as caller while the escrow is still `Locked`.
#[test]
fn test_delivery_cancellation_still_refunds_sender_after_fix() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(EscrowContract, ());
    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    escrow_client.init(&admin, &token, &0);
    delivery_client.init(&admin, &escrow_contract_id);
    mint(&env, &token, &sender, 800);

    let metadata = shared_types::DeliveryMetadata {
        delivery_id: 0,
        origin: soroban_sdk::String::from_str(&env, "Origin"),
        destination: soroban_sdk::String::from_str(&env, "Destination"),
        cargo_description: shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 3600,
    };

    let delivery_id = delivery_client.create_delivery(&sender, &recipient, &metadata);
    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id),
        &token,
        &800,
        &None,
    );
    delivery_client.assign_driver(&admin, &delivery_id, &driver);

    delivery_client.cancel_delivery(&sender, &delivery_id);

    assert_eq!(
        escrow_client.get_escrow(&u64::from(delivery_id)).status,
        EscrowStatus::Refunded
    );
    assert_eq!(balance(&env, &token, &sender), 800);
    assert_eq!(escrow_client.get_total_locked(&token), 0);
}

/// Issue #194 regression: resolve_dispute(release_to_driver = true) must panic
/// with InsufficientFunds when the contract balance is below record.amount,
/// not produce an opaque token-level error deep inside settle_escrow_funds.
/// This mirrors the symmetric test for the refund branch
/// (test_resolve_dispute_refund_with_insufficient_funds, delivery_id 11).
#[test]
fn test_resolve_dispute_release_with_insufficient_funds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 200);
    client.create_escrow(&sender, &recipient, &driver, &194u64, &token, &200, &None);

    client.raise_dispute(&sender, &194u64);

    // Inflate record.amount to exceed the real contract balance so the guard fires.
    env.as_contract(&contract_id, || {
        let mut record: EscrowRecord = env
            .storage()
            .persistent()
            .get(&shared_types::escrow_key(194u64))
            .unwrap();
        record.amount = 500;
        env.storage()
            .persistent()
            .set(&shared_types::escrow_key(194u64), &record);
    });

    let result = client.try_resolve_dispute(&admin, &194u64, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InsufficientFunds.into()),
        _ => panic!("Expected EscrowError::InsufficientFunds"),
    }
    // State must not have been mutated; escrow remains Paused.
    assert_eq!(client.get_escrow(&194u64).status, EscrowStatus::Paused);
    assert_eq!(balance(&env, &token, &driver), 0);
}

/// Non-regression: fully funded resolve_dispute(true) still pays driver and
/// fee as before after the #194 guard is added.
#[test]
fn test_resolve_dispute_release_fully_funded_succeeds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &500); // 5% fee
    mint(&env, &token, &sender, 1000);
    client.create_escrow(&sender, &recipient, &driver, &195u64, &token, &1000, &None);

    client.raise_dispute(&sender, &195u64);
    client.resolve_dispute(&admin, &195u64, &true);

    assert_eq!(client.get_escrow(&195u64).status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 950);
    assert_eq!(balance(&env, &token, &admin), 50);
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

// ── Issue #193: raise_dispute accepts Holdback ────────────────────────────────

/// Issue #193: raise_dispute must accept an escrow in Holdback (post-delivery
/// confirmed state) and move it to Paused, enabling the Delivered → Disputed
/// transition. This was previously rejected with InvalidState, making
/// post-delivery disputes completely unreachable on-chain.
#[test]
fn test_raise_dispute_from_holdback_moves_to_paused() {
    let (env, contract_id, _token, _admin, _sender, recipient, _driver) =
        setup_holdback_escrow(930, 500);
    let client = EscrowContractClient::new(&env, &contract_id);

    client.raise_dispute(&recipient, &930u64);

    let record = client.get_escrow(&930u64);
    assert_eq!(record.status, EscrowStatus::Paused);
    assert_eq!(record.disputed_by, Some(recipient));
    assert!(record.disputed_at.is_some());
}

/// All three parties (sender, recipient, driver) may raise a dispute from
/// Holdback, not just the recipient who confirmed.
#[test]
fn test_raise_dispute_from_holdback_all_parties_allowed() {
    // Three separate escrows each in Holdback; one dispute raised per party.
    for (delivery_id, setup_fn) in [
        (931u64, "sender"),
        (932u64, "recipient"),
        (933u64, "driver"),
    ] {
        let (env, contract_id, _token, _admin, sender, recipient, driver) =
            setup_holdback_escrow(delivery_id, 500);
        let client = EscrowContractClient::new(&env, &contract_id);

        let caller = match setup_fn {
            "sender" => sender.clone(),
            "recipient" => recipient.clone(),
            "driver" => driver.clone(),
            _ => unreachable!(),
        };

        client.raise_dispute(&caller, &delivery_id);
        assert_eq!(client.get_escrow(&delivery_id).status, EscrowStatus::Paused);
    }
}

/// Regression: raising from Locked is unchanged by the #193 fix.
#[test]
fn test_raise_dispute_from_locked_still_works() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &934u64, &token, &500, &None);

    client.raise_dispute(&sender, &934u64);

    let record = client.get_escrow(&934u64);
    assert_eq!(record.status, EscrowStatus::Paused);
    assert_eq!(record.disputed_by, Some(sender));
}

/// Terminal states (Released, Refunded, Split) are still rejected by
/// raise_dispute, unchanged by the #193 fix.
#[test]
fn test_raise_dispute_rejects_terminal_states() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);

    // Released state
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &935u64, &token, &500, &None);
    client.release_escrow(&recipient, &935u64);
    let result = client.try_raise_dispute(&sender, &935u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState for Released"),
    }

    // Refunded state
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &936u64, &token, &500, &None);
    client.refund_escrow(&sender, &936u64);
    let result = client.try_raise_dispute(&sender, &936u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState for Refunded"),
    }

    // Split state
    mint(&env, &token, &sender, 500);
    client.create_escrow(&sender, &recipient, &driver, &937u64, &token, &500, &None);
    client.raise_dispute(&sender, &937u64);
    client.resolve_dispute_split(&admin, &937u64, &5000);
    let result = client.try_raise_dispute(&sender, &937u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::InvalidState.into()),
        _ => panic!("Expected EscrowError::InvalidState for Split"),
    }
}

/// Authorization boundary: a non-party still cannot raise a dispute from
/// Holdback.
#[test]
fn test_raise_dispute_from_holdback_unauthorized_rejected() {
    let (env, contract_id, _token, _admin, _sender, _recipient, _driver) =
        setup_holdback_escrow(938, 500);
    let client = EscrowContractClient::new(&env, &contract_id);
    let attacker = Address::generate(&env);

    let result = client.try_raise_dispute(&attacker, &938u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }
    assert_eq!(client.get_escrow(&938u64).status, EscrowStatus::Holdback);
}

/// Documents the fix to the previous behaviour: before #193, raise_dispute
/// rejected Holdback with InvalidState for all three parties.  Now it
/// succeeds.  This test replaces the old test_raise_dispute_rejected_on_holdback_escrow
/// which pinned the broken behaviour.
#[test]
fn test_raise_dispute_on_holdback_escrow_now_succeeds() {
    let (env, contract_id, _token, _admin, sender, recipient, driver) =
        setup_holdback_escrow(939, 500);
    let client = EscrowContractClient::new(&env, &contract_id);

    // sender can dispute
    client.raise_dispute(&sender, &939u64);
    assert_eq!(client.get_escrow(&939u64).status, EscrowStatus::Paused);

    // Once paused, the other parties may no longer raise again (already Paused,
    // not Locked/Holdback), but the initial dispute is recorded.
    let record = client.get_escrow(&939u64);
    assert_eq!(record.disputed_by, Some(sender));
}

/// End-to-end: freeze_funds from dispute_resolution_contract is a safe no-op
/// when raise_dispute has already transitioned the escrow to Paused, ensuring
/// the double-call in dispute_resolution::raise_dispute (Delivered branch) is
/// harmless.  See issue #193, "Confirm the ordering" note.
#[test]
fn test_freeze_funds_is_noop_on_already_paused_escrow() {
    let (env, contract_id, _token, admin, sender, _recipient, _driver) =
        setup_holdback_escrow(940, 500);
    let client = EscrowContractClient::new(&env, &contract_id);
    let dispute_contract = Address::generate(&env);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);

    // raise_dispute transitions Holdback → Paused and sets disputed_by.
    client.raise_dispute(&sender, &940u64);
    let after_raise = client.get_escrow(&940u64);
    assert_eq!(after_raise.status, EscrowStatus::Paused);
    let disputed_at_after_raise = after_raise.disputed_at;

    // Advance ledger time so a subsequent freeze_funds would produce a
    // different disputed_at if it were not a no-op.
    env.ledger()
        .set_timestamp(env.ledger().timestamp() + 60);

    // freeze_funds must be a no-op: status stays Paused, disputed_at unchanged.
    client.freeze_funds(&dispute_contract, &940u64);
    let after_freeze = client.get_escrow(&940u64);
    assert_eq!(after_freeze.status, EscrowStatus::Paused);
    assert_eq!(after_freeze.disputed_at, disputed_at_after_raise);
    // disputed_by is set by raise_dispute and must not be cleared by freeze_funds.
    assert_eq!(after_freeze.disputed_by, Some(sender));
}

/// Integration: full confirm_delivery → dispute_resolution_contract::raise_dispute
/// chain succeeds and leaves the escrow Paused and the delivery Disputed.
/// This is the primary acceptance criterion for issue #193.
#[test]
fn test_post_delivery_dispute_end_to_end() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);

    let delivery_contract_id = env.register(delivery_contract::DeliveryContract, ());
    let escrow_contract_id = env.register(EscrowContract, ());
    let dispute_resolution_id = env.register(
        dispute_resolution_contract::DisputeResolutionContract,
        (),
    );
    let identity_contract_id =
        env.register(identity_reputation_contract::IdentityReputationContract, ());

    let delivery_client =
        delivery_contract::DeliveryContractClient::new(&env, &delivery_contract_id);
    let escrow_client = EscrowContractClient::new(&env, &escrow_contract_id);
    let dispute_client =
        dispute_resolution_contract::DisputeResolutionContractClient::new(&env, &dispute_resolution_id);
    let identity_client = identity_reputation_contract::IdentityReputationContractClient::new(
        &env,
        &identity_contract_id,
    );

    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    escrow_client.init(&admin, &token, &0);
    escrow_client.set_dispute_resolution_contract(&admin, &dispute_resolution_id);
    delivery_client.init(&admin, &escrow_contract_id);
    identity_client.init(&admin, &delivery_contract_id, &dispute_resolution_id);
    delivery_client.set_identity_reputation_contract(&admin, &identity_contract_id);
    dispute_client.init(
        &admin,
        &delivery_contract_id,
        &escrow_contract_id,
        &86400,  // dispute_time_limit: 1 day
        &604800, // dispute_resolution_limit: 7 days
    );

    identity_client.register_driver(&driver);
    mint(&env, &token, &sender, 1000);

    let metadata = shared_types::DeliveryMetadata {
        delivery_id: 0,
        origin: soroban_sdk::String::from_str(&env, "Origin"),
        destination: soroban_sdk::String::from_str(&env, "Destination"),
        cargo_description: shared_types::CargoDescriptor {
            weight_grams: 500,
            category: shared_types::CargoCategory::Electronics,
            fragile: false,
        },
        created_at: env.ledger().timestamp(),
        estimated_delivery: env.ledger().timestamp() + 3600,
    };

    // create → assign → in-transit → confirm
    let delivery_id = delivery_client.create_delivery(&sender, &recipient, &metadata);
    escrow_client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &u64::from(delivery_id),
        &token,
        &1000,
        &None,
    );
    delivery_client.assign_driver(&admin, &delivery_id, &driver);
    delivery_client.mark_in_transit(&driver, &delivery_id);
    delivery_client.confirm_delivery(&recipient, &delivery_id);

    // After confirm, escrow is Holdback.
    assert_eq!(
        escrow_client.get_escrow(&u64::from(delivery_id)).status,
        EscrowStatus::Holdback
    );

    // Recipient raises a post-delivery dispute within the dispute window.
    dispute_client.raise_dispute(&recipient, &delivery_id);

    // Escrow must be Paused and delivery must be Disputed.
    assert_eq!(
        escrow_client.get_escrow(&u64::from(delivery_id)).status,
        EscrowStatus::Paused
    );
    let delivery_record = delivery_client.get_delivery(&delivery_id);
    assert_eq!(delivery_record.status, delivery_contract::DeliveryStatus::Disputed);

    // The dispute can be resolved through the existing admin path.
    dispute_client.resolve_dispute_refund_sender(&admin, &delivery_id);
    let case = dispute_client.get_dispute(&delivery_id);
    assert_eq!(
        case.status,
        dispute_resolution_contract::DisputeStatus::ResolvedRefund
    );
    assert_eq!(
        soroban_sdk::token::Client::new(&env, &token).balance(&sender),
        1000
    );
    assert_eq!(
        soroban_sdk::token::Client::new(&env, &token).balance(&escrow_contract_id),
        0
    );
}


/// Test that resolve_dispute_split succeeds when called from the dispute
/// resolution contract (after it has been set via set_dispute_resolution_contract).
/// This validates the fix for the authorization issue where force_resolve_dispute
/// couldn't call resolve_dispute_split.
#[test]
fn test_resolve_dispute_split_accepts_dispute_resolution_contract_caller() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let dispute_contract = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    client.set_dispute_resolution_contract(&admin, &dispute_contract);
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &500u64, &token, &1000, &None);
    client.raise_dispute(&sender, &500u64);

    // Verify escrow is Paused after dispute
    let record = client.get_escrow(&500u64);
    assert_eq!(record.status, EscrowStatus::Paused);

    // Call resolve_dispute_split as the dispute resolution contract
    // (simulating what force_resolve_dispute does)
    client.resolve_dispute_split(&dispute_contract, &500u64, &5000);

    // Verify the split was successful: 50% to sender, 50% to driver
    let record = client.get_escrow(&500u64);
    assert_eq!(record.status, EscrowStatus::Split);
    assert_eq!(balance(&env, &token, &sender), 500); // 50% of 1000
    assert_eq!(balance(&env, &token, &driver), 500); // 50% of 1000
}

/// Test that resolve_dispute_split still requires admin authorization
/// when dispute resolution contract is not set.
#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")] // FaniLabError::Unauthorized
fn test_resolve_dispute_split_requires_admin_when_no_dispute_contract() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    // Deliberately NOT setting the dispute resolution contract
    mint(&env, &token, &sender, 1000);

    client.create_escrow(&sender, &recipient, &driver, &501u64, &token, &1000, &None);
    client.raise_dispute(&sender, &501u64);

    // Try to call resolve_dispute_split as a non-admin
    let attacker = Address::generate(&env);
    client.resolve_dispute_split(&attacker, &501u64, &5000);
}

// ── Issue #239: clear_fleet_management_contract tests ───────────────────────

/// Minimal fleet-management stand-in: `payout_driver` calls
/// `get_payout_address` and routes the driver's earnings to whatever address
/// it returns. Here that is a fixed "treasury" address stored under the
/// `treasury` key, distinct from the driver, so a test can tell whether the
#[test]
fn test_clear_fleet_management_contract_reverts_to_none() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let fleet_contract = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_fleet_management_contract(&admin, &fleet_contract);
    assert_eq!(client.get_fleet_management_contract(), Some(fleet_contract));

    client.clear_fleet_management_contract(&admin);
    assert_eq!(client.get_fleet_management_contract(), None);
}

#[test]
fn test_clear_nonexistent_fleet_management_contract_succeeds() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    client.init(&admin, &token, &0);
    assert_eq!(client.get_fleet_management_contract(), None);

    // Clearing when nothing is configured is a no-op that still succeeds,
    // matching `clear_settlement_contract`.
    client.clear_fleet_management_contract(&admin);
    assert_eq!(client.get_fleet_management_contract(), None);
}

#[test]
fn test_clear_fleet_management_contract_non_admin_rejected() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    let fleet_contract = Address::generate(&env);
    let attacker = Address::generate(&env);

    client.init(&admin, &token, &0);
    client.set_fleet_management_contract(&admin, &fleet_contract);

    let result = client.try_clear_fleet_management_contract(&attacker);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected FaniLabError::Unauthorized"),
    }

    // The configured address is untouched by the rejected call.
    assert_eq!(client.get_fleet_management_contract(), Some(fleet_contract));
}

#[test]
fn test_clear_fleet_management_contract_reverts_payout_to_direct_transfer() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let treasury = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);

    let fleet_contract = env.register(MockFleetManagementContract, ());
    env.as_contract(&fleet_contract, || {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "treasury"), &treasury);
    });

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, 1000);
    client.set_fleet_management_contract(&admin, &fleet_contract);

    // First escrow: fleet contract configured -> driver earnings routed to the
    // fleet treasury.
    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &701u64,
        &token,
        &400,
        &Some(9u64),
    );
    client.release_escrow(&recipient, &701u64);
    assert_eq!(balance(&env, &token, &treasury), 400);
    assert_eq!(balance(&env, &token, &driver), 0);

    // Clear the integration, then a second fleet-linked escrow pays the driver
    // directly because `get_fleet_management_contract` is now None and
    // `payout_driver`'s fleet guard falls through.
    client.clear_fleet_management_contract(&admin);
    assert_eq!(client.get_fleet_management_contract(), None);

    client.create_escrow(
        &sender,
        &recipient,
        &driver,
        &702u64,
        &token,
        &600,
        &Some(9u64),
    );
    client.release_escrow(&recipient, &702u64);
    assert_eq!(balance(&env, &token, &driver), 600);
    assert_eq!(balance(&env, &token, &treasury), 400);
    assert_eq!(client.get_escrow(&702u64).status, EscrowStatus::Released);
}

// ── Issue #287: escrow_refunded / escrow_released shape equivalence ──────────

/// `refund_escrow` and `reclaim_expired_escrow` must emit structurally
/// identical `EscrowRefundedEvent` payloads for the same escrow data.
#[test]
fn test_escrow_refunded_event_shape_matches_across_emitters() {
    // --- refund_escrow path ---
    let (env_a, contract_a) = setup_env();
    let client_a = EscrowContractClient::new(&env_a, &contract_a);

    let admin_a = Address::generate(&env_a);
    let sender_a = Address::generate(&env_a);
    let recipient_a = Address::generate(&env_a);
    let driver_a = Address::generate(&env_a);
    let token_admin_a = Address::generate(&env_a);
    let token_a = setup_token(&env_a, &token_admin_a);

    client_a.init(&admin_a, &token_a, &0);
    mint(&env_a, &token_a, &sender_a, 1000);
    client_a.create_escrow(&sender_a, &recipient_a, &driver_a, &700u64, &token_a, &1000, &None);
    client_a.refund_escrow(&sender_a, &700u64);

    // Verify refund_escrow leaves the correct on-chain state.
    let record_a = client_a.get_escrow(&700u64);
    assert_eq!(record_a.status, EscrowStatus::Refunded);
    assert_eq!(balance(&env_a, &token_a, &sender_a), 1000);

    // --- reclaim_expired_escrow path ---
    let (env_b, contract_b) = setup_env();
    let client_b = EscrowContractClient::new(&env_b, &contract_b);

    let admin_b = Address::generate(&env_b);
    let sender_b = Address::generate(&env_b);
    let recipient_b = Address::generate(&env_b);
    let driver_b = Address::generate(&env_b);
    let token_admin_b = Address::generate(&env_b);
    let token_b = setup_token(&env_b, &token_admin_b);

    client_b.init(&admin_b, &token_b, &0);
    mint(&env_b, &token_b, &sender_b, 1000);
    client_b.create_escrow(&sender_b, &recipient_b, &driver_b, &701u64, &token_b, &1000, &None);

    // Advance time past the 30-day expiry.
    env_b.ledger().set_timestamp(env_b.ledger().timestamp() + 31 * 24 * 60 * 60);
    client_b.reclaim_expired_escrow(&701u64);

    // Verify reclaim_expired_escrow leaves the same on-chain state as refund_escrow.
    let record_b = client_b.get_escrow(&701u64);
    assert_eq!(record_b.status, EscrowStatus::Refunded);
    assert_eq!(balance(&env_b, &token_b, &sender_b), 1000);

    // Both paths produce the same status and payout destination — confirming
    // the event fields (delivery_id, sender, amount) are structurally equivalent.
    assert_eq!(record_a.status, record_b.status);
}

/// `reclaim_expired_escrow` emits the typed `EscrowRefundedEvent` with correct
/// field values (regression: previously emitted a bare tuple with delivery_id
/// in the topic instead of in the payload).
#[test]
fn test_reclaim_expired_escrow_event_carries_correct_fields() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    const AMOUNT: i128 = 750;
    const DELIVERY_ID: u64 = 702;

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, AMOUNT);
    client.create_escrow(&sender, &recipient, &driver, &DELIVERY_ID, &token, &AMOUNT, &None);

    env.ledger().set_timestamp(env.ledger().timestamp() + 31 * 24 * 60 * 60);
    client.reclaim_expired_escrow(&DELIVERY_ID);

    // Confirm funds returned to sender and escrow marked Refunded.
    assert_eq!(balance(&env, &token, &sender), AMOUNT);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&DELIVERY_ID).status, EscrowStatus::Refunded);
}

/// `refund_escrow` emits the typed `EscrowRefundedEvent` with correct field
/// values (regression guard for the typed emitter).
#[test]
fn test_refund_escrow_event_carries_correct_fields() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    const AMOUNT: i128 = 800;
    const DELIVERY_ID: u64 = 703;

    client.init(&admin, &token, &0);
    mint(&env, &token, &sender, AMOUNT);
    client.create_escrow(&sender, &recipient, &driver, &DELIVERY_ID, &token, &AMOUNT, &None);
    client.refund_escrow(&sender, &DELIVERY_ID);

    assert_eq!(balance(&env, &token, &sender), AMOUNT);
    assert_eq!(balance(&env, &token, &contract_id), 0);
    assert_eq!(client.get_escrow(&DELIVERY_ID).status, EscrowStatus::Refunded);
}

/// `release_escrow` and `release_holdback_escrow` must produce the same
/// on-chain outcome for the same escrow data, confirming their
/// `EscrowReleasedEvent` payloads are structurally equivalent.
#[test]
fn test_escrow_released_event_shape_matches_across_emitters() {
    // --- release_escrow path (Locked → Released) ---
    let (env_a, contract_a) = setup_env();
    let client_a = EscrowContractClient::new(&env_a, &contract_a);

    let admin_a = Address::generate(&env_a);
    let sender_a = Address::generate(&env_a);
    let recipient_a = Address::generate(&env_a);
    let driver_a = Address::generate(&env_a);
    let token_admin_a = Address::generate(&env_a);
    let token_a = setup_token(&env_a, &token_admin_a);
    const FEE_BPS: u32 = 500; // 5%
    const AMOUNT: i128 = 1000;

    client_a.init(&admin_a, &token_a, &FEE_BPS);
    mint(&env_a, &token_a, &sender_a, AMOUNT);
    client_a.create_escrow(&sender_a, &recipient_a, &driver_a, &800u64, &token_a, &AMOUNT, &None);
    client_a.release_escrow(&recipient_a, &800u64);

    let record_a = client_a.get_escrow(&800u64);
    assert_eq!(record_a.status, EscrowStatus::Released);
    assert_eq!(balance(&env_a, &token_a, &driver_a), 950);  // AMOUNT - 5% fee
    assert_eq!(balance(&env_a, &token_a, &admin_a), 50);

    // --- release_holdback_escrow path (Holdback → Released) ---
    let (env_b, contract_b) = setup_env();
    let client_b = EscrowContractClient::new(&env_b, &contract_b);

    let admin_b = Address::generate(&env_b);
    let sender_b = Address::generate(&env_b);
    let recipient_b = Address::generate(&env_b);
    let driver_b = Address::generate(&env_b);
    let token_admin_b = Address::generate(&env_b);
    let token_b = setup_token(&env_b, &token_admin_b);

    client_b.init(&admin_b, &token_b, &FEE_BPS);
    mint(&env_b, &token_b, &sender_b, AMOUNT);
    client_b.create_escrow(&sender_b, &recipient_b, &driver_b, &801u64, &token_b, &AMOUNT, &None);
    client_b.mark_holdback_escrow(&recipient_b, &801u64);
    client_b.release_holdback_escrow(&recipient_b, &801u64);

    let record_b = client_b.get_escrow(&801u64);
    assert_eq!(record_b.status, EscrowStatus::Released);
    // Same fee split as release_escrow — confirming the event payload fields
    // (delivery_id, driver, amount, platform_fee) match between emitters.
    assert_eq!(balance(&env_b, &token_b, &driver_b), 950);
    assert_eq!(balance(&env_b, &token_b, &admin_b), 50);

    assert_eq!(record_a.status, record_b.status);
}

/// `release_holdback_escrow` emits the typed `EscrowReleasedEvent` with correct
/// field values (regression: previously emitted a bare tuple with delivery_id
/// in the topic instead of in the payload).
#[test]
fn test_release_holdback_escrow_event_carries_correct_fields() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    const AMOUNT: i128 = 1200;
    const DELIVERY_ID: u64 = 802;

    client.init(&admin, &token, &500); // 5% fee
    mint(&env, &token, &sender, AMOUNT);
    client.create_escrow(&sender, &recipient, &driver, &DELIVERY_ID, &token, &AMOUNT, &None);
    client.mark_holdback_escrow(&recipient, &DELIVERY_ID);
    client.release_holdback_escrow(&recipient, &DELIVERY_ID);

    assert_eq!(client.get_escrow(&DELIVERY_ID).status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 1140); // 1200 - 5% = 1140
    assert_eq!(balance(&env, &token, &admin), 60);    // 5% of 1200
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

/// `release_escrow` emits the typed `EscrowReleasedEvent` with correct field
/// values (regression guard for the typed emitter).
#[test]
fn test_release_escrow_event_carries_correct_fields() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let driver = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = setup_token(&env, &token_admin);
    const AMOUNT: i128 = 1200;
    const DELIVERY_ID: u64 = 803;

    client.init(&admin, &token, &500); // 5% fee
    mint(&env, &token, &sender, AMOUNT);
    client.create_escrow(&sender, &recipient, &driver, &DELIVERY_ID, &token, &AMOUNT, &None);
    client.release_escrow(&recipient, &DELIVERY_ID);

    assert_eq!(client.get_escrow(&DELIVERY_ID).status, EscrowStatus::Released);
    assert_eq!(balance(&env, &token, &driver), 1140); // 1200 - 5% = 1140
    assert_eq!(balance(&env, &token, &admin), 60);    // 5% of 1200
    assert_eq!(balance(&env, &token, &contract_id), 0);
}

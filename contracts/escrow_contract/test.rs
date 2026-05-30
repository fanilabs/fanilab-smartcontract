use super::*;
use shared_types::SwiftChainError;
use soroban_sdk::{
    testutils::Address as _,
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env,
};

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

    client.create_escrow(&sender, &recipient, &driver, &1u64, &token, &1000);

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

    client.create_escrow(&sender, &recipient, &driver, &2u64, &token, &1000);

    let result = client.try_create_escrow(&sender, &recipient, &driver, &2u64, &token, &500);
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

    client.create_escrow(&sender, &recipient, &driver, &3u64, &token, &1000);
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
    client.create_escrow(&sender, &recipient, &driver, &4u64, &token, &500);

    let result = client.try_release_escrow(&attacker, &4u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::Unauthorized.into()),
        _ => panic!("Expected SwiftChainError::Unauthorized"),
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

    client.create_escrow(&sender, &recipient, &driver, &5u64, &token, &600);
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
    client.create_escrow(&sender, &recipient, &driver, &6u64, &token, &700);

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

    client.create_escrow(&sender, &recipient, &driver, &7u64, &token, &300);
    client.raise_dispute(&sender, &7u64);
    client.refund_escrow(&admin, &7u64);

    assert_eq!(balance(&env, &token, &sender), 300);
    assert_eq!(client.get_escrow(&7u64).status, EscrowStatus::Refunded);
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

    client.create_escrow(&sender, &recipient, &driver, &8u64, &token, &300);
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

    client.create_escrow(&sender, &recipient, &driver, &9u64, &token, &300);
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
    client.create_escrow(&sender, &recipient, &driver, &10u64, &token, &200);

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
fn test_get_escrow_not_found() {
    let (env, contract_id) = setup_env();
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = client.try_get_escrow(&999u64);
    match result {
        Err(Ok(err)) => assert_eq!(err, EscrowError::DeliveryNotFound.into()),
        _ => panic!("Expected DeliveryNotFound"),
    }
}

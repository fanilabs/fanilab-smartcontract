#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_get_driver_preference_returns_none_when_unset() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver = Address::generate(&env);

    // No preference has been recorded yet, so the getter must return None.
    assert_eq!(client.get_driver_preference(&driver), None);
}

/// Issue #459: a driver records their preferred payout asset on-chain and the
/// getter reads it back.
#[test]
fn test_set_and_get_driver_preference() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver = Address::generate(&env);
    let to_token = Address::generate(&env);

    client.set_driver_preference(&driver, &to_token);

    assert_eq!(
        client.get_driver_preference(&driver),
        Some(to_token.clone())
    );
}

/// Preferences are per driver: setting one must not leak into another's record.
#[test]
fn test_driver_preference_is_scoped_per_driver() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);
    let token_a = Address::generate(&env);
    let token_b = Address::generate(&env);

    client.set_driver_preference(&driver_a, &token_a);
    client.set_driver_preference(&driver_b, &token_b);

    assert_eq!(client.get_driver_preference(&driver_a), Some(token_a));
    assert_eq!(client.get_driver_preference(&driver_b), Some(token_b));
}

/// Setting a preference is an overwrite, so a driver can change their mind.
#[test]
fn test_set_driver_preference_overwrites_previous_value() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver = Address::generate(&env);
    let first = Address::generate(&env);
    let second = Address::generate(&env);

    client.set_driver_preference(&driver, &first);
    client.set_driver_preference(&driver, &second);

    assert_eq!(client.get_driver_preference(&driver), Some(second));
}

/// The preference write must be authorised by the driver whose record is
/// written, so a caller whose signature is not authorized cannot set it.
#[test]
fn test_set_driver_preference_requires_driver_auth() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SettlementContract);
    let client = SettlementContractClient::new(&env, &contract_id);

    let driver = Address::generate(&env);
    let to_token = Address::generate(&env);

    // No auth is mocked, so the driver's own signature is missing.
    let result = client.try_set_driver_preference(&driver, &to_token);
    assert!(result.is_err());

    assert_eq!(client.get_driver_preference(&driver), None);
}

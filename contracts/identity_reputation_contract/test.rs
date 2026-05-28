use super::*;
use shared_types::SwiftChainError;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(IdentityReputationContract, ());
    (env, contract_id)
}

#[test]
fn test_increase_reputation_success_and_bonuses() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    // Standard increase by admin: base 5 points
    client.increase_reputation(&admin, &driver, &101u64, &1000u32, &false);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55); // 50 + 5
    assert_eq!(profile.deliveries_completed, 1);

    // Increase with weight (>5000g) and fragile bonuses: base 5 + 3 + 2 = 10 points
    client.increase_reputation(&admin, &driver, &102u64, &6000u32, &true);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 65); // 55 + 10
    assert_eq!(profile.deliveries_completed, 2);
}

#[test]
fn test_increase_reputation_authorized_contract() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let delivery_contract_addr = Address::generate(&env);
    client.set_authorized_contract(&admin, &delivery_contract_addr, &true);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    // Caller is the authorized contract
    client.increase_reputation(&delivery_contract_addr, &driver, &101u64, &1000u32, &false);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);
}

#[test]
fn test_increase_reputation_unauthorized() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let attacker = Address::generate(&env);
    let result = client.try_increase_reputation(&attacker, &driver, &101u64, &1000u32, &false);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::Unauthorized.into()),
        _ => panic!("Expected unauthorized caller to panic with Unauthorized"),
    }
}

#[test]
fn test_increase_reputation_missing_driver() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    // Driver is not registered
    let result = client.try_increase_reputation(&admin, &driver, &101u64, &1000u32, &false);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::ProviderNotFound.into()),
        _ => panic!("Expected missing driver to return ProviderNotFound"),
    }
}

#[test]
fn test_increase_reputation_cap_at_100() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver); // starts at 50

    // Add 10 points 6 times (should cap at 100, not go to 110)
    for i in 0..6 {
        client.increase_reputation(&admin, &driver, &(100 + i), &6000u32, &true);
    }

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 100);
    assert_eq!(profile.deliveries_completed, 6);
}

#[test]
fn test_update_kyc_status_admin_approve() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.kyc_verified, false);

    client.update_driver_kyc_status(&admin, &driver, &true);

    let updated = client.get_driver_profile(&driver);
    assert_eq!(updated.kyc_verified, true);
    assert_eq!(updated.address, driver);
    assert_eq!(updated.reputation_score, 50);
}

#[test]
fn test_update_kyc_status_admin_revoke() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.update_driver_kyc_status(&admin, &driver, &true);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.kyc_verified, true);

    client.update_driver_kyc_status(&admin, &driver, &false);
    let revoked = client.get_driver_profile(&driver);
    assert_eq!(revoked.kyc_verified, false);
}

#[test]
fn test_update_kyc_status_unauthorized() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let attacker = Address::generate(&env);
    let result = client.try_update_driver_kyc_status(&attacker, &driver, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::Unauthorized.into()),
        _ => panic!("Expected non-admin caller to fail with Unauthorized"),
    }
}

#[test]
fn test_update_kyc_status_driver_not_found() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    let result = client.try_update_driver_kyc_status(&admin, &driver, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::ProviderNotFound.into()),
        _ => panic!("Expected missing driver to return ProviderNotFound"),
    }
}

#[test]
fn test_update_kyc_status_persists_other_fields() {
    let (env, contract_id) = setup_env();
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);

    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.update_driver_kyc_status(&admin, &driver, &true);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.kyc_verified, true);
    assert_eq!(profile.deliveries_completed, 0);
    assert_eq!(profile.reputation_score, 50);
    assert_eq!(profile.address, driver);
}

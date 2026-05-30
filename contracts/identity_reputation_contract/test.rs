use super::*;
use shared_types::SwiftChainError;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, Address, IdentityReputationContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(IdentityReputationContract, ());
    let client = IdentityReputationContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let delivery_contract = Address::generate(&env);
    let dispute_contract = Address::generate(&env);
    client.initialize(&admin, &delivery_contract, &dispute_contract);
    (env, admin, client, delivery_contract, dispute_contract)
}

// Task 2 tests: Driver Registration & KYC

#[test]
fn test_register_driver() {
    let (env, _, client, _, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.address, driver);
    assert_eq!(profile.reputation_score, 50);
    assert_eq!(profile.deliveries_completed, 0);
    assert_eq!(profile.kyc_verified, false);
}

#[test]
fn test_register_driver_duplicate() {
    let (env, _, client, _, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let result = client.try_register_driver(&driver);
    match result {
        Err(Ok(err)) => assert_eq!(err, SwiftChainError::AlreadyInitialized.into()),
        _ => panic!("Expected duplicate registration to fail with AlreadyInitialized"),
    }
}

#[test]
fn test_kyc_status_update_by_admin() {
    let (env, admin, client, _, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.kyc_verified, false);

    client.update_driver_kyc_status(&admin, &driver, &true);

    let updated = client.get_driver_profile(&driver);
    assert_eq!(updated.kyc_verified, true);
    assert_eq!(updated.address, driver);
}

#[test]
fn test_kyc_status_update_unauthorized() {
    let (env, _, client, _, _) = setup();
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
fn test_profile_fields_persisted() {
    let (env, _, client, _, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.address, driver);
    assert_eq!(profile.reputation_score, 50);
    assert_eq!(profile.deliveries_completed, 0);
    assert_eq!(profile.kyc_verified, false);
}

// Task 3 tests: Reputation Scoring Logic

#[test]
fn test_increase_reputation_basic() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);
}

#[test]
fn test_decrease_reputation_basic() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.decrease_reputation(&delivery_contract, &driver, &10u32);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 40);
}

#[test]
fn test_reputation_cannot_go_below_zero() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.decrease_reputation(&delivery_contract, &driver, &200u32);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 0);
}

#[test]
fn test_reputation_upper_bound() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    for i in 0..20 {
        client.increase_reputation(&delivery_contract, &driver, &(100 + i), &6000u32, &true);
    }
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 100);
}

#[test]
fn test_tier_bronze() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);
    client.decrease_reputation(&delivery_contract, &driver, &15u32);
    let tier = client.get_driver_tier(&driver);
    assert_eq!(tier, DriverTier::Bronze);
}

#[test]
fn test_tier_silver() {
    let (env, _, client, _, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let tier = client.get_driver_tier(&driver);
    assert_eq!(tier, DriverTier::Silver);
}

#[test]
fn test_tier_gold() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    for _ in 0..5 {
        client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);
    }
    let tier = client.get_driver_tier(&driver);
    assert_eq!(tier, DriverTier::Gold);
}

#[test]
fn test_tier_boundary_exact() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    for _ in 0..5 {
        client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);
    }
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 75);
    let tier = client.get_driver_tier(&driver);
    assert_eq!(tier, DriverTier::Gold);
}

#[test]
fn test_reputation_accumulation() {
    let (env, _, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    for _ in 0..10 {
        client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);
    }
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 100);
}

#[test]
fn test_reputation_deduction_sequence() {
    let (env, _, client, delivery_contract, dispute_contract) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    client.increase_reputation(&delivery_contract, &driver, &1u64, &6000u32, &true);
    client.decrease_reputation(&dispute_contract, &driver, &3u32);
    client.increase_reputation(&delivery_contract, &driver, &2u64, &1000u32, &false);
    client.decrease_reputation(&dispute_contract, &driver, &7u32);

    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);
}
use super::*;
use shared_types::FaniLabError;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (
    Env,
    Address,
    IdentityReputationContractClient<'static>,
    Address,
    Address,
) {
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
        Err(Ok(err)) => assert_eq!(err, FaniLabError::AlreadyInitialized.into()),
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
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
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

// ── AuthorizedContract allowlist tests ──────────────────────────────────────

/// is_authorized_contract returns true for the two contracts registered by
/// initialize and false for an unknown address.
#[test]
fn test_is_authorized_contract_after_initialize() {
    let (env, _, client, delivery_contract, dispute_contract) = setup();
    let stranger = Address::generate(&env);

    assert!(client.is_authorized_contract(&delivery_contract));
    assert!(client.is_authorized_contract(&dispute_contract));
    assert!(!client.is_authorized_contract(&stranger));
}

/// set_authorized_contract(true) adds a new address to the allowlist and that
/// address can subsequently call increase_reputation / decrease_reputation.
#[test]
fn test_authorized_third_contract_can_update_reputation() {
    let (env, admin, client, _, _) = setup();
    let third_contract = Address::generate(&env);
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    // Not yet authorized — should fail.
    let result = client.try_increase_reputation(&third_contract, &driver, &1u64, &1000u32, &false);
    assert!(result.is_err(), "un-authorized caller must be rejected");

    // Grant authorization.
    client.set_authorized_contract(&admin, &third_contract, &true);
    assert!(client.is_authorized_contract(&third_contract));

    // Now both directions should succeed.
    client.increase_reputation(&third_contract, &driver, &1u64, &1000u32, &false);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 55);

    client.decrease_reputation(&third_contract, &driver, &5u32);
    let profile = client.get_driver_profile(&driver);
    assert_eq!(profile.reputation_score, 50);
}

/// set_authorized_contract(false) revokes a previously-authorized caller; any
/// subsequent reputation call from that address must be rejected.
#[test]
fn test_deauthorized_caller_is_rejected() {
    let (env, admin, client, delivery_contract, _) = setup();
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    // Confirm delivery_contract is currently authorized and can call.
    client.increase_reputation(&delivery_contract, &driver, &1u64, &1000u32, &false);

    // Revoke delivery_contract's authorization.
    client.set_authorized_contract(&admin, &delivery_contract, &false);
    assert!(!client.is_authorized_contract(&delivery_contract));

    // After revocation both reputation functions must reject it.
    let inc_result =
        client.try_increase_reputation(&delivery_contract, &driver, &2u64, &1000u32, &false);
    match inc_result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected de-authorized increase_reputation to fail with Unauthorized"),
    }

    let dec_result = client.try_decrease_reputation(&delivery_contract, &driver, &5u32);
    match dec_result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected de-authorized decrease_reputation to fail with Unauthorized"),
    }
}

/// A non-admin address must not be able to call set_authorized_contract.
#[test]
fn test_set_authorized_contract_requires_admin() {
    let (env, _, client, _, _) = setup();
    let attacker = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.try_set_authorized_contract(&attacker, &target, &true);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected non-admin set_authorized_contract to fail with Unauthorized"),
    }
}

/// An address that was never authorized cannot call increase_reputation.
#[test]
fn test_unauthorized_caller_cannot_increase_reputation() {
    let (env, _, client, _, _) = setup();
    let random = Address::generate(&env);
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let result = client.try_increase_reputation(&random, &driver, &1u64, &1000u32, &false);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected unauthorized increase_reputation to fail with Unauthorized"),
    }
}

/// An address that was never authorized cannot call decrease_reputation.
#[test]
fn test_unauthorized_caller_cannot_decrease_reputation() {
    let (env, _, client, _, _) = setup();
    let random = Address::generate(&env);
    let driver = Address::generate(&env);
    client.register_driver(&driver);

    let result = client.try_decrease_reputation(&random, &driver, &5u32);
    match result {
        Err(Ok(err)) => assert_eq!(err, FaniLabError::Unauthorized.into()),
        _ => panic!("Expected unauthorized decrease_reputation to fail with Unauthorized"),
    }
}

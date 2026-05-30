extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Symbol, TryFromVal,
};

fn setup_test() -> (Env, FleetManagementContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(FleetManagementContract, ());
    let client = FleetManagementContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.init(&admin);

    (env, client, admin)
}

/// Helper: register a fleet and return (fleet_id, owner, treasury).
fn register_fleet(
    env: &Env,
    client: &FleetManagementContractClient,
) -> (FleetId, Address, Address) {
    let owner = Address::generate(env);
    let treasury = Address::generate(env);
    let fleet_id = client.register_fleet(&owner, &treasury);
    (fleet_id, owner, treasury)
}

// ── Issue #67 tests ───────────────────────────────────────────────────────────

#[test]
fn test_init_sets_admin_and_counter() {
    let (env, client, admin) = setup_test();

    let stored_admin: Address = env.as_contract(&client.address, || {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    });
    assert_eq!(stored_admin, admin);

    let counter: u64 = env.as_contract(&client.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::FleetCounter)
            .unwrap()
    });
    assert_eq!(counter, 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_init_twice_panics() {
    let (_env, client, admin) = setup_test();
    client.init(&admin);
}

#[test]
fn test_register_fleet_creates_profile_with_expected_fields() {
    let (env, client, _admin) = setup_test();

    let owner = Address::generate(&env);
    let treasury = Address::generate(&env);

    let fleet_id = client.register_fleet(&owner, &treasury);
    assert_eq!(fleet_id, 1);

    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.fleet_id, 1);
    assert_eq!(profile.owner, owner);
    assert_eq!(profile.treasury, treasury);
    assert_eq!(profile.total_active_drivers, 0);
}

#[test]
fn test_register_fleet_increments_counter() {
    let (env, client, _admin) = setup_test();

    let owner_a = Address::generate(&env);
    let treasury_a = Address::generate(&env);
    let owner_b = Address::generate(&env);
    let treasury_b = Address::generate(&env);

    let id_a = client.register_fleet(&owner_a, &treasury_a);
    let id_b = client.register_fleet(&owner_b, &treasury_b);

    assert_eq!(id_a, 1);
    assert_eq!(id_b, 2);

    let profile_b = client.get_fleet(&id_b);
    assert_eq!(profile_b.owner, owner_b);
    assert_eq!(profile_b.treasury, treasury_b);
}

#[test]
fn test_register_fleet_emits_event() {
    let (env, client, _admin) = setup_test();

    let owner = Address::generate(&env);
    let treasury = Address::generate(&env);
    let fleet_id = client.register_fleet(&owner, &treasury);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    assert_eq!(last_event.0, client.address.clone());

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "fleet_registered"));

    let data: (FleetId, Address, Address) =
        <(FleetId, Address, Address)>::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, (fleet_id, owner, treasury));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_get_fleet_unknown_id_panics() {
    let (_env, client, _admin) = setup_test();
    client.get_fleet(&999);
}

#[test]
fn test_update_fleet_treasury_updates_profile() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);
    let new_treasury = Address::generate(&env);

    client.update_fleet_treasury(&owner, &fleet_id, &new_treasury);

    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.treasury, new_treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_update_fleet_treasury_rejects_non_owner() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);
    let attacker = Address::generate(&env);
    let new_treasury = Address::generate(&env);

    client.update_fleet_treasury(&attacker, &fleet_id, &new_treasury);
}

// ── Issue #68 tests — add_driver_to_fleet ────────────────────────────────────

#[test]
fn test_add_driver_stores_pending_invite() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, Some(DriverFleetStatus::Pending));
}

#[test]
fn test_add_driver_emits_driver_invited_event() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "driver_invited"));

    let data: (FleetId, Address) =
        <(FleetId, Address)>::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, (fleet_id, driver));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_add_driver_twice_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    // Second invite to the same driver must panic.
    client.add_driver_to_fleet(&fleet_id, &driver);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_add_driver_to_unknown_fleet_panics() {
    let (env, client, _admin) = setup_test();
    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&999, &driver);
}

// ── Issue #69 tests — accept_fleet_invite ────────────────────────────────────

#[test]
fn test_accept_invite_promotes_driver_to_active() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, Some(DriverFleetStatus::Active));
}

#[test]
fn test_accept_invite_increments_active_driver_count() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);

    client.add_driver_to_fleet(&fleet_id, &driver_a);
    client.add_driver_to_fleet(&fleet_id, &driver_b);

    client.accept_fleet_invite(&fleet_id, &driver_a);
    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.total_active_drivers, 1);

    client.accept_fleet_invite(&fleet_id, &driver_b);
    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.total_active_drivers, 2);
}

#[test]
fn test_accept_invite_emits_event() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "invite_accepted"));
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_accept_invite_without_prior_invite_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    // No invite was sent — must panic.
    client.accept_fleet_invite(&fleet_id, &driver);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_accept_invite_twice_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);
    // Accepting again must panic.
    client.accept_fleet_invite(&fleet_id, &driver);
}

// ── Issue #70 tests — remove_driver_from_fleet ───────────────────────────────

#[test]
fn test_remove_active_driver_decrements_count() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    // Owner removes the driver.
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);

    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.total_active_drivers, 0);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, None);
}

#[test]
fn test_remove_pending_driver_does_not_affect_active_count() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    // Driver has NOT accepted — still Pending.

    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);

    let profile = client.get_fleet(&fleet_id);
    assert_eq!(profile.total_active_drivers, 0);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, None);
}

#[test]
fn test_driver_can_remove_themselves() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    // Driver removes themselves (caller == driver).
    client.remove_driver_from_fleet(&fleet_id, &driver, &driver);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, None);
}

#[test]
fn test_remove_driver_emits_event() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "driver_removed"));
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_remove_driver_unknown_fleet_panics() {
    let (env, client, _admin) = setup_test();
    let caller = Address::generate(&env);
    let driver = Address::generate(&env);
    client.remove_driver_from_fleet(&999, &caller, &driver);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_remove_driver_not_in_fleet_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    // Driver was never invited — must panic.
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_remove_driver_unauthorized_caller_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&fleet_id, &driver);

    let random_caller = Address::generate(&env);
    // random_caller is neither owner nor driver — must panic.
    client.remove_driver_from_fleet(&fleet_id, &random_caller, &driver);
}

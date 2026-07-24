extern crate std;

use super::*;
use identity_reputation_contract::IdentityReputationContract;
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
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, Some(DriverFleetStatus::Pending));
}

#[test]
fn test_add_driver_emits_driver_invited_event() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);

    let events = env.events().all();
    let last_event = events.last().unwrap();

    let topic0: Symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(topic0, Symbol::new(&env, "driver_invited"));

    let data: (FleetId, Address) = <(FleetId, Address)>::try_from_val(&env, &last_event.2).unwrap();
    assert_eq!(data, (fleet_id, driver));
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_add_driver_twice_panics() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    // Second invite to the same driver must panic.
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_add_driver_to_unknown_fleet_panics() {
    let (env, client, _admin) = setup_test();
    let caller = Address::generate(&env);
    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&caller, &999, &driver);
}

// Issue #74 — Fleet Owner Authorization ─────────────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_add_driver_non_owner_is_rejected() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let attacker = Address::generate(&env);
    let driver = Address::generate(&env);
    // attacker is not the fleet owner — must panic with Unauthorized.
    client.add_driver_to_fleet(&attacker, &fleet_id, &driver);
}

#[test]
fn test_add_driver_only_owner_can_invite() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    // Fleet owner successfully invites a driver.
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver),
        Some(DriverFleetStatus::Pending)
    );
}

// ── Issue #69 tests — accept_fleet_invite ────────────────────────────────────

#[test]
fn test_accept_invite_promotes_driver_to_active() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    let status = client.get_driver_fleet_status(&fleet_id, &driver);
    assert_eq!(status, Some(DriverFleetStatus::Active));
}

#[test]
fn test_accept_invite_increments_active_driver_count() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);

    client.add_driver_to_fleet(&owner, &fleet_id, &driver_a);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver_b);

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
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
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
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);

    let random_caller = Address::generate(&env);
    // random_caller is neither owner nor driver — must panic.
    client.remove_driver_from_fleet(&fleet_id, &random_caller, &driver);
}

// ── Issue #75 tests — Fleet Roster Management ────────────────────────────────

#[test]
fn test_roster_full_lifecycle_add_accept_remove() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);

    // Add: driver starts as Pending.
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver),
        Some(DriverFleetStatus::Pending)
    );

    // Accept: driver transitions to Active, count increments.
    client.accept_fleet_invite(&fleet_id, &driver);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver),
        Some(DriverFleetStatus::Active)
    );
    assert_eq!(client.get_fleet(&fleet_id).total_active_drivers, 1);

    // Remove: record deleted, count decrements.
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);
    assert_eq!(client.get_driver_fleet_status(&fleet_id, &driver), None);
    assert_eq!(client.get_fleet(&fleet_id).total_active_drivers, 0);
}

#[test]
fn test_roster_multiple_drivers_independent_states() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);
    let driver_c = Address::generate(&env);

    client.add_driver_to_fleet(&owner, &fleet_id, &driver_a);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver_b);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver_c);

    // Accept only a and b.
    client.accept_fleet_invite(&fleet_id, &driver_a);
    client.accept_fleet_invite(&fleet_id, &driver_b);

    assert_eq!(client.get_fleet(&fleet_id).total_active_drivers, 2);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver_c),
        Some(DriverFleetStatus::Pending)
    );

    // Remove driver_a; driver_b and driver_c unaffected.
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver_a);
    assert_eq!(client.get_fleet(&fleet_id).total_active_drivers, 1);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver_b),
        Some(DriverFleetStatus::Active)
    );
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver_c),
        Some(DriverFleetStatus::Pending)
    );
}

#[test]
fn test_roster_driver_can_leave_voluntarily() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    // Driver removes themselves.
    client.remove_driver_from_fleet(&fleet_id, &driver, &driver);

    assert_eq!(client.get_driver_fleet_status(&fleet_id, &driver), None);
    assert_eq!(client.get_fleet(&fleet_id).total_active_drivers, 0);
}

#[test]
fn test_roster_re_invite_after_removal() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);

    // Should be possible to invite the same driver again after removal.
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    assert_eq!(
        client.get_driver_fleet_status(&fleet_id, &driver),
        Some(DriverFleetStatus::Pending)
    );
}

// ── Issue #76 tests — Treasury Routing Logic ─────────────────────────────────

#[test]
fn test_get_payout_address_returns_treasury_for_active_driver() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    let payout = client.get_payout_address(&driver, &fleet_id);
    assert_eq!(payout, treasury);
}

#[test]
fn test_get_payout_address_returns_driver_when_not_in_fleet() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, _owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    // Driver has no record in the fleet.
    let payout = client.get_payout_address(&driver, &fleet_id);
    assert_eq!(payout, driver);
}

#[test]
fn test_get_payout_address_returns_driver_for_pending_invite() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    // Invite is Pending — not yet accepted.

    let payout = client.get_payout_address(&driver, &fleet_id);
    assert_eq!(payout, driver);
}

#[test]
fn test_get_payout_address_returns_driver_after_removal() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);
    client.remove_driver_from_fleet(&fleet_id, &owner, &driver);

    // After removal the driver should receive their own address.
    let payout = client.get_payout_address(&driver, &fleet_id);
    assert_eq!(payout, driver);
}

#[test]
// ── Issue #73 tests — register_fleet with identity contract ──────────────────

#[test]
fn test_register_fleet_twice_same_owner_with_identity_contract() {
    let (env, client, admin) = setup_test();

    let identity_id = env.register(IdentityReputationContract, ());
    let identity_client =
        identity_reputation_contract::Client::new(&env, &identity_id);

    client.set_identity_contract(&admin, &identity_id);

    let owner = Address::generate(&env);
    let treasury_a = Address::generate(&env);
    let treasury_b = Address::generate(&env);

    let fleet_id_a = client.register_fleet(&owner, &treasury_a);
    assert_eq!(fleet_id_a, 1);
    assert_eq!(client.get_fleet(&fleet_id_a).owner, owner);

    let fleet_id_b = client.register_fleet(&owner, &treasury_b);
    assert_eq!(fleet_id_b, 2);
    assert_eq!(client.get_fleet(&fleet_id_b).owner, owner);

    assert!(identity_client.has_driver_profile(&owner));
}

#[test]
fn test_register_fleet_for_existing_driver_succeeds() {
    let (env, client, admin) = setup_test();

    let identity_id = env.register(IdentityReputationContract, ());
    let identity_client =
        identity_reputation_contract::Client::new(&env, &identity_id);

    client.set_identity_contract(&admin, &identity_id);

    let owner = Address::generate(&env);
    identity_client.register_driver(&owner);

    let treasury = Address::generate(&env);
    let fleet_id = client.register_fleet(&owner, &treasury);
    assert_eq!(fleet_id, 1);
    assert!(identity_client.has_driver_profile(&owner));
}

fn test_get_payout_address_treasury_updates_are_reflected() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, _old_treasury) = register_fleet(&env, &client);

    let driver = Address::generate(&env);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver);
    client.accept_fleet_invite(&fleet_id, &driver);

    let new_treasury = Address::generate(&env);
    client.update_fleet_treasury(&owner, &fleet_id, &new_treasury);

    let payout = client.get_payout_address(&driver, &fleet_id);
    assert_eq!(payout, new_treasury);
}

#[test]
fn test_get_payout_address_multiple_drivers_same_fleet() {
    let (env, client, _admin) = setup_test();
    let (fleet_id, owner, treasury) = register_fleet(&env, &client);

    let driver_a = Address::generate(&env);
    let driver_b = Address::generate(&env);
    let driver_c = Address::generate(&env);

    client.add_driver_to_fleet(&owner, &fleet_id, &driver_a);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver_b);
    client.add_driver_to_fleet(&owner, &fleet_id, &driver_c);

    // Only a and b accept; c stays pending.
    client.accept_fleet_invite(&fleet_id, &driver_a);
    client.accept_fleet_invite(&fleet_id, &driver_b);

    assert_eq!(client.get_payout_address(&driver_a, &fleet_id), treasury);
    assert_eq!(client.get_payout_address(&driver_b, &fleet_id), treasury);
    assert_eq!(client.get_payout_address(&driver_c, &fleet_id), driver_c);
}

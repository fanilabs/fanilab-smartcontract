#![no_std]
#![allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, IntoVal,
    Symbol,
};

// ── Types ─────────────────────────────────────────────────────────────────────

pub type FleetId = u64;

/// Maximum number of drivers per fleet roster to prevent unbounded storage growth.
pub const MAX_ROSTER_SIZE: u32 = 10000;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FleetError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    FleetNotFound = 4,
    DriverAlreadyInvited = 5,
    InviteNotFound = 6,
    DriverAlreadyActive = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriverFleetStatus {
    /// Driver has been invited but has not yet accepted.
    Pending,
    /// Driver has accepted and is an active member of the fleet.
    Active,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetProfile {
    pub fleet_id: FleetId,
    pub owner: Address,
    pub treasury: Address,
    pub total_active_drivers: u32,
}

/// Persistent storage keys for the fleet management contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance key — stores the contract administrator address.
    Admin,
    /// Instance key — optional address of the identity_reputation_contract.
    IdentityContract,
    /// Persistent key — monotonically incrementing fleet counter.
    FleetCounter,
    /// Persistent key — fleet profile keyed by fleet id.
    Fleet(FleetId),
    /// Persistent key — driver's status within a fleet (Pending | Active).
    DriverFleet(FleetId, Address),
    /// Persistent key — roster of drivers (addresses) in a fleet, for enumeration.
    FleetRoster(FleetId),
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct FleetManagementContract;

#[contractimpl]
impl FleetManagementContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Initialise the contract, setting the admin and zeroing the fleet counter.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, FleetError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::FleetCounter, &0u64);
    }

    /// Configure the address of the identity_reputation_contract.  Admin only.
    /// Once set, `register_fleet` will automatically create an identity profile
    /// for the fleet owner via a cross-contract call.
    pub fn set_identity_contract(env: Env, admin: Address, identity_contract: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FleetError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::IdentityContract, &identity_contract);
    }

    // ── Issue #67 — register_fleet ────────────────────────────────────────────

    /// Register a new fleet, designating an owner and a treasury wallet.
    ///
    /// The caller (owner) must sign the transaction.  Returns the new fleet id.
    /// If an identity contract is configured, automatically creates an identity
    /// profile for the owner via a cross-contract call.
    pub fn register_fleet(env: Env, owner: Address, treasury: Address) -> FleetId {
        owner.require_auth();

        // Bump and persist the fleet counter.
        let counter_key = DataKey::FleetCounter;
        let current: u64 = env
            .storage()
            .persistent()
            .get(&counter_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::NotInitialized));

        let fleet_id: FleetId = current + 1;
        env.storage().persistent().set(&counter_key, &fleet_id);

        // Build and store the fleet profile.
        let profile = FleetProfile {
            fleet_id,
            owner: owner.clone(),
            treasury: treasury.clone(),
            total_active_drivers: 0,
        };

        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage()
            .persistent()
            .extend_ttl(&fleet_key, 518400, 518400);

        // Issue #73 — if identity contract is configured, register the fleet
        // owner as a driver in the identity_reputation_contract.
        if let Some(identity_addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::IdentityContract)
        {
            let _: () = env.invoke_contract(
                &identity_addr,
                &Symbol::new(&env, "register_driver"),
                soroban_sdk::vec![&env, owner.clone().into_val(&env)],
            );
        }

        // Emit event: topic = "fleet_registered", data = (fleet_id, owner, treasury).
        env.events().publish(
            (Symbol::new(&env, "fleet_registered"),),
            (fleet_id, owner, treasury),
        );

        fleet_id
    }

    /// Return the stored profile for a fleet.  Panics with `FleetNotFound` when
    /// no fleet with that id exists.
    pub fn get_fleet(env: Env, fleet_id: FleetId) -> FleetProfile {
        env.storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound))
    }

    /// Update the treasury wallet for an existing fleet.
    pub fn update_fleet_treasury(env: Env, owner: Address, fleet_id: FleetId, treasury: Address) {
        owner.require_auth();

        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        if profile.owner != owner {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        profile.treasury = treasury.clone();

        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage()
            .persistent()
            .extend_ttl(&fleet_key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "fleet_treasury_updated"),),
            (fleet_id, owner, treasury),
        );
    }

    // ── Issue #68 — add_driver_to_fleet ───────────────────────────────────────

    /// Invite a driver to a fleet.  Only the fleet owner may call this.
    ///
    /// `caller` must be the registered fleet owner and must sign the
    /// transaction.  Stores a `Pending` invite for `driver` under this fleet.
    /// The driver must later call `accept_fleet_invite` to become active.
    pub fn add_driver_to_fleet(env: Env, caller: Address, fleet_id: FleetId, driver: Address) {
        caller.require_auth();

        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        if profile.owner != caller {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());

        // Guard: do not overwrite an existing invite or active membership.
        if env.storage().persistent().has(&invite_key) {
            let existing: DriverFleetStatus = env.storage().persistent().get(&invite_key).unwrap();
            match existing {
                DriverFleetStatus::Pending => {
                    panic_with_error!(&env, FleetError::DriverAlreadyInvited)
                }
                DriverFleetStatus::Active => {
                    panic_with_error!(&env, FleetError::DriverAlreadyActive)
                }
            }
        }

        // Record the pending invite.
        env.storage()
            .persistent()
            .set(&invite_key, &DriverFleetStatus::Pending);
        env.storage()
            .persistent()
            .extend_ttl(&invite_key, 518400, 518400);

        // Emit event.
        env.events()
            .publish((Symbol::new(&env, "driver_invited"),), (fleet_id, driver));
    }

    // ── Issue #69 — accept_fleet_invite ───────────────────────────────────────

    /// Accept a pending fleet invite.  The driver themselves must sign this
    /// transaction.  Transitions status from `Pending` → `Active` and
    /// increments `total_active_drivers` on the fleet profile.
    pub fn accept_fleet_invite(env: Env, fleet_id: FleetId, driver: Address) {
        // Driver must authorise.
        driver.require_auth();

        // Verify the fleet exists.
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());

        // Verify there is a pending invite.
        let status: DriverFleetStatus = env
            .storage()
            .persistent()
            .get(&invite_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::InviteNotFound));

        match status {
            DriverFleetStatus::Active => panic_with_error!(&env, FleetError::DriverAlreadyActive),
            DriverFleetStatus::Pending => {}
        }

        // Promote driver to active.
        env.storage()
            .persistent()
            .set(&invite_key, &DriverFleetStatus::Active);
        env.storage()
            .persistent()
            .extend_ttl(&invite_key, 518400, 518400);

        // Update active driver count on the fleet profile.
        profile.total_active_drivers += 1;
        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage()
            .persistent()
            .extend_ttl(&fleet_key, 518400, 518400);

        // Add driver to fleet roster for enumeration.
        let roster_key = DataKey::FleetRoster(fleet_id);
        let mut roster: soroban_sdk::Vec<Address> = env
            .storage()
            .persistent()
            .get(&roster_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));

        // Guard against unbounded roster growth.
        if roster.len() >= MAX_ROSTER_SIZE as u32 {
            panic_with_error!(&env, FleetError::FleetNotFound);
        }

        // Avoid duplicate roster entries: check if driver already in roster.
        let mut already_in_roster = false;
        for i in 0..roster.len() {
            if let Some(existing) = roster.get(i) {
                if existing == driver {
                    already_in_roster = true;
                    break;
                }
            }
        }

        if !already_in_roster {
            roster.push_back(driver.clone());
            env.storage().persistent().set(&roster_key, &roster);
            env.storage()
                .persistent()
                .extend_ttl(&roster_key, 518400, 518400);
        }

        // Emit event.
        env.events()
            .publish((Symbol::new(&env, "invite_accepted"),), (fleet_id, driver));
    }

    // ── Issue #70 — remove_driver_from_fleet ──────────────────────────────────

    /// Remove a driver from a fleet.  Either the fleet owner or the driver
    /// themselves may call this function (bilateral severance).
    ///
    /// `caller` must be either the fleet owner or the driver being removed.
    /// Deletes the driver's fleet record and, if the driver was `Active`,
    /// decrements `total_active_drivers` on the fleet profile.
    pub fn remove_driver_from_fleet(env: Env, fleet_id: FleetId, caller: Address, driver: Address) {
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        // Verify caller is authorised: must be either the fleet owner or the driver.
        let is_owner = caller == profile.owner;
        let is_driver = caller == driver;
        if !is_owner && !is_driver {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        // The caller must sign this transaction.
        caller.require_auth();

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());

        let status: DriverFleetStatus = env
            .storage()
            .persistent()
            .get(&invite_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::InviteNotFound));

        // Decrement active driver count only when the driver was active.
        if status == DriverFleetStatus::Active && profile.total_active_drivers > 0 {
            profile.total_active_drivers -= 1;
            let fleet_key = DataKey::Fleet(fleet_id);
            env.storage().persistent().set(&fleet_key, &profile);
        }

        // Remove the driver's fleet record.
        env.storage().persistent().remove(&invite_key);

        // Remove driver from fleet roster.
        let roster_key = DataKey::FleetRoster(fleet_id);
        if let Some(mut roster) = env.storage().persistent().get::<_, soroban_sdk::Vec<Address>>(&roster_key) {
            let mut new_roster = soroban_sdk::Vec::new(&env);
            for i in 0..roster.len() {
                if let Some(existing) = roster.get(i) {
                    if existing != driver {
                        new_roster.push_back(existing);
                    }
                }
            }
            if new_roster.len() > 0 {
                env.storage().persistent().set(&roster_key, &new_roster);
                env.storage()
                    .persistent()
                    .extend_ttl(&roster_key, 518400, 518400);
            } else {
                env.storage().persistent().remove(&roster_key);
            }
        }

        // Emit event.
        env.events()
            .publish((Symbol::new(&env, "driver_removed"),), (fleet_id, driver));
    }

    // ── Issue #72 — get_payout_address ───────────────────────────────────────

    /// Return the address that the escrow_contract should route funds to for a
    /// given driver and fleet.
    ///
    /// Returns the fleet's treasury if the driver is an active member of that
    /// fleet, otherwise returns the driver's own address.
    pub fn get_payout_address(env: Env, driver: Address, fleet_id: FleetId) -> Address {
        let status: Option<DriverFleetStatus> = env
            .storage()
            .persistent()
            .get(&DataKey::DriverFleet(fleet_id, driver.clone()));

        match status {
            Some(DriverFleetStatus::Active) => {
                let profile: FleetProfile = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Fleet(fleet_id))
                    .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));
                profile.treasury
            }
            _ => driver,
        }
    }

    /// Return the status of a driver within a fleet, or `None` if no record
    /// exists.  Useful for off-chain queries and integration tests.
    pub fn get_driver_fleet_status(
        env: Env,
        fleet_id: FleetId,
        driver: Address,
    ) -> Option<DriverFleetStatus> {
        env.storage()
            .persistent()
            .get(&DataKey::DriverFleet(fleet_id, driver))
    }

    /// Return the roster of all drivers (both Pending and Active) for a fleet.
    /// Returns an empty Vec if no drivers are in the fleet.
    pub fn get_fleet_roster(env: Env, fleet_id: FleetId) -> soroban_sdk::Vec<Address> {
        let roster_key = DataKey::FleetRoster(fleet_id);
        env.storage()
            .persistent()
            .get(&roster_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }
}

#[cfg(test)]
mod test;

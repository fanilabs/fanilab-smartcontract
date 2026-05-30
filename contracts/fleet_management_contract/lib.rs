#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Symbol,
};

// ── Types ─────────────────────────────────────────────────────────────────────

pub type FleetId = u64;

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
    /// Persistent key — monotonically incrementing fleet counter.
    FleetCounter,
    /// Persistent key — fleet profile keyed by fleet id.
    Fleet(FleetId),
    /// Persistent key — driver's status within a fleet (Pending | Active).
    DriverFleet(FleetId, Address),
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

    // ── Issue #67 — register_fleet ────────────────────────────────────────────

    /// Register a new fleet, designating an owner and a treasury wallet.
    ///
    /// The caller (owner) must sign the transaction.  Returns the new fleet id.
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
    /// Stores a `Pending` invite for `driver` under this fleet.  The driver
    /// must later call `accept_fleet_invite` to become active.
    pub fn add_driver_to_fleet(env: Env, fleet_id: FleetId, driver: Address) {
        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        // Require fleet-owner authorisation.
        profile.owner.require_auth();

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());

        // Guard: do not overwrite an existing invite or active membership.
        if env.storage().persistent().has(&invite_key) {
            let existing: DriverFleetStatus = env
                .storage()
                .persistent()
                .get(&invite_key)
                .unwrap();
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
        env.events().publish(
            (Symbol::new(&env, "driver_invited"),),
            (fleet_id, driver),
        );
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

        // Emit event.
        env.events().publish(
            (Symbol::new(&env, "invite_accepted"),),
            (fleet_id, driver),
        );
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

        // Emit event.
        env.events().publish(
            (Symbol::new(&env, "driver_removed"),),
            (fleet_id, driver),
        );
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
}

#[cfg(test)]
mod test;

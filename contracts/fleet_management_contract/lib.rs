#![no_std]

use shared_types::{
    events, is_admin, ttl, DriverInvitedEvent, DriverRemovedEvent, FleetDeactivatedEvent,
    FleetOwnerReassignedEvent, FleetRegisteredEvent, FleetTreasuryChangeProposedEvent,
    FleetTreasuryForceUpdatedEvent, FleetTreasuryUpdatedEvent, InviteAcceptedEvent,
    PayoutRoutingFallbackEvent, StorageKey,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, IntoVal,
    Symbol,
};

// ── Types ─────────────────────────────────────────────────────────────────────

pub type FleetId = u64;

/// Maximum number of drivers per fleet roster to prevent unbounded storage growth.
pub const MAX_ROSTER_SIZE: u32 = 10000;

/// Minimum delay between proposing a fleet treasury change and it becoming
/// eligible for confirmation, giving active drivers advance notice before
/// their future payouts are redirected (Issue #70).
pub const TREASURY_CHANGE_TIMELOCK_SECONDS: u64 = 3 * 24 * 60 * 60; // 3 days

fn require_escrow_not_paused(env: &Env) {
    let Some(escrow_contract) = env
        .storage()
        .instance()
        .get::<_, Address>(&DataKey::EscrowContract)
    else {
        return;
    };
    let paused: bool = env.invoke_contract(
        &escrow_contract,
        &Symbol::new(env, "is_paused"),
        soroban_sdk::vec![env],
    );
    if paused {
        panic_with_error!(env, shared_types::FaniLabError::ProtocolPaused);
    }
}

/// Require that `caller` is one of the fleet's configured signers and that the
/// fleet's `signature_threshold` is satisfiable by that single authorization.
/// A caller who is not a signer, or a threshold that a lone signer cannot meet,
/// is rejected with `FleetError::Unauthorized`.
fn require_signer_threshold(env: &Env, profile: &FleetProfile, caller: &Address) {
    let mut authorized_signer_count = 0u32;
    for i in 0..profile.signers.len() {
        if let Some(signer) = profile.signers.get(i) {
            if signer == *caller {
                authorized_signer_count += 1;
                break;
            }
        }
    }

    if authorized_signer_count < profile.signature_threshold {
        panic_with_error!(env, FleetError::Unauthorized);
    }
}

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
    NoPendingTreasuryChange = 8,
    TimelockNotElapsed = 9,
    FleetInactive = 10,
    InvalidConfiguration = 11,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriverFleetStatus {
    /// Driver has been invited but has not yet accepted.
    Pending,
    /// Driver has accepted and is an active member of the fleet.
    Active,
    /// Driver has been removed from the fleet (terminal state, historical record preserved).
    Removed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetProfile {
    pub fleet_id: FleetId,
    pub owner: Address,
    pub treasury: Address,
    pub total_active_drivers: u32,
    pub signers: soroban_sdk::Vec<Address>,
    pub signature_threshold: u32,
    /// Whether the fleet is currently operating. Set to `false` by
    /// `deactivate_fleet` (Issue #108); a deactivated fleet rejects new
    /// driver invitations and is not selected for new driver payouts. Payout
    /// destinations for existing escrows are fixed when those escrows are
    /// created.
    pub active: bool,
}

/// A treasury change proposed by the fleet owner but not yet confirmed.
/// Becomes eligible for confirmation once `activates_at` has elapsed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTreasuryChange {
    pub treasury: Address,
    pub activates_at: u64,
}

/// Persistent storage keys for the fleet management contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance key — optional address of the identity_reputation_contract.
    IdentityContract,
    /// Instance key — address of the escrow contract used for pause checks.
    EscrowContract,
    /// Persistent key — monotonically incrementing fleet counter.
    FleetCounter,
    /// Persistent key — fleet profile keyed by fleet id.
    Fleet(FleetId),
    /// Persistent key — driver's status within a fleet (Pending | Active).
    DriverFleet(FleetId, Address),
    /// Persistent key — one active roster entry, keyed by fleet and index.
    FleetRoster(FleetId, u32),
    /// Persistent key — pending, not-yet-confirmed treasury change for a fleet.
    PendingTreasury(FleetId),
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct FleetManagementContract;

#[contractimpl]
impl FleetManagementContract {
    // ── Initialisation ────────────────────────────────────────────────────────

    /// Initialise the contract, setting the admin and zeroing the fleet counter.
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FleetError::AlreadyInitialized);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::FleetCounter, &0u64);
    }

    /// Configure the address of the identity_reputation_contract.  Admin only.
    /// Once set, `register_fleet` will automatically create an identity profile
    /// for the fleet owner via a cross-contract call.
    pub fn set_identity_contract(env: Env, admin: Address, identity_contract: Address) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FleetError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::IdentityContract, &identity_contract);
    }

    pub fn set_escrow_contract(env: Env, admin: Address, escrow_contract: Address) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FleetError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
    }

    // ── Issue #67 — register_fleet ────────────────────────────────────────────

    /// Register a new fleet, designating an owner and a treasury wallet.
    ///
    /// The caller (owner) must sign the transaction.  Returns the new fleet id.
    /// If an identity contract is configured, automatically creates an identity
    /// profile for the owner via a cross-contract call.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn register_fleet(env: Env, owner: Address, treasury: Address) -> FleetId {
        owner.require_auth();
        require_escrow_not_paused(&env);

        // Bump and persist the fleet counter.
        let counter_key = DataKey::FleetCounter;
        let current: u64 = env
            .storage()
            .persistent()
            .get(&counter_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::NotInitialized));

        let fleet_id: FleetId = current + 1;
        env.storage().persistent().set(&counter_key, &fleet_id);

        // Build and store the fleet profile with single-owner multi-sig (backward compatible).
        let mut signers = soroban_sdk::Vec::new(&env);
        signers.push_back(owner.clone());

        let profile = FleetProfile {
            fleet_id,
            owner: owner.clone(),
            treasury: treasury.clone(),
            total_active_drivers: 0,
            signers,
            signature_threshold: 1u32,
            active: true,
        };

        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Issue #73 — if identity contract is configured, register the fleet
        // owner as a driver in the identity_reputation_contract.
        if let Some(identity_addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::IdentityContract)
        {
            let has_profile: bool = env.invoke_contract(
                &identity_addr,
                &Symbol::new(&env, "has_driver_profile"),
                soroban_sdk::vec![&env, owner.clone().into_val(&env)],
            );
            if !has_profile {
                let _: () = env.invoke_contract(
                    &identity_addr,
                    &Symbol::new(&env, "register_driver"),
                    soroban_sdk::vec![&env, owner.clone().into_val(&env)],
                );
            }
        }

        // Emit event: topic = "fleet_registered", data = (fleet_id, owner, treasury).
        env.events().publish(
            (events::fleet_registered(&env),),
            FleetRegisteredEvent {
                fleet_id,
                owner,
                treasury,
            },
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

    // ── Issue #108 — deactivate_fleet ─────────────────────────────────────────

    /// Deactivate a fleet, marking it inactive (terminal, closable lifecycle
    /// step). Callable by the fleet owner or the contract admin.
    ///
    /// Once deactivated, `add_driver_to_fleet` rejects new invitations and
    /// `get_payout_address` falls back to routing payouts to the driver's own
    /// address instead of the fleet treasury. Existing drivers are left in
    /// place rather than auto-removed; they may be individually removed via
    /// `remove_driver_from_fleet` if desired.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn deactivate_fleet(env: Env, caller: Address, fleet_id: FleetId) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let fleet_key = DataKey::Fleet(fleet_id);
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&fleet_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        if profile.owner != caller && !is_admin(&env, &caller) {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        profile.active = false;
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::fleet_deactivated(&env),),
            FleetDeactivatedEvent { fleet_id, caller },
        );
    }

    // ── Issue #69 — admin override / recovery ─────────────────────────────────

    /// Reassign the owner of a fleet without the current owner's cooperation.
    ///
    /// This is an emergency recovery path for when a fleet owner's key is
    /// lost or compromised.  Only the contract admin may call this.  The
    /// function emits a dedicated `fleet_owner_reassigned` event so that the
    /// change is auditable on-chain and distinguishable from normal owner
    /// transfers (which do not yet exist).
    ///
    /// Side effects:
    /// - `profile.owner` is updated to `new_owner`.
    /// - `profile.signers` is reset to `[new_owner]` with threshold 1, so the
    ///   new owner has immediate unilateral control.  If the fleet used a
    ///   multi-sig configuration the admin (or the new owner) can restore it
    ///   via `configure_signers` after recovery.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn admin_reassign_fleet_owner(
        env: Env,
        admin: Address,
        fleet_id: FleetId,
        new_owner: Address,
    ) {
        admin.require_auth();

        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        let fleet_key = DataKey::Fleet(fleet_id);
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&fleet_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        let old_owner = profile.owner.clone();

        profile.owner = new_owner.clone();
        // Reset signers to the new owner with threshold 1 so they have
        // immediate unilateral control; the multi-sig config can be
        // re-established via configure_signers once the situation is resolved.
        let mut new_signers = soroban_sdk::Vec::new(&env);
        new_signers.push_back(new_owner.clone());
        profile.signers = new_signers;
        profile.signature_threshold = 1;

        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::fleet_owner_reassigned(&env),),
            FleetOwnerReassignedEvent {
                fleet_id,
                admin,
                old_owner,
                new_owner,
            },
        );
    }

    /// Force-update a fleet's treasury address without the fleet owner's
    /// cooperation and without waiting for a timelock to expire.
    ///
    /// This is an emergency path for when a fleet treasury key is compromised
    /// and active driver payouts need to be redirected immediately.  Only the
    /// contract admin may call this.  The function emits a dedicated
    /// `fleet_treasury_force_updated` event, distinguishable from both
    /// owner-initiated proposals (`fleet_treasury_change_proposed`) and
    /// owner-initiated confirmations (`fleet_treasury_updated`), so that
    /// off-chain monitors can distinguish emergency admin overrides from normal
    /// treasury updates.
    ///
    /// If there is a pending owner-initiated treasury change in progress it is
    /// cleared, preventing it from overwriting the admin-set address after
    /// the emergency is resolved.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn admin_force_update_treasury(
        env: Env,
        admin: Address,
        fleet_id: FleetId,
        new_treasury: Address,
    ) {
        admin.require_auth();

        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        let fleet_key = DataKey::Fleet(fleet_id);
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&fleet_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        let old_treasury = profile.treasury.clone();
        profile.treasury = new_treasury.clone();

        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Discard any in-progress owner-initiated pending change to prevent
        // it from overwriting this emergency update once its timelock expires.
        let pending_key = DataKey::PendingTreasury(fleet_id);
        if env.storage().persistent().has(&pending_key) {
            env.storage().persistent().remove(&pending_key);
        }

        env.events().publish(
            (events::fleet_treasury_force_updated(&env),),
            FleetTreasuryForceUpdatedEvent {
                fleet_id,
                admin,
                old_treasury,
                new_treasury,
            },
        );
    }

    // ── Issue #70 — treasury change timelock ──────────────────────────────────

    /// Propose a new treasury wallet for an existing fleet.  Only the fleet
    /// owner may call this.  The change does not take effect immediately:
    /// it becomes eligible for confirmation only after
    /// `TREASURY_CHANGE_TIMELOCK_SECONDS` have elapsed, giving active drivers
    /// advance notice (via the `fleet_treasury_change_proposed` event) before
    /// their future payouts are redirected. Proposing again before
    /// confirmation overwrites the pending change and restarts the timelock.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn update_fleet_treasury(env: Env, owner: Address, fleet_id: FleetId, treasury: Address) {
        owner.require_auth();
        require_escrow_not_paused(&env);

        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        require_signer_threshold(&env, &profile, &owner);

        let activates_at = env
            .ledger()
            .timestamp()
            .saturating_add(TREASURY_CHANGE_TIMELOCK_SECONDS);

        let pending_key = DataKey::PendingTreasury(fleet_id);
        let pending = PendingTreasuryChange {
            treasury: treasury.clone(),
            activates_at,
        };
        env.storage().persistent().set(&pending_key, &pending);
        env.storage().persistent().extend_ttl(
            &pending_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::fleet_treasury_change_proposed(&env),),
            FleetTreasuryChangeProposedEvent {
                fleet_id,
                owner,
                current_treasury: profile.treasury,
                proposed_treasury: treasury,
                activates_at,
            },
        );
    }

    /// Confirm a previously proposed treasury change once its timelock has
    /// elapsed, applying it to the fleet profile used by `get_payout_address`.
    /// Callable by anyone: the security guarantee is the elapsed delay, not
    /// caller identity, matching `reclaim_expired_escrow`'s permissionless
    /// finalization pattern.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn confirm_fleet_treasury_update(env: Env, fleet_id: FleetId) {
        require_escrow_not_paused(&env);
        let pending_key = DataKey::PendingTreasury(fleet_id);
        let pending: PendingTreasuryChange = env
            .storage()
            .persistent()
            .get(&pending_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::NoPendingTreasuryChange));

        if env.ledger().timestamp() < pending.activates_at {
            panic_with_error!(&env, FleetError::TimelockNotElapsed);
        }

        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        profile.treasury = pending.treasury.clone();

        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );
        env.storage().persistent().remove(&pending_key);

        env.events().publish(
            (events::fleet_treasury_updated(&env),),
            FleetTreasuryUpdatedEvent {
                fleet_id,
                owner: profile.owner,
                treasury: pending.treasury,
            },
        );
    }

    /// Return the pending treasury change for a fleet, if any, so off-chain
    /// clients (e.g. driver apps) can display the upcoming payout redirect
    /// and its activation time.
    pub fn get_pending_treasury_update(
        env: Env,
        fleet_id: FleetId,
    ) -> Option<PendingTreasuryChange> {
        env.storage()
            .persistent()
            .get(&DataKey::PendingTreasury(fleet_id))
    }

    // ── Issue #68 — add_driver_to_fleet ───────────────────────────────────────

    /// Invite a driver to a fleet.  Only an authorized signer may call this.
    ///
    /// `caller` must be an authorized signer and must sign the transaction.
    /// Stores a `Pending` invite for `driver` under this fleet.
    /// The driver must later call `accept_fleet_invite` to become active.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn add_driver_to_fleet(env: Env, caller: Address, fleet_id: FleetId, driver: Address) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        if !profile.active {
            panic_with_error!(&env, FleetError::FleetInactive);
        }

        require_signer_threshold(&env, &profile, &caller);

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());

        // Guard: do not overwrite an existing invite or active membership.
        // Removed drivers may be re-invited.
        if env.storage().persistent().has(&invite_key) {
            let existing: DriverFleetStatus = env.storage().persistent().get(&invite_key).unwrap();
            match existing {
                DriverFleetStatus::Pending => {
                    panic_with_error!(&env, FleetError::DriverAlreadyInvited)
                }
                DriverFleetStatus::Active => {
                    panic_with_error!(&env, FleetError::DriverAlreadyActive)
                }
                DriverFleetStatus::Removed => {
                    // Allow re-inviting a previously removed driver.
                }
            }
        }

        // Record the pending invite.
        env.storage()
            .persistent()
            .set(&invite_key, &DriverFleetStatus::Pending);
        env.storage().persistent().extend_ttl(
            &invite_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Emit event.
        env.events().publish(
            (events::driver_invited(&env),),
            DriverInvitedEvent { fleet_id, driver },
        );
    }

    // ── Issue #109 — cancel_invite ─────────────────────────────────────────────

    /// Cancel a driver's pending invite before it has been accepted.  Only an
    /// authorized signer may call this — the same authority required to issue
    /// the invite via `add_driver_to_fleet`.
    ///
    /// Unlike `remove_driver_from_fleet` (bilateral severance of an already
    /// active relationship), this withdraws an invite the driver never
    /// accepted, clearing the slot so the driver can be re-invited immediately.
    pub fn cancel_invite(env: Env, owner: Address, fleet_id: FleetId, driver: Address) {
        owner.require_auth();
        require_escrow_not_paused(&env);

        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        require_signer_threshold(&env, &profile, &owner);

        let invite_key = DataKey::DriverFleet(fleet_id, driver.clone());
        let status: DriverFleetStatus = env
            .storage()
            .persistent()
            .get(&invite_key)
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::InviteNotFound));

        if status != DriverFleetStatus::Pending {
            panic_with_error!(&env, FleetError::DriverAlreadyActive);
        }

        env.storage().persistent().remove(&invite_key);
    }

    // ── Issue #69 — accept_fleet_invite ───────────────────────────────────────

    /// Accept a pending fleet invite.  The driver themselves must sign this
    /// transaction.  Transitions status from `Pending` → `Active` and
    /// increments `total_active_drivers` on the fleet profile.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn accept_fleet_invite(env: Env, fleet_id: FleetId, driver: Address) {
        // Driver must authorise.
        driver.require_auth();
        require_escrow_not_paused(&env);

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
            DriverFleetStatus::Removed => panic_with_error!(&env, FleetError::InviteNotFound),
        }

        // Guard against unbounded roster growth before changing membership state.
        if profile.total_active_drivers >= MAX_ROSTER_SIZE {
            panic_with_error!(&env, FleetError::FleetNotFound);
        }

        // Promote driver to active.
        env.storage()
            .persistent()
            .set(&invite_key, &DriverFleetStatus::Active);
        env.storage().persistent().extend_ttl(
            &invite_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Update active driver count on the fleet profile.
        profile.total_active_drivers += 1;
        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Add the driver as one indexed roster entry. The previous active
        // count is the next free roster index.
        let roster_key = DataKey::FleetRoster(fleet_id, profile.total_active_drivers - 1);
        env.storage().persistent().set(&roster_key, &driver);
        env.storage().persistent().extend_ttl(
            &roster_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Emit event.
        env.events().publish(
            (events::invite_accepted(&env),),
            InviteAcceptedEvent { fleet_id, driver },
        );
    }

    // ── Issue #70 — remove_driver_from_fleet ──────────────────────────────────

    /// Remove a driver from a fleet.  Either a fleet signer or the driver
    /// themselves may call this function (bilateral severance).
    ///
    /// `caller` must be either an authorized signer or the driver being removed.
    /// Deletes the driver's fleet record and, if the driver was `Active`,
    /// decrements `total_active_drivers` on the fleet profile.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn remove_driver_from_fleet(env: Env, fleet_id: FleetId, caller: Address, driver: Address) {
        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        // The caller must sign this transaction.
        caller.require_auth();
        require_escrow_not_paused(&env);

        // Verify caller is authorised: must be either an authorized signer or the driver.
        let is_driver = caller == driver;
        if !is_driver {
            require_signer_threshold(&env, &profile, &caller);
        }

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

        // Transition to Removed terminal state instead of deleting, preserving historical record.
        env.storage()
            .persistent()
            .set(&invite_key, &DriverFleetStatus::Removed);
        env.storage().persistent().extend_ttl(
            &invite_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Remove the driver from the indexed roster and compact the remaining
        // entries so enumeration stays contiguous without rewriting one large value.
        if status == DriverFleetStatus::Active {
            let roster_len = profile.total_active_drivers + 1;
            let mut removed_index = None;
            for index in 0..roster_len {
                let roster_key = DataKey::FleetRoster(fleet_id, index);
                if env
                    .storage()
                    .persistent()
                    .get::<_, Address>(&roster_key)
                    .is_some_and(|existing| existing == driver)
                {
                    removed_index = Some(index);
                    break;
                }
            }

            if let Some(index) = removed_index {
                for next_index in index..(roster_len - 1) {
                    let next_key = DataKey::FleetRoster(fleet_id, next_index + 1);
                    let current_key = DataKey::FleetRoster(fleet_id, next_index);
                    let next_driver: Address = env
                        .storage()
                        .persistent()
                        .get(&next_key)
                        .unwrap();
                    env.storage().persistent().set(&current_key, &next_driver);
                    env.storage().persistent().extend_ttl(
                        &current_key,
                        ttl::LEDGER_TTL_THRESHOLD,
                        ttl::LEDGER_TTL_EXTEND_TO,
                    );
                }
                env.storage()
                    .persistent()
                    .remove(&DataKey::FleetRoster(fleet_id, roster_len - 1));
            }
        }

        // Emit event.
        env.events().publish(
            (events::driver_removed(&env),),
            DriverRemovedEvent { fleet_id, driver },
        );
    }

    // ── Issue #72 — get_payout_address ───────────────────────────────────────

    /// Return the address that the escrow_contract should route funds to for a
    /// given driver and fleet.
    ///
    /// Returns the fleet's treasury if the driver is an active member of that
    /// fleet, otherwise returns the driver's own address.
    #[allow(deprecated)]
    pub fn get_payout_address(env: Env, driver: Address, fleet_id: FleetId) -> Address {
        let status: Option<DriverFleetStatus> = env
            .storage()
            .persistent()
            .get(&DataKey::DriverFleet(fleet_id, driver.clone()));

        match status {
            Some(DriverFleetStatus::Active) => {
                let profile: Option<FleetProfile> = env
                    .storage()
                    .persistent()
                    .get(&DataKey::Fleet(fleet_id));
                match profile {
                    Some(profile) if profile.active => profile.treasury,
                    Some(_) => driver,
                    None => {
                        env.events().publish(
                            (events::payout_routing_fallback(&env),),
                            PayoutRoutingFallbackEvent {
                                fleet_id,
                                driver: driver.clone(),
                            },
                        );
                        driver
                    }
                }
            }
            Some(DriverFleetStatus::Pending) | Some(DriverFleetStatus::Removed) | None => driver,
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

    /// Return the roster of all active drivers for a fleet.
    /// Returns an empty Vec if no drivers are active in the fleet.
    pub fn get_fleet_roster(env: Env, fleet_id: FleetId) -> soroban_sdk::Vec<Address> {
        let mut roster = soroban_sdk::Vec::new(&env);
        let active_count = env
            .storage()
            .persistent()
            .get::<_, FleetProfile>(&DataKey::Fleet(fleet_id))
            .map(|profile| profile.total_active_drivers)
            .unwrap_or(0);

        for index in 0..active_count {
            if let Some(driver) = env
                .storage()
                .persistent()
                .get::<_, Address>(&DataKey::FleetRoster(fleet_id, index))
            {
                roster.push_back(driver);
            }
        }
        roster
    }

    /// Configure multi-signature requirements for a fleet.
    /// Only the fleet owner may call this. Sets the authorized signers and
    /// signature threshold for treasury and driver removal actions.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn configure_signers(
        env: Env,
        owner: Address,
        fleet_id: FleetId,
        signers: soroban_sdk::Vec<Address>,
        threshold: u32,
    ) {
        owner.require_auth();
        require_escrow_not_paused(&env);

        let mut profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        if profile.owner != owner {
            panic_with_error!(&env, FleetError::Unauthorized);
        }

        if threshold == 0 || threshold > signers.len() {
            panic_with_error!(&env, FleetError::InvalidConfiguration);
        }

        profile.signers = signers;
        profile.signature_threshold = threshold;

        let fleet_key = DataKey::Fleet(fleet_id);
        env.storage().persistent().set(&fleet_key, &profile);
        env.storage().persistent().extend_ttl(
            &fleet_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (Symbol::new(&env, "signers_configured"),),
            (fleet_id, owner, threshold),
        );
    }

    /// Get the signers and threshold for a fleet.
    pub fn get_fleet_signers(env: Env, fleet_id: FleetId) -> (soroban_sdk::Vec<Address>, u32) {
        let profile: FleetProfile = env
            .storage()
            .persistent()
            .get(&DataKey::Fleet(fleet_id))
            .unwrap_or_else(|| panic_with_error!(&env, FleetError::FleetNotFound));

        (profile.signers, profile.signature_threshold)
    }
}

#[cfg(test)]
mod test;

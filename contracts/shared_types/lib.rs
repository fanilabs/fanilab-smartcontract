#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FaniLabError {
    /// Caller is not authorized to perform the requested action.
    Unauthorized = 1,
    /// Contract or protocol state has already been initialized.
    AlreadyInitialized = 2,
    /// Contract or protocol state has not been initialized yet.
    NotInitialized = 3,
    /// Delivery record or related escrow entry could not be found.
    DeliveryNotFound = 4,
    /// Requested operation is invalid for the current protocol state.
    InvalidState = 5,
    /// Contract balance is too low to complete the requested transfer.
    InsufficientFunds = 6,
    /// Delivery identifier already exists in protocol storage.
    DuplicateDelivery = 8,
    /// Provider or driver record could not be found.
    ProviderNotFound = 9,
    /// Protocol is paused and fund movements are halted.
    ProtocolPaused = 11,
    /// Requested operation would exceed a fixed capacity/growth limit
    /// (e.g. a bounded collection is already at its maximum length).
    LimitExceeded = 12,
}

// Event topic constants for on-chain event tracking
pub mod events {
    use soroban_sdk::{Env, Symbol};

    pub const DELIVERY_CREATED: &str = "delivery_created";
    pub const ESCROW_FUNDED: &str = "escrow_funded";
    pub const DRIVER_ASSIGNED: &str = "driver_assigned";
    pub const DELIVERY_CONFIRMED: &str = "delivery_confirmed";
    pub const ESCROW_RELEASED: &str = "escrow_released";
    pub const DELIVERY_DISPUTED: &str = "delivery_disputed";
    pub const ESCROW_REFUNDED: &str = "escrow_refunded";

    pub fn delivery_created(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_CREATED)
    }

    pub fn escrow_funded(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_FUNDED)
    }

    pub fn driver_assigned(env: &Env) -> Symbol {
        Symbol::new(env, DRIVER_ASSIGNED)
    }

    pub fn delivery_confirmed(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_CONFIRMED)
    }

    pub fn escrow_released(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_RELEASED)
    }

    pub fn escrow_refunded(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_REFUNDED)
    }

    pub fn delivery_disputed(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_DISPUTED)
    }

    pub fn dispute_resolved(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_resolved")
    }

    pub fn delivery_cancelled(env: &Env) -> Symbol {
        Symbol::new(env, "delivery_cancelled")
    }

    pub fn delivery_in_transit(env: &Env) -> Symbol {
        Symbol::new(env, "delivery_in_transit")
    }

    // Fleet management events
    pub fn fleet_registered(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_registered")
    }

    pub fn fleet_treasury_updated(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_treasury_updated")
    }

    pub fn fleet_treasury_change_proposed(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_treasury_change_proposed")
    }

    pub fn driver_invited(env: &Env) -> Symbol {
        Symbol::new(env, "driver_invited")
    }

    pub fn invite_accepted(env: &Env) -> Symbol {
        Symbol::new(env, "invite_accepted")
    }

    pub fn driver_removed(env: &Env) -> Symbol {
        Symbol::new(env, "driver_removed")
    }

    pub fn payout_routing_fallback(env: &Env) -> Symbol {
        Symbol::new(env, "payout_routing_fallback")
    }

    pub fn fleet_deactivated(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_deactivated")
    }

    /// Emitted when the contract admin reassigns a fleet's owner address
    /// (e.g. after the original owner key is lost or compromised).
    pub fn fleet_owner_reassigned(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_owner_reassigned")
    }

    /// Emitted when the contract admin force-updates a fleet's treasury
    /// address, bypassing the normal owner-initiated timelock flow.
    pub fn fleet_treasury_force_updated(env: &Env) -> Symbol {
        Symbol::new(env, "fleet_treasury_force_updated")
    }

    // Dispute resolution events
    pub fn dispute_raised(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_raised")
    }

    pub fn evidence_added(env: &Env) -> Symbol {
        Symbol::new(env, "evidence_added")
    }

    pub fn dispute_resolved_refund(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_resolved_refund")
    }

    pub fn dispute_resolved_split(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_resolved_split")
    }

    pub fn dispute_resolved_payout(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_resolved_payout")
    }

    // Identity and reputation events
    pub fn driver_registered(env: &Env) -> Symbol {
        Symbol::new(env, "driver_registered")
    }

    pub fn driver_suspended(env: &Env) -> Symbol {
        Symbol::new(env, "driver_suspended")
    }

    pub fn driver_reinstated(env: &Env) -> Symbol {
        Symbol::new(env, "driver_reinstated")
    }

    pub fn user_registered(env: &Env) -> Symbol {
        Symbol::new(env, "user_registered")
    }

    pub fn kyc_status_updated(env: &Env) -> Symbol {
        Symbol::new(env, "kyc_status_updated")
    }

    pub fn reputation_increased(env: &Env) -> Symbol {
        Symbol::new(env, "reputation_increased")
    }

    pub fn reputation_decreased(env: &Env) -> Symbol {
        Symbol::new(env, "reputation_decreased")
    }

    pub fn reputation_awarded(env: &Env) -> Symbol {
        Symbol::new(env, "reputation_awarded")
    }

    // Protocol/admin lifecycle events. These previously used raw inline
    // Symbol::new(&env, "PascalCase") calls at each contract's call site
    // instead of going through this module, the one place in the codebase
    // that mixed casing conventions for event topics (Issue #47).
    pub fn protocol_initialized(env: &Env) -> Symbol {
        Symbol::new(env, "protocol_initialized")
    }

    pub fn fee_updated(env: &Env) -> Symbol {
        Symbol::new(env, "fee_updated")
    }

    pub fn settlement_contract_proposed(env: &Env) -> Symbol {
        // Soroban Symbol values are capped at 32 bytes; the fuller
        // "settlement_contract_change_proposed" (35 bytes) exceeds that.
        Symbol::new(env, "settlement_contract_proposed")
    }

    pub fn settlement_contract_updated(env: &Env) -> Symbol {
        Symbol::new(env, "settlement_contract_updated")
    }

    pub fn admin_transferred(env: &Env) -> Symbol {
        Symbol::new(env, "admin_transferred")
    }

    pub fn protocol_pause_status_changed(env: &Env) -> Symbol {
        Symbol::new(env, "protocol_pause_status_changed")
    }

    pub fn delivery_contract_initialized(env: &Env) -> Symbol {
        Symbol::new(env, "delivery_contract_initialized")
    }
}

pub mod ttl {
    pub const LEDGER_TTL_THRESHOLD: u32 = 518400;
    pub const LEDGER_TTL_EXTEND_TO: u32 = 1036800;
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryCreatedEvent {
    /// Unique protocol delivery identifier created by the delivery contract.
    pub delivery_id: u64,
    /// Address that created and funds the delivery request.
    pub sender: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFundedEvent {
    /// Delivery identifier whose escrow was funded.
    pub delivery_id: u64,
    /// Address that transferred tokens into escrow.
    pub sender: Address,
    /// Token contract address used for the escrow balance.
    pub token: Address,
    /// Amount transferred into escrow.
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverAssignedEvent {
    /// Delivery identifier assigned to a driver.
    pub delivery_id: u64,
    /// Driver address assigned to complete the delivery.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryConfirmedEvent {
    /// Delivery identifier confirmed by the recipient.
    pub delivery_id: u64,
    /// Recipient address that confirmed completion.
    pub recipient: Address,
    /// Ledger timestamp when delivery completion was confirmed.
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowReleasedEvent {
    /// Delivery identifier whose escrow was released.
    pub delivery_id: u64,
    /// Driver address receiving released escrow funds.
    pub driver: Address,
    /// Amount released to the driver.
    pub amount: i128,
    /// Platform fee withheld during release.
    pub platform_fee: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryDisputedEvent {
    /// Delivery identifier moved into dispute handling.
    pub delivery_id: u64,
    /// Address that raised or recorded the dispute.
    pub reporter: Address,
    /// Ledger timestamp when the dispute was recorded.
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRefundedEvent {
    /// Delivery identifier whose escrow was refunded.
    pub delivery_id: u64,
    /// Original sender address receiving refunded funds.
    pub sender: Address,
    /// Amount returned to the sender.
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    /// Delivery identifier whose dispute was resolved.
    pub delivery_id: u64,
    /// Admin address that resolved the dispute.
    pub resolver: Address,
}

// ── Fleet management event payloads ──────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetRegisteredEvent {
    /// Newly assigned fleet identifier.
    pub fleet_id: u64,
    /// Address of the fleet owner.
    pub owner: Address,
    /// Treasury wallet address for payouts.
    pub treasury: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetTreasuryUpdatedEvent {
    /// Fleet identifier whose treasury was updated.
    pub fleet_id: u64,
    /// Fleet owner address that authorized the change.
    pub owner: Address,
    /// New treasury wallet address.
    pub treasury: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetTreasuryChangeProposedEvent {
    /// Fleet identifier whose treasury change was proposed.
    pub fleet_id: u64,
    /// Fleet owner address that proposed the change.
    pub owner: Address,
    /// Current (still-active) treasury wallet address.
    pub current_treasury: Address,
    /// Proposed new treasury wallet address.
    pub proposed_treasury: Address,
    /// Ledger timestamp after which the change may be confirmed.
    pub activates_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverInvitedEvent {
    /// Fleet the driver was invited to.
    pub fleet_id: u64,
    /// Driver address that received the invite.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InviteAcceptedEvent {
    /// Fleet the driver accepted membership in.
    pub fleet_id: u64,
    /// Driver address that accepted the invite.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverRemovedEvent {
    /// Fleet the driver was removed from.
    pub fleet_id: u64,
    /// Driver address that was removed.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRoutingFallbackEvent {
    /// Fleet whose missing profile caused the fallback.
    pub fleet_id: u64,
    /// Driver receiving the payout directly.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetDeactivatedEvent {
    /// Fleet identifier that was deactivated.
    pub fleet_id: u64,
    /// Address that authorized the deactivation (owner or admin).
    pub caller: Address,
}

/// Emitted by `admin_reassign_fleet_owner` — admin-initiated ownership
/// transfer when the original owner key is lost or compromised.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetOwnerReassignedEvent {
    /// Fleet whose ownership was reassigned.
    pub fleet_id: u64,
    /// Admin address that performed the reassignment.
    pub admin: Address,
    /// Previous owner address that was replaced.
    pub old_owner: Address,
    /// New owner address that was assigned.
    pub new_owner: Address,
}

/// Emitted by `admin_force_update_treasury` — admin-initiated treasury
/// override that bypasses the owner-initiated timelock.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetTreasuryForceUpdatedEvent {
    /// Fleet whose treasury was forcibly updated.
    pub fleet_id: u64,
    /// Admin address that performed the override.
    pub admin: Address,
    /// Previous treasury address.
    pub old_treasury: Address,
    /// New treasury address.
    pub new_treasury: Address,
}

// ── Dispute resolution event payloads ────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRaisedEvent {
    /// Delivery identifier the dispute was raised on.
    pub delivery_id: u64,
    /// Address that raised the dispute.
    pub caller: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedRefundEvent {
    /// Delivery identifier whose dispute was resolved with a refund.
    pub delivery_id: u64,
    /// Admin address that resolved the dispute.
    pub caller: Address,
    /// Driver address penalized by the resolution.
    pub driver: Address,
    /// Reputation penalty applied to the driver.
    pub penalty: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedSplitEvent {
    /// Delivery identifier whose dispute was resolved with a split.
    pub delivery_id: u64,
    /// Admin address that resolved the dispute.
    pub caller: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedPayoutEvent {
    /// Delivery identifier whose dispute was resolved with driver payout.
    pub delivery_id: u64,
    /// Admin address that resolved the dispute.
    pub caller: Address,
}

// ── Identity and reputation event payloads ───────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverRegisteredEvent {
    /// Driver address that was registered.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverSuspendedEvent {
    /// Driver address whose profile was suspended.
    pub driver: Address,
    /// Admin address that performed the suspension.
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverReinstatedEvent {
    /// Driver address whose profile was reinstated.
    pub driver: Address,
    /// Admin address that performed the reinstatement.
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserRegisteredEvent {
    /// User address that was registered.
    pub user: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KycStatusUpdatedEvent {
    /// Driver address whose KYC status was updated.
    pub driver: Address,
    /// New KYC verification state.
    pub kyc_verified: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationIncreasedEvent {
    /// Driver address whose reputation was increased.
    pub driver: Address,
    /// Delivery identifier that triggered the change.
    pub delivery_id: u64,
    /// Points added to the driver's reputation score.
    pub points: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationDecreasedEvent {
    /// Driver address whose reputation was decreased.
    pub driver: Address,
    /// Points deducted from the driver's reputation score.
    pub points: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReputationAwardedEvent {
    /// Driver address that received a flat reputation award.
    pub driver: Address,
    /// Points added to the driver's reputation score (post-cap value may be lower).
    pub points: u32,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct DeliveryId(pub u64);

impl DeliveryId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for DeliveryId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<DeliveryId> for u64 {
    fn from(value: DeliveryId) -> Self {
        value.0
    }
}

impl PartialEq<u64> for DeliveryId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<DeliveryId> for u64 {
    fn eq(&self, other: &DeliveryId) -> bool {
        *self == other.0
    }
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum DeliveryStatus {
    Pending,
    Active,
    InTransit,
    Delivered,
    Disputed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryRecord {
    pub delivery_id: DeliveryId,
    pub sender: Address,
    pub recipient: Address,
    pub driver: Option<Address>,
    pub status: DeliveryStatus,
    pub metadata: DeliveryMetadata,
    pub created_at: u64,
    pub delivered_at: Option<u64>,
    pub transit_started_at: Option<u64>,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum EscrowState {
    Locked,
    Holdback,
    Released,
    Refunded,
    Paused,
    Split,
}

pub type EscrowStatus = EscrowState;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub token: Address,
    pub platform_fee_bps: u32,
    pub protocol_version: u32,
    pub slippage_tolerance_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Instance storage for the shared admin address.
    Admin,
    /// Persistent storage for a delivery record.
    Delivery(DeliveryId),
    /// Persistent storage for an escrow record.
    Escrow(DeliveryId),
    /// Persistent storage for a driver profile.
    DriverProfile(Address),
    /// Instance storage for protocol-wide configuration.
    ProtocolConfig,
}

pub fn delivery_key(id: impl Into<DeliveryId>) -> StorageKey {
    StorageKey::Delivery(id.into())
}

pub fn escrow_key(id: impl Into<DeliveryId>) -> StorageKey {
    StorageKey::Escrow(id.into())
}

/// Returns `true` when `caller` matches the admin stored under
/// `StorageKey::Admin` in **instance** storage.
///
/// Returns `false` — rather than panicking — when the contract has not yet
/// been initialised.  This is intentional: `escrow_contract`,
/// `delivery_contract`, `fleet_management_contract`, and
/// `identity_reputation_contract` all share this single source-of-truth so
/// the pre-init behaviour is always consistent (ADR-003).
///
/// `dispute_resolution_contract` is the one exception: it supports multiple
/// simultaneous admins with a last-admin-removal guard, a genuinely
/// different governance model this single-admin helper cannot express, so
/// it keeps its own `DataKey::Admin(Address)` + `DataKey::AdminList`
/// implementation rather than being forced onto this function (Issue #77).
pub fn is_admin(env: &soroban_sdk::Env, caller: &soroban_sdk::Address) -> bool {
    if let Some(admin) = env
        .storage()
        .instance()
        .get::<_, soroban_sdk::Address>(&StorageKey::Admin)
    {
        admin == *caller
    } else {
        false
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRecord {
    pub delivery_id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub driver: Address,
    pub token: Address,
    pub amount: i128,
    pub status: EscrowState,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub disputed_by: Option<Address>,
    pub disputed_at: Option<u64>,
    /// Ledger timestamp at which the escrow entered `EscrowState::Holdback`
    /// via `mark_holdback_escrow`. `None` until then (and for escrows that
    /// never reach `Holdback`). Used by `release_expired_holdback` to permit
    /// a permissionless payout to the driver once the admin-configurable
    /// holdback window has elapsed, so a passive recipient can no longer
    /// strand driver funds indefinitely (Issue #192).
    pub holdback_started_at: Option<u64>,
    pub fleet_id: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverProfile {
    pub address: Address,
    pub deliveries_completed: u32,
    pub reputation_score: u32,
    pub registered_at: u64,
    pub kyc_verified: bool,
    /// Lifecycle status — `Active` on registration, `Suspended` after an
    /// admin call to `suspend_driver`, restorable via `reinstate_driver`.
    pub status: DriverStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserProfile {
    pub address: Address,
    pub registered_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CargoCategory {
    Documents,
    Electronics,
    Perishables,
    Clothing,
    General,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoDescriptor {
    pub weight_grams: u32,
    pub category: CargoCategory,
    pub fragile: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryMetadata {
    /// Not caller-authoritative: `delivery_contract` overwrites this with
    /// the internally generated `DeliveryId` on every create/update call,
    /// discarding whatever value the caller supplied. Kept as a field
    /// (rather than removed) so a `DeliveryMetadata` read back from storage
    /// is self-describing without a second lookup.
    pub delivery_id: u64,
    pub origin: String,
    pub destination: String,
    pub cargo_description: CargoDescriptor,
    pub created_at: u64,
    pub estimated_delivery: u64,
}

#[cfg(test)]
mod test {
    use super::{
        delivery_key, escrow_key, CargoCategory, CargoDescriptor, DeliveryConfirmedEvent,
        DeliveryCreatedEvent, DeliveryDisputedEvent, DeliveryId, DeliveryMetadata, DeliveryRecord,
        DeliveryStatus, DriverAssignedEvent, DriverProfile, EscrowFundedEvent, EscrowRecord,
        EscrowRefundedEvent, EscrowReleasedEvent, EscrowState, FaniLabError, ProtocolConfig,
        StorageKey, UserProfile,
    };
    use soroban_sdk::{contract, testutils::Address as _, Address, Env, String};

    // `is_admin` is a free function reading instance storage, which SDK 27
    // only allows from within a contract's execution context — this minimal
    // contract exists solely to give these unit tests one to run inside via
    // `env.as_contract(...)`.
    #[contract]
    struct TestContract;

    #[test]
    fn is_admin_returns_false_before_init() {
        // Verify that is_admin returns false (not panics) when called before
        // the contract has been initialised — i.e. when StorageKey::Admin is
        // absent from instance storage.  This test pins the pre-init
        // behaviour so both escrow_contract and delivery_contract are
        // consistent (issue #68).
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let caller = Address::generate(&env);
        // StorageKey::Admin was never written — is_admin must return false.
        let result = env.as_contract(&contract_id, || super::is_admin(&env, &caller));
        assert!(!result, "is_admin should return false when uninitialized");
    }

    #[test]
    fn is_admin_returns_true_for_matching_admin() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let admin = Address::generate(&env);
        // Manually store the admin as the contract would during `init`.
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&StorageKey::Admin, &admin);
        });
        assert!(env.as_contract(&contract_id, || super::is_admin(&env, &admin)));
    }

    #[test]
    fn is_admin_returns_false_for_non_admin() {
        let env = Env::default();
        let contract_id = env.register(TestContract, ());
        let admin = Address::generate(&env);
        let other = Address::generate(&env);
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&StorageKey::Admin, &admin);
        });
        assert!(!env.as_contract(&contract_id, || super::is_admin(&env, &other)));
    }

    #[test]
    fn delivery_id_wraps_raw_u64() {
        let delivery_id = DeliveryId::new(42);

        assert_eq!(delivery_id, 42);
        assert_eq!(u64::from(delivery_id), 42);
    }

    #[test]
    fn delivery_and_escrow_states_expose_expected_variants() {
        assert_eq!(DeliveryStatus::Pending, DeliveryStatus::Pending);
        assert_eq!(DeliveryStatus::Active, DeliveryStatus::Active);
        assert_eq!(DeliveryStatus::InTransit, DeliveryStatus::InTransit);
        assert_eq!(DeliveryStatus::Delivered, DeliveryStatus::Delivered);
        assert_eq!(DeliveryStatus::Disputed, DeliveryStatus::Disputed);
        assert_eq!(DeliveryStatus::Cancelled, DeliveryStatus::Cancelled);

        assert_eq!(EscrowState::Locked, EscrowState::Locked);
        assert_eq!(EscrowState::Released, EscrowState::Released);
        assert_eq!(EscrowState::Refunded, EscrowState::Refunded);
        assert_eq!(EscrowState::Paused, EscrowState::Paused);
        assert_eq!(EscrowState::Split, EscrowState::Split);
    }

    #[test]
    fn storage_key_helpers_construct_expected_variants() {
        let delivery_id = DeliveryId::new(7);

        assert_eq!(delivery_key(delivery_id), StorageKey::Delivery(delivery_id));
        assert_eq!(escrow_key(delivery_id), StorageKey::Escrow(delivery_id));
    }

    #[test]
    fn unauthorized_has_expected_discriminant() {
        assert_eq!(FaniLabError::Unauthorized as u32, 1);
    }

    #[test]
    fn already_initialized_has_expected_discriminant() {
        assert_eq!(FaniLabError::AlreadyInitialized as u32, 2);
    }

    #[test]
    fn not_initialized_has_expected_discriminant() {
        assert_eq!(FaniLabError::NotInitialized as u32, 3);
    }

    #[test]
    fn delivery_not_found_has_expected_discriminant() {
        assert_eq!(FaniLabError::DeliveryNotFound as u32, 4);
    }

    #[test]
    fn invalid_state_has_expected_discriminant() {
        assert_eq!(FaniLabError::InvalidState as u32, 5);
    }

    #[test]
    fn insufficient_funds_has_expected_discriminant() {
        assert_eq!(FaniLabError::InsufficientFunds as u32, 6);
    }

    #[test]
    fn duplicate_delivery_has_expected_discriminant() {
        assert_eq!(FaniLabError::DuplicateDelivery as u32, 8);
    }

    #[test]
    fn provider_not_found_has_expected_discriminant() {
        assert_eq!(FaniLabError::ProviderNotFound as u32, 9);
    }

    #[test]
    fn delivery_created_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let event = DeliveryCreatedEvent {
            delivery_id: 1,
            sender: sender.clone(),
        };

        assert_eq!(event.delivery_id, 1);
        assert_eq!(event.sender, sender);
    }

    #[test]
    fn escrow_funded_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let token = Address::generate(&env);
        let event = EscrowFundedEvent {
            delivery_id: 2,
            sender: sender.clone(),
            token: token.clone(),
            amount: 250,
        };

        assert_eq!(event.delivery_id, 2);
        assert_eq!(event.sender, sender);
        assert_eq!(event.token, token);
        assert_eq!(event.amount, 250);
    }

    #[test]
    fn driver_assigned_event_preserves_fields() {
        let env = Env::default();
        let driver = Address::generate(&env);
        let event = DriverAssignedEvent {
            delivery_id: 3,
            driver: driver.clone(),
        };

        assert_eq!(event.delivery_id, 3);
        assert_eq!(event.driver, driver);
    }

    #[test]
    fn delivery_confirmed_event_preserves_fields() {
        let env = Env::default();
        let recipient = Address::generate(&env);
        let event = DeliveryConfirmedEvent {
            delivery_id: 4,
            recipient: recipient.clone(),
            timestamp: 12345,
        };

        assert_eq!(event.delivery_id, 4);
        assert_eq!(event.recipient, recipient);
        assert_eq!(event.timestamp, 12345);
    }

    #[test]
    fn escrow_released_event_preserves_fields() {
        let env = Env::default();
        let driver = Address::generate(&env);
        let event = EscrowReleasedEvent {
            delivery_id: 5,
            driver: driver.clone(),
            amount: 500,
            platform_fee: 10,
        };

        assert_eq!(event.delivery_id, 5);
        assert_eq!(event.driver, driver);
        assert_eq!(event.amount, 500);
        assert_eq!(event.platform_fee, 10);
    }

    #[test]
    fn delivery_disputed_event_preserves_fields() {
        let env = Env::default();
        let reporter = Address::generate(&env);
        let event = DeliveryDisputedEvent {
            delivery_id: 6,
            reporter: reporter.clone(),
            timestamp: 56789,
        };

        assert_eq!(event.delivery_id, 6);
        assert_eq!(event.reporter, reporter);
        assert_eq!(event.timestamp, 56789);
    }

    #[test]
    fn escrow_refunded_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let event = EscrowRefundedEvent {
            delivery_id: 7,
            sender: sender.clone(),
            amount: 700,
        };

        assert_eq!(event.delivery_id, 7);
        assert_eq!(event.sender, sender);
        assert_eq!(event.amount, 700);
    }

    #[test]
    fn test_cargo_descriptor() {
        let _env = Env::default();
        let desc = CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        };
        assert_eq!(desc.weight_grams, 500);
        assert!(desc.fragile);
        assert_eq!(desc.category, CargoCategory::Electronics);
    }

    #[test]
    fn test_delivery_metadata() {
        let env = Env::default();
        let cargo = CargoDescriptor {
            weight_grams: 1000,
            category: CargoCategory::General,
            fragile: false,
        };
        let metadata = DeliveryMetadata {
            delivery_id: 1,
            origin: String::from_str(&env, "Location A"),
            destination: String::from_str(&env, "Location B"),
            cargo_description: cargo,
            created_at: 1000000,
            estimated_delivery: 2000000,
        };
        assert_eq!(metadata.delivery_id, 1);
        assert_eq!(metadata.created_at, 1000000);
        assert_eq!(metadata.cargo_description.weight_grams, 1000);
    }

    #[test]
    fn protocol_config_preserves_fields() {
        let env = Env::default();
        let token = Address::generate(&env);
        let config = ProtocolConfig {
            token: token.clone(),
            platform_fee_bps: 500,
            protocol_version: 1,
            slippage_tolerance_bps: 100,
        };

        assert_eq!(config.token, token);
        assert_eq!(config.platform_fee_bps, 500);
        assert_eq!(config.protocol_version, 1);
        assert_eq!(config.slippage_tolerance_bps, 100);
    }

    #[test]
    fn delivery_record_preserves_fields() {
        let env = Env::default();
        let delivery_id = DeliveryId::new(99);
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let driver = Address::generate(&env);
        let cargo = CargoDescriptor {
            weight_grams: 2000,
            category: CargoCategory::Electronics,
            fragile: true,
        };
        let metadata = DeliveryMetadata {
            delivery_id: 99,
            origin: String::from_str(&env, "Origin"),
            destination: String::from_str(&env, "Destination"),
            cargo_description: cargo,
            created_at: 5000000,
            estimated_delivery: 6000000,
        };
        let record = DeliveryRecord {
            delivery_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            driver: Some(driver.clone()),
            status: DeliveryStatus::InTransit,
            metadata,
            created_at: 5000000,
            delivered_at: Some(5500000),
            transit_started_at: Some(5100000),
        };

        assert_eq!(record.delivery_id, delivery_id);
        assert_eq!(record.sender, sender);
        assert_eq!(record.recipient, recipient);
        assert_eq!(record.driver, Some(driver));
        assert_eq!(record.status, DeliveryStatus::InTransit);
        assert_eq!(record.created_at, 5000000);
        assert_eq!(record.delivered_at, Some(5500000));
        assert_eq!(record.transit_started_at, Some(5100000));
    }

    #[test]
    fn escrow_record_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let recipient = Address::generate(&env);
        let driver = Address::generate(&env);
        let token = Address::generate(&env);
        let disputed_by = Address::generate(&env);
        let record = EscrowRecord {
            delivery_id: 42,
            sender: sender.clone(),
            recipient: recipient.clone(),
            driver: driver.clone(),
            token: token.clone(),
            amount: 1000000,
            status: EscrowState::Locked,
            created_at: 7000000,
            expires_at: Some(8000000),
            disputed_by: Some(disputed_by.clone()),
            disputed_at: Some(7500000),
            holdback_started_at: Some(7200000),
            fleet_id: Some(42),
        };

        assert_eq!(record.delivery_id, 42);
        assert_eq!(record.sender, sender);
        assert_eq!(record.recipient, recipient);
        assert_eq!(record.driver, driver);
        assert_eq!(record.token, token);
        assert_eq!(record.amount, 1000000);
        assert_eq!(record.status, EscrowState::Locked);
        assert_eq!(record.created_at, 7000000);
        assert_eq!(record.expires_at, Some(8000000));
        assert_eq!(record.disputed_by, Some(disputed_by));
        assert_eq!(record.disputed_at, Some(7500000));
        assert_eq!(record.holdback_started_at, Some(7200000));
        assert_eq!(record.fleet_id, Some(42));
    }

    #[test]
    fn driver_profile_preserves_fields() {
        let env = Env::default();
        let address = Address::generate(&env);
        let profile = DriverProfile {
            address: address.clone(),
            deliveries_completed: 12,
            reputation_score: 85,
            registered_at: 1000000,
            kyc_verified: true,
            status: DriverStatus::Active,
        };

        assert_eq!(profile.address, address);
        assert_eq!(profile.deliveries_completed, 12);
        assert_eq!(profile.reputation_score, 85);
        assert_eq!(profile.registered_at, 1000000);
        assert!(profile.kyc_verified);
    }

    #[test]
    fn user_profile_preserves_fields() {
        let env = Env::default();
        let address = Address::generate(&env);
        let profile = UserProfile {
            address: address.clone(),
            registered_at: 2000000,
        };

        assert_eq!(profile.address, address);
        assert_eq!(profile.registered_at, 2000000);
    }
}

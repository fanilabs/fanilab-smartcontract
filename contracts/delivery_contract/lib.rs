#![no_std]

use shared_types::FaniLabError;
use shared_types::{
    delivery_key, events, is_admin, ttl, DeliveryConfirmedEvent, DeliveryCreatedEvent,
    DeliveryDisputedEvent, DeliveryMetadata, DriverAssignedEvent, DriverProfile, StorageKey,
};
pub use shared_types::{DeliveryId, DeliveryRecord, DeliveryStatus};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, IntoVal,
    Symbol,
};

// Local DeliveryMetadata removed in favor of shared_types::DeliveryMetadata

/// Auto-registers a `UserProfile` for `user` in the configured identity
/// contract, if one doesn't already exist.
///
/// Only ever called with `sender` — never `recipient` — because
/// `identity_reputation_contract::register_user` requires `user.require_auth()`.
/// `sender` is the party that already authorized this top-level call, so the
/// nested cross-contract `require_auth()` for the same address is satisfied
/// by that existing authorization. `recipient` has not authorized anything
/// at delivery-creation time (a sender may create a delivery to any
/// recipient address unilaterally), so calling this on their behalf would
/// require a real signature this flow does not and should not have.
fn ensure_user_profile(env: &Env, identity_contract: &Address, user: &Address) {
    let has_profile: bool = env.invoke_contract(
        identity_contract,
        &Symbol::new(env, "has_user_profile"),
        soroban_sdk::vec![env, user.into_val(env)],
    );
    if !has_profile {
        let _: shared_types::UserProfile = env.invoke_contract(
            identity_contract,
            &Symbol::new(env, "register_user"),
            soroban_sdk::vec![env, user.into_val(env)],
        );
    }
}

/// Maximum deliveries per batch to stay within Soroban resource limits.
pub const MAX_BATCH_SIZE: u32 = 100;

fn require_escrow_not_paused(env: &Env) {
    let escrow_contract: Address = env
        .storage()
        .instance()
        .get(&DataKey::EscrowContract)
        .unwrap_or_else(|| panic_with_error!(env, FaniLabError::NotInitialized));
    let paused: bool = env.invoke_contract(
        &escrow_contract,
        &Symbol::new(env, "is_paused"),
        soroban_sdk::vec![env],
    );
    if paused {
        panic_with_error!(env, FaniLabError::ProtocolPaused);
    }
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    DeliveryCounter,
    EscrowContract,
    DeliveryIndex(Address, u32, u32),
    DeliveryIndexLen(Address, u32),
    IdentityReputationContract,
    /// When `true`, `assign_driver` requires the driver to have
    /// `kyc_verified = true` in the identity contract before assignment is
    /// permitted.  Defaults to `false` so existing deployments and test
    /// fixtures continue to work unchanged (Issue #314).
    RequireKyc,
}

const INDEX_PAGE: u32 = 64;
#[rustfmt::skip]
fn index_push(env: &Env, owner: &Address, kind: u32, id: DeliveryId) {
    let len_key = DataKey::DeliveryIndexLen(owner.clone(), kind);
    let len: u32 = env.storage().persistent().get(&len_key).unwrap_or(0);
    let key = DataKey::DeliveryIndex(owner.clone(), kind, len / INDEX_PAGE);
    let mut page: soroban_sdk::Vec<DeliveryId> = env.storage().persistent().get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    page.push_back(id);
    env.storage().persistent().set(&key, &page);
    env.storage().persistent().set(&len_key, &(len + 1));
}

#[rustfmt::skip]
fn index_page(env: &Env, owner: Address, kind: u32, offset: u32, limit: u32) -> soroban_sdk::Vec<DeliveryId> {
    let len: u32 = env.storage().persistent()
        .get(&DataKey::DeliveryIndexLen(owner.clone(), kind)).unwrap_or(0);
    let mut out = soroban_sdk::Vec::new(env);
    let end = len.min(offset.saturating_add(limit.min(100)));
    for i in offset.min(len)..end {
        let page: soroban_sdk::Vec<DeliveryId> = env.storage().persistent()
            .get(&DataKey::DeliveryIndex(owner.clone(), kind, i / INDEX_PAGE))
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));
        if let Some(id) = page.get(i % INDEX_PAGE) { out.push_back(id); }
    }
    out
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DeliveryError {
    InvalidState = 1,
    InvalidMetadata = 2,
    /// A batch operation (e.g. create_deliveries_batch) exceeded MAX_BATCH_SIZE.
    BatchTooLarge = 3,
    /// A driver address matched the delivery's sender or recipient, which is
    /// never valid — enforced both at assignment time and again at
    /// confirmation time as a defense-in-depth check.
    InvalidDriver = 4,
    /// Sender and recipient must be different parties.
    InvalidParties = 5,
    /// The escrow securing this delivery is not present, or is not in the
    /// `Locked` state, at a point where funded escrow is a precondition
    /// (currently: the `Active -> InTransit` transition). See Issue #298.
    EscrowNotLocked = 6,
}

mod constants {
    pub const MAX_LOCATION_LEN: u32 = 256;
    pub const MAX_WEIGHT_GRAMS: u32 = 1_000_000;
}

/// Validate whether a status transition is permitted by the delivery state machine.
///
/// Allowed transitions:
///   Pending   → Active, Cancelled
///   Active    → InTransit, Disputed, Cancelled
///   InTransit → Delivered, Disputed, Cancelled  (Issue #300: Cancelled added so
///               reclaim_delivery_expired_escrow can synchronise an in-transit
///               delivery whose escrow expired)
///   Delivered → Disputed
///   Disputed  → Delivered (only via dispute resolution)
///   Cancelled → (terminal, no transitions)
///   Delivered → (terminal, no further transitions)
pub fn validate_transition(from: DeliveryStatus, to: DeliveryStatus) -> Result<(), FaniLabError> {
    let valid = matches!(
        (from, to),
        (DeliveryStatus::Pending, DeliveryStatus::Active)
            | (DeliveryStatus::Pending, DeliveryStatus::Cancelled)
            | (DeliveryStatus::Active, DeliveryStatus::InTransit)
            | (DeliveryStatus::Active, DeliveryStatus::Disputed)
            | (DeliveryStatus::Active, DeliveryStatus::Cancelled)
            | (DeliveryStatus::InTransit, DeliveryStatus::Delivered)
            | (DeliveryStatus::InTransit, DeliveryStatus::Disputed)
            | (DeliveryStatus::InTransit, DeliveryStatus::Cancelled)
            | (DeliveryStatus::Delivered, DeliveryStatus::Disputed)
            | (DeliveryStatus::Disputed, DeliveryStatus::Delivered)
    );
    if valid {
        Ok(())
    } else {
        Err(FaniLabError::InvalidState)
    }
}

fn validate_delivery_metadata(
    _env: &Env,
    metadata: &DeliveryMetadata,
) -> Result<(), DeliveryError> {
    if metadata.origin.is_empty() || metadata.origin.len() > constants::MAX_LOCATION_LEN {
        return Err(DeliveryError::InvalidMetadata);
    }
    if metadata.destination.is_empty() || metadata.destination.len() > constants::MAX_LOCATION_LEN {
        return Err(DeliveryError::InvalidMetadata);
    }
    if metadata.cargo_description.weight_grams == 0
        || metadata.cargo_description.weight_grams > constants::MAX_WEIGHT_GRAMS
    {
        return Err(DeliveryError::InvalidMetadata);
    }
    Ok(())
}

#[contract]
pub struct DeliveryContract;

#[contractimpl]
impl DeliveryContract {
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn init(env: Env, admin: Address, escrow_contract: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &0u64);
        env.storage().persistent().extend_ttl(
            &DataKey::DeliveryCounter,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::delivery_contract_initialized(&env),),
            (admin, escrow_contract),
        );
    }

    pub fn set_identity_reputation_contract(env: Env, admin: Address, identity_contract: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::IdentityReputationContract, &identity_contract);
    }

    pub fn get_identity_reputation_contract(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
    }

    /// Enable or disable the KYC gate on `assign_driver` (Issue #314).
    ///
    /// When `required` is `true`, every subsequent call to `assign_driver`
    /// will cross-call `identity_reputation_contract::get_driver_profile` and
    /// reject drivers whose `kyc_verified` field is `false` or who have no
    /// profile at all.
    ///
    /// When `required` is `false` (the default) KYC is not checked and
    /// existing flows are unaffected.  The default allows existing test
    /// fixtures and testnet deployments to keep working without changes.
    ///
    /// The gate is silently skipped when no identity contract is configured,
    /// matching the optional-integration pattern used by reputation calls,
    /// so deployments that have not wired up the identity contract continue
    /// to work even when this flag is `true`.
    ///
    /// **Authorization:** Admin only.
    #[allow(deprecated)] // events().publish() deprecated in SDK 27; see SOROBAN_SDK_27_MIGRATION.md
    pub fn set_require_kyc(env: Env, admin: Address, required: bool) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::RequireKyc, &required);
        env.events().publish(
            (Symbol::new(&env, "require_kyc_updated"),),
            (admin, required),
        );
    }

    /// Returns the current KYC-enforcement setting.  `true` means
    /// `assign_driver` rejects unverified drivers; `false` (default) means
    /// the gate is disabled (Issue #314).
    pub fn get_require_kyc(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::RequireKyc)
            .unwrap_or(false)
    }

    /// Returns the escrow_contract address this delivery_contract was
    /// initialised with (Issue #129 — deployment docs referenced this
    /// accessor before it existed).
    pub fn get_escrow_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn create_delivery(
        env: Env,
        sender: Address,
        recipient: Address,
        metadata: DeliveryMetadata,
    ) -> DeliveryId {
        sender.require_auth();
        require_escrow_not_paused(&env);

        if sender == recipient {
            panic_with_error!(&env, DeliveryError::InvalidParties);
        }

        validate_delivery_metadata(&env, &metadata)
            .unwrap_or_else(|_| panic_with_error!(&env, DeliveryError::InvalidMetadata));

        if let Some(identity_contract) = Self::get_identity_reputation_contract(env.clone()) {
            ensure_user_profile(&env, &identity_contract, &sender);
        }

        // Extend instance TTL on every delivery creation so the protocol's
        // core configuration pointers (Admin, EscrowContract,
        // IdentityReputationContract) cannot be archived while the contract is
        // actively used (Issue #393).
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        let mut counter: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::DeliveryCounter)
            .unwrap_or(0);
        counter += 1;
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &counter);
        env.storage().persistent().extend_ttl(
            &DataKey::DeliveryCounter,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let delivery_id = DeliveryId::from(counter);

        // The caller-supplied metadata.delivery_id is never authoritative -
        // overwrite it with the internally generated ID so the two can never
        // diverge (Issue #45), without removing the field from the public
        // DeliveryMetadata struct.
        let mut metadata = metadata;
        metadata.delivery_id = counter;

        let record = DeliveryRecord {
            delivery_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            driver: None,
            status: DeliveryStatus::Pending,
            metadata,
            created_at: env.ledger().timestamp(),
            delivered_at: None,
            transit_started_at: None,
        };

        let key = delivery_key(delivery_id);
        env.storage().persistent().set(&key, &record);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        index_push(&env, &sender, 0, delivery_id);
        index_push(&env, &recipient, 1, delivery_id);
        env.events().publish(
            (events::delivery_created(&env),),
            DeliveryCreatedEvent {
                delivery_id: delivery_id.value(),
                sender,
            },
        );

        delivery_id
    }

    /// Create multiple deliveries in a single transaction. Sender must authorize.
    /// Returns Vec of created delivery IDs. Each delivery is stored with Pending status
    /// and secondary indexes are updated, but NO escrow is created.
    ///
    /// **IMPORTANT:** This function DOES NOT create escrows. Escrow creation is a separate,
    /// required step: after obtaining delivery IDs from this function, call
    /// `escrow_contract::create_escrows_batch` with the returned delivery_ids to fund
    /// the escrows. The two operations must be paired: deliveries without escrows will
    /// fail at driver assignment or delivery confirmation with DeliveryNotFound errors.
    ///
    /// **Integration Sequence:**
    /// 1. Call `delivery_contract::create_deliveries_batch` with metadata → returns Vec<DeliveryId>
    /// 2. Call `escrow_contract::create_escrows_batch` with (delivery_id, driver, amount, fleet_id) tuples
    ///
    /// The ordering constraint exists because delivery_ids must be known before escrows
    /// can reference them, and `create_escrows_batch` accepts explicit delivery_ids.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn create_deliveries_batch(
        env: Env,
        sender: Address,
        recipient: Address,
        metadata_list: soroban_sdk::Vec<DeliveryMetadata>,
    ) -> soroban_sdk::Vec<DeliveryId> {
        sender.require_auth();
        require_escrow_not_paused(&env);

        if sender == recipient {
            panic_with_error!(&env, DeliveryError::InvalidParties);
        }

        if metadata_list.len() > MAX_BATCH_SIZE {
            panic_with_error!(&env, DeliveryError::BatchTooLarge);
        }

        for i in 0..metadata_list.len() {
            if let Some(metadata) = metadata_list.get(i) {
                validate_delivery_metadata(&env, &metadata)
                    .unwrap_or_else(|_| panic_with_error!(&env, DeliveryError::InvalidMetadata));
            }
        }

        if let Some(identity_contract) = Self::get_identity_reputation_contract(env.clone()) {
            ensure_user_profile(&env, &identity_contract, &sender);
        }

        let mut result = soroban_sdk::Vec::new(&env);
        let mut counter: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::DeliveryCounter)
            .unwrap_or(0);

        let timestamp = env.ledger().timestamp();

        for i in 0..metadata_list.len() {
            if let Some(mut metadata) = metadata_list.get(i) {
                counter += 1;
                env.storage()
                    .persistent()
                    .set(&DataKey::DeliveryCounter, &counter);
                env.storage().persistent().extend_ttl(
                    &DataKey::DeliveryCounter,
                    ttl::LEDGER_TTL_THRESHOLD,
                    ttl::LEDGER_TTL_EXTEND_TO,
                );

                let delivery_id = DeliveryId::from(counter);
                // See create_delivery: overwrite with the real generated ID
                // rather than trusting the caller-supplied value (Issue #45).
                metadata.delivery_id = counter;

                let record = DeliveryRecord {
                    delivery_id,
                    sender: sender.clone(),
                    recipient: recipient.clone(),
                    driver: None,
                    status: DeliveryStatus::Pending,
                    metadata,
                    created_at: timestamp,
                    delivered_at: None,
                    transit_started_at: None,
                };

                let key = delivery_key(delivery_id);
                env.storage().persistent().set(&key, &record);
                env.storage().persistent().extend_ttl(
                    &key,
                    ttl::LEDGER_TTL_THRESHOLD,
                    ttl::LEDGER_TTL_EXTEND_TO,
                );

                index_push(&env, &sender, 0, delivery_id);
                index_push(&env, &recipient, 1, delivery_id);

                env.events().publish(
                    (events::delivery_created(&env),),
                    DeliveryCreatedEvent {
                        delivery_id: delivery_id.value(),
                        sender: sender.clone(),
                    },
                );

                result.push_back(delivery_id);
            }
        }

        result
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn update_delivery_metadata(
        env: Env,
        sender: Address,
        delivery_id: DeliveryId,
        metadata: DeliveryMetadata,
    ) {
        sender.require_auth();
        require_escrow_not_paused(&env);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if delivery.sender != sender {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        if delivery.status != DeliveryStatus::Pending {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        validate_delivery_metadata(&env, &metadata)
            .unwrap_or_else(|_| panic_with_error!(&env, DeliveryError::InvalidMetadata));

        // See create_delivery: overwrite with the real, already-assigned ID
        // rather than trusting the caller-supplied value (Issue #45).
        let mut metadata = metadata;
        metadata.delivery_id = delivery_id.into();
        delivery.metadata = metadata;

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (Symbol::new(&env, "delivery_metadata_updated"),),
            (delivery_id, sender),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn cancel_delivery(env: Env, sender: Address, delivery_id: DeliveryId) {
        sender.require_auth();
        require_escrow_not_paused(&env);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if delivery.sender != sender {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        validate_transition(delivery.status, DeliveryStatus::Cancelled)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &soroban_sdk::Symbol::new(&env, "refund_escrow"),
            soroban_sdk::vec![
                &env,
                sender.into_val(&env),
                u64::from(delivery_id).into_val(&env),
            ],
        );

        delivery.status = DeliveryStatus::Cancelled;
        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events()
            .publish((events::delivery_cancelled(&env),), (delivery_id, sender));
    }

    /// Assign a driver and move the delivery `Pending -> Active`.
    ///
    /// Issue #298 decision: `assign_driver` deliberately does **not** verify
    /// that the escrow exists and is `Locked`. Assignment is a weak,
    /// reversible commitment (the delivery can still be cancelled, and a
    /// different driver assigned), and during onboarding it frequently races
    /// escrow funding. The escrow precondition is enforced one step later, at
    /// `mark_in_transit` — the point where the driver physically takes
    /// custody of the package and the commitment becomes real.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn assign_driver(env: Env, caller: Address, delivery_id: DeliveryId, driver: Address) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let is_caller_admin = is_admin(&env, &caller);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        // Only admin or the delivery sender can assign a driver
        let is_sender = caller == delivery.sender;
        if !is_caller_admin && !is_sender {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        if driver == delivery.sender || driver == delivery.recipient {
            panic_with_error!(&env, DeliveryError::InvalidDriver);
        }

        let identity_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        let is_suspended: bool = env.invoke_contract(
            &identity_contract,
            &Symbol::new(&env, "is_driver_suspended"),
            soroban_sdk::vec![&env, driver.into_val(&env)],
        );
        if is_suspended {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        validate_transition(delivery.status, DeliveryStatus::Active)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        // Issue #314: when `require_kyc` is enabled and the identity contract
        // is configured, verify that the driver's profile exists and that
        // `kyc_verified == true` before proceeding.  The gate is skipped
        // when the identity contract is absent so deployments that have not
        // wired it up continue to work even with the flag set.
        let require_kyc: bool = env
            .storage()
            .instance()
            .get(&DataKey::RequireKyc)
            .unwrap_or(false);
        if require_kyc {
            if let Some(identity_contract) = Self::get_identity_reputation_contract(env.clone()) {
                // `has_driver_profile` is a non-panicking presence check.
                let has_profile: bool = env.invoke_contract(
                    &identity_contract,
                    &Symbol::new(&env, "has_driver_profile"),
                    soroban_sdk::vec![&env, driver.clone().into_val(&env)],
                );
                if !has_profile {
                    panic_with_error!(&env, FaniLabError::ProviderNotFound);
                }
                let profile: DriverProfile = env.invoke_contract(
                    &identity_contract,
                    &Symbol::new(&env, "get_driver_profile"),
                    soroban_sdk::vec![&env, driver.clone().into_val(&env)],
                );
                if !profile.kyc_verified {
                    panic_with_error!(&env, FaniLabError::Unauthorized);
                }
            }
        }

        delivery.driver = Some(driver.clone());
        delivery.status = DeliveryStatus::Active;

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // Extend instance TTL on every driver assignment so the protocol's
        // core configuration pointers cannot be archived while the contract
        // is actively used (Issue #393).
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        env.events().publish(
            (events::driver_assigned(&env),),
            DriverAssignedEvent {
                delivery_id: delivery_id.value(),
                driver,
            },
        );
    }

    /// Allow the assigned driver to mark a delivery as actively in transit.
    /// Transitions: Active -> InTransit. Records the ledger timestamp.
    ///
    /// Issue #298: before advancing, this verifies via a cross-call to
    /// `escrow_contract::get_escrow` that the escrow securing this delivery
    /// exists and is `Locked`. escrow creation is a separate call on a
    /// separate contract, so without this a driver could take custody of a
    /// package for a delivery that has no escrow at all, or one whose escrow
    /// was already refunded (e.g. by `reclaim_expired_escrow`, which does not
    /// touch the delivery). A missing escrow surfaces as the escrow
    /// contract's `DeliveryNotFound`; a present-but-not-`Locked` escrow
    /// yields the typed `DeliveryError::EscrowNotLocked`. This adds one
    /// cross-contract call to a hot path, which is a deliberate trade-off:
    /// the driver's assurance that funds exist is worth the extra read.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn mark_in_transit(env: Env, driver: Address, delivery_id: DeliveryId) {
        driver.require_auth();
        require_escrow_not_paused(&env);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        // Verify caller is the assigned driver for this delivery
        match &delivery.driver {
            Some(assigned) if *assigned == driver => {}
            _ => panic_with_error!(&env, FaniLabError::Unauthorized),
        }

        let identity_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        let is_suspended: bool = env.invoke_contract(
            &identity_contract,
            &Symbol::new(&env, "is_driver_suspended"),
            soroban_sdk::vec![&env, driver.into_val(&env)],
        );
        if is_suspended {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        validate_transition(delivery.status, DeliveryStatus::InTransit)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        // Issue #298: the escrow securing this delivery must exist and be
        // Locked before the driver commits to transit.
        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        let escrow: shared_types::EscrowRecord = env.invoke_contract(
            &escrow_address,
            &Symbol::new(&env, "get_escrow"),
            soroban_sdk::vec![&env, u64::from(delivery_id).into_val(&env)],
        );
        if escrow.status != shared_types::EscrowStatus::Locked {
            panic_with_error!(&env, DeliveryError::EscrowNotLocked);
        }

        let timestamp = env.ledger().timestamp();
        delivery.status = DeliveryStatus::InTransit;
        delivery.transit_started_at = Some(timestamp);

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::delivery_in_transit(&env),),
            (delivery_id, driver, timestamp),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn confirm_delivery(env: Env, recipient: Address, delivery_id: DeliveryId) {
        recipient.require_auth();
        require_escrow_not_paused(&env);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if recipient != delivery.recipient {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        if let Some(driver) = &delivery.driver {
            if driver == &recipient || driver == &delivery.sender {
                panic_with_error!(&env, DeliveryError::InvalidDriver);
            }
        }

        validate_transition(delivery.status, DeliveryStatus::Delivered)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &soroban_sdk::Symbol::new(&env, "mark_holdback_escrow"),
            soroban_sdk::vec![
                &env,
                // Issue #466: pass this contract's own address, not the
                // recipient's. The escrow now restricts this transition to the
                // configured `delivery_contract` (or an admin) so a recipient
                // cannot strand the delivery record by calling the escrow
                // directly. Passing the recipient here would fail the escrow's
                // own authorization check on every confirmation.
                env.current_contract_address().into_val(&env),
                u64::from(delivery_id).into_val(&env),
            ],
        );

        delivery.status = DeliveryStatus::Delivered;
        delivery.delivered_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        if let Some(driver_addr) = &delivery.driver {
            if let Some(identity_contract) = Self::get_identity_reputation_contract(env.clone()) {
                let cargo_desc = &delivery.metadata.cargo_description;
                let _: () = env.invoke_contract(
                    &identity_contract,
                    &Symbol::new(&env, "increase_reputation"),
                    soroban_sdk::vec![
                        &env,
                        env.current_contract_address().into_val(&env),
                        driver_addr.into_val(&env),
                        u64::from(delivery_id).into_val(&env),
                        cargo_desc.weight_grams.into_val(&env),
                        cargo_desc.fragile.into_val(&env),
                    ],
                );
            }
        }

        env.events().publish(
            (events::delivery_confirmed(&env),),
            DeliveryConfirmedEvent {
                delivery_id: delivery_id.value(),
                recipient,
                timestamp: delivery.delivered_at.unwrap_or(0),
            },
        );
    }

    /// Allow sender or recipient to escalate a delivery to Disputed and pause
    /// the escrow via a cross-contract call. The escrow call executes first so
    /// that delivery state is never mutated when the escrow call fails.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn raise_dispute(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        let is_sender = caller == delivery.sender;
        let is_recipient = caller == delivery.recipient;
        let is_driver = delivery.driver.as_ref().map(|d| *d == caller).unwrap_or(false);
        if !is_sender && !is_recipient && !is_driver {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        validate_transition(delivery.status, DeliveryStatus::Disputed)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        // Cross-contract call first: if escrow raises dispute fails, delivery
        // state is not mutated (implicit rollback via propagated panic).
        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &Symbol::new(&env, "raise_dispute"),
            soroban_sdk::vec![
                &env,
                caller.into_val(&env),
                u64::from(delivery_id).into_val(&env),
            ],
        );

        let timestamp = env.ledger().timestamp();
        delivery.status = DeliveryStatus::Disputed;

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::delivery_disputed(&env),),
            DeliveryDisputedEvent {
                delivery_id: delivery_id.value(),
                reporter: caller,
                timestamp,
            },
        );
    }

    pub fn get_driver_profile(env: Env, driver: Address) -> DriverProfile {
        let identity_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        env.invoke_contract(
            &identity_contract,
            &Symbol::new(&env, "get_driver_profile"),
            soroban_sdk::vec![&env, driver.into_val(&env)],
        )
    }

    pub fn get_delivery(env: Env, delivery_id: DeliveryId) -> DeliveryRecord {
        let key = delivery_key(delivery_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound))
    }

    /// Returns combined delivery and escrow state, and flags known-invalid combinations.
    ///
    /// The escrow component is `None` when the escrow has not been created yet
    /// (i.e. the delivery is in `Pending` state and the sender has not funded it).
    /// Callers must handle the `None` case — it is a normal condition for freshly
    /// created deliveries, not an error (Issue #395).
    ///
    /// `is_synchronized` is `false` whenever the escrow is absent, since an
    /// unfunded delivery is not yet in a fully synchronized protocol state.
    pub fn get_combined_state(
        env: Env,
        delivery_id: DeliveryId,
    ) -> (DeliveryRecord, Option<shared_types::EscrowRecord>, bool) {
        let delivery = Self::get_delivery(env.clone(), delivery_id);

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        // Check whether the escrow record exists before fetching it. A delivery
        // in Pending state may not have a corresponding escrow yet — that is a
        // valid protocol state, not a bug, so we must not panic here (Issue #395).
        use soroban_sdk::IntoVal;
        let escrow_exists: bool = env.invoke_contract(
            &escrow_address,
            &Symbol::new(&env, "has_escrow"),
            soroban_sdk::vec![&env, u64::from(delivery_id).into_val(&env)],
        );

        if !escrow_exists {
            // No escrow yet — return None; the delivery and escrow states are
            // not synchronized by definition.
            return (delivery, None, false);
        }

        let escrow: shared_types::EscrowRecord = env.invoke_contract(
            &escrow_address,
            &Symbol::new(&env, "get_escrow"),
            soroban_sdk::vec![&env, u64::from(delivery_id).into_val(&env)],
        );

        let is_synchronized = Self::validate_state_sync(&delivery, &escrow);
        (delivery, Some(escrow), is_synchronized)
    }

    /// Validates that delivery and escrow states match expected protocol invariants.
    /// Returns true if states are synchronized, false if a mismatch is detected.
    fn validate_state_sync(delivery: &DeliveryRecord, escrow: &shared_types::EscrowRecord) -> bool {
        match (&delivery.status, &escrow.status) {
            // Pending/Active: escrow should be Locked
            (DeliveryStatus::Pending, shared_types::EscrowStatus::Locked) => true,
            (DeliveryStatus::Active, shared_types::EscrowStatus::Locked) => true,

            // InTransit: escrow should still be Locked
            (DeliveryStatus::InTransit, shared_types::EscrowStatus::Locked) => true,

            // Delivered: escrow can be in Holdback (post-confirmation, pre-release)
            // or Released (after driver payout completes)
            (DeliveryStatus::Delivered, shared_types::EscrowStatus::Holdback) => true,
            (DeliveryStatus::Delivered, shared_types::EscrowStatus::Released) => true,

            // Disputed: escrow must be Paused
            (DeliveryStatus::Disputed, shared_types::EscrowStatus::Paused) => true,

            // Cancelled: escrow should be Refunded
            (DeliveryStatus::Cancelled, shared_types::EscrowStatus::Refunded) => true,

            // Any other combination is a mismatch
            _ => false,
        }
    }

    /// Get all delivery IDs created by a sender.
    pub fn get_deliveries_by_sender(env: Env, sender: Address) -> soroban_sdk::Vec<DeliveryId> {
        index_page(&env, sender, 0, 0, 100)
    }

    /// Get all delivery IDs with a specific recipient.
    pub fn get_deliveries_by_recipient(
        env: Env,
        recipient: Address,
    ) -> soroban_sdk::Vec<DeliveryId> {
        index_page(&env, recipient, 1, 0, 100)
    }

    #[rustfmt::skip]
    pub fn get_deliveries_page(env: Env, owner: Address, kind: u32, offset: u32, limit: u32) -> soroban_sdk::Vec<DeliveryId> {
        index_page(&env, owner, kind, offset, limit)
    }

    /// Permissionless entry point that reclaims an expired escrow **and**
    /// simultaneously transitions the delivery to `Cancelled`, keeping
    /// both records synchronized (Issue #300).
    ///
    /// Design rationale (delivery-side driver):
    /// Every existing cross-contract call in this protocol runs
    /// delivery → escrow.  Introducing an escrow → delivery edge would add a
    /// new dependency direction and complicate reasoning about call ordering.
    /// By placing the combined logic here we preserve the existing direction:
    /// this function calls `reclaim_expired_escrow` on the escrow contract and
    /// then updates the delivery record itself.
    ///
    /// The escrow call runs first (checks-effects-interactions pattern): if it
    /// fails (not expired, not Locked, etc.) the delivery status is never
    /// mutated.
    ///
    /// Preconditions (enforced by the escrow contract):
    ///   - The escrow must be in `Locked` state.
    ///   - `expires_at` must have elapsed.
    ///
    /// Preconditions (enforced here):
    ///   - The delivery must exist.
    ///   - The delivery must be in a state from which `Cancelled` is a legal
    ///     transition: `Pending`, `Active`, or `InTransit`.
    ///
    /// Post-condition: `get_combined_state` reports synchronized
    /// (`Cancelled` + `Refunded`).
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn reclaim_delivery_expired_escrow(env: Env, delivery_id: DeliveryId) {
        // ── 1. Load delivery ────────────────────────────────────────────────
        let key = delivery_key(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        // ── 2. Validate delivery-side transition (before touching escrow) ──
        validate_transition(delivery.status, DeliveryStatus::Cancelled)
            .unwrap_or_else(|_| panic_with_error!(&env, FaniLabError::InvalidState));

        // ── 3. Call escrow contract (interaction before local state write) ─
        //    `reclaim_expired_escrow` panics when the escrow is not Locked,
        //    not expired, or the contract is paused — all of which propagate
        //    as a revert so the delivery status below is never mutated.
        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        let _: () = env.invoke_contract(
            &escrow_address,
            &soroban_sdk::Symbol::new(&env, "reclaim_expired_escrow"),
            soroban_sdk::vec![&env, u64::from(delivery_id).into_val(&env)],
        );

        // ── 4. Commit local delivery state change ──────────────────────────
        delivery.status = DeliveryStatus::Cancelled;
        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        // ── 5. Emit event ──────────────────────────────────────────────────
        env.events().publish(
            (events::delivery_cancelled(&env),),
            (delivery_id, delivery.sender),
        );
    }
}

#[cfg(test)]
mod test;

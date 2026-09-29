#![no_std]

use shared_types::{
    events, ttl, DeliveryId, DeliveryStatus, DisputeRaisedEvent, DisputeResolvedPayoutEvent,
    DisputeResolvedRefundEvent, DisputeResolvedSplitEvent, EscrowRecord, EscrowStatus,
    FaniLabError,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, BytesN, Env, IntoVal, Symbol,
    Vec,
};

// Fallbacks for the three admin-configurable reputation adjustments. Each is
// used by the corresponding `get_dispute_reputation_*` getter when no explicit
// configuration has been stored, and each can be retuned at runtime through
// `set_dispute_reputation_penalty`, `set_dispute_reputation_reward`, and
// `set_dispute_split_penalty` (Issues #139 and #462).
const DEFAULT_DISPUTE_REPUTATION_PENALTY: u32 = 10;
const DEFAULT_DISPUTE_REPUTATION_REWARD: u32 = 5;
const DEFAULT_DISPUTE_REPUTATION_SPLIT_PENALTY: u32 = 5;

/// Mirror of `identity_reputation_contract::MAX_REPUTATION` (the score ceiling,
/// currently 100). The dispute contract cannot import that crate without taking
/// a dependency on it, so the value is duplicated here as a documented constant;
/// the two must be kept in sync if the reputation ceiling ever changes.
const IDENTITY_MAX_REPUTATION: u32 = 100;

/// Upper bound for the admin-configurable dispute reputation penalty.
///
/// Without a ceiling a single mistyped configuration value (e.g. `100` instead
/// of `10`) turns every subsequent adverse ruling into a permanent reset of the
/// driver's reputation to zero. Half of `IDENTITY_MAX_REPUTATION` is the most a
/// single ruling may ever remove.
const MAX_DISPUTE_REPUTATION_PENALTY: u32 = IDENTITY_MAX_REPUTATION / 2;

// Compile-time sanity checks: every reputation adjustment this contract can
// apply — the default constants — must sit within the same ceiling enforced on
// the configurable values, which in turn must sit within the reputation score
// ceiling itself. A future edit that violates one of these fails the build
// rather than shipping a silently unsafe value.
#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(MAX_DISPUTE_REPUTATION_PENALTY <= IDENTITY_MAX_REPUTATION);
    assert!(DEFAULT_DISPUTE_REPUTATION_PENALTY <= MAX_DISPUTE_REPUTATION_PENALTY);
    assert!(DEFAULT_DISPUTE_REPUTATION_REWARD <= MAX_DISPUTE_REPUTATION_PENALTY);
    assert!(DEFAULT_DISPUTE_REPUTATION_SPLIT_PENALTY <= MAX_DISPUTE_REPUTATION_PENALTY);
};

const MIN_DISPUTE_TIME_LIMIT: u64 = 86400; // 1 day in seconds

/// Lower bound for the dispute *resolution* window (the delay before any party
/// may `force_resolve_dispute`). Mirrors `MIN_DISPUTE_TIME_LIMIT`: parties must
/// always have at least a day to reach a verdict before the automatic fallback
/// split can be forced. No upper bound is imposed — an over-long window only
/// delays a safety-valve fallback and creates no attack surface for the trusted
/// admin, so bounding it further would add friction without protection.
const MIN_DISPUTE_RESOLUTION_LIMIT: u64 = 86400; // 1 day in seconds

/// Maximum number of evidence hashes a single party may attach to one dispute.
///
/// The cap is enforced *per submitting party* rather than as one shared budget:
/// a shared cap let any one party exhaust it with duplicate/junk hashes and
/// permanently lock the counterparty out of recording evidence. With at most
/// three authorized parties (sender, recipient, driver) the hard per-dispute
/// ceiling is `3 * MAX_EVIDENCE_HASHES_PER_PARTY`, so storage growth stays
/// bounded (the original intent of issue #49).
const MAX_EVIDENCE_HASHES_PER_PARTY: u32 = 20;

/// Upper bound on the total admin roster size (Issue #464).
///
/// `AdminList` is loaded, iterated, and re-serialized by every governance
/// call. Soroban's strict CPU/memory budget means an unbounded roster could
/// be grown until deserializing it exceeds those limits, permanently bricking
/// `remove_admin`/`add_admin` and locking out governance entirely. The cap
/// keeps the vector safely within budget while leaving ample room for a
/// realistic multisig roster.
const MAX_ADMINS: u32 = 50;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Open,
    ResolvedRefund,
    ResolvedPayout,
    Split,
}

/// A single piece of evidence attached to a dispute, recorded together with the
/// party that submitted it. Storing the submitter (rather than a bare hash)
/// makes the per-party quota enforceable and gives an on-chain audit trail of
/// who supplied which hash.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceEntry {
    pub submitter: Address,
    pub hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCase {
    pub delivery_id: DeliveryId,
    pub status: DisputeStatus,
    pub raised_at: u64,
    pub raised_by: Address,
    // Pre-mainnet: this stored type changed from `Vec<BytesN<32>>` to carry the
    // submitter. No migration path is provided because no production records
    // exist yet.
    pub evidence_hashes: Vec<EvidenceEntry>,
    pub resolved_at: Option<u64>,
    pub resolved_by: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin(Address),
    AdminList,
    DeliveryContract,
    EscrowContract,
    IdentityReputationContract,
    DisputeTimeLimit,
    DisputeResolutionLimit,
    Dispute(DeliveryId),
    DisputeReputationPenalty,
    /// Paged index page: (page_number) → Vec<DeliveryId>
    /// Page size is `DISPUTE_INDEX_PAGE` entries.
    DisputeIndex(u32),
    /// Total number of disputes ever recorded (monotonically increasing).
    DisputeIndexLen,
}

/// Number of dispute IDs stored per index page.  Matches the paging
/// constant used by `escrow_contract` and `delivery_contract` (Issue #234).
const DISPUTE_INDEX_PAGE: u32 = 64;

/// Append `delivery_id` to the dispute enumeration index.
fn dispute_index_push(env: &Env, delivery_id: DeliveryId) {
    let len: u32 = env
        .storage()
        .instance()
        .get(&DataKey::DisputeIndexLen)
        .unwrap_or(0);
    let page_key = DataKey::DisputeIndex(len / DISPUTE_INDEX_PAGE);
    let mut page: soroban_sdk::Vec<DeliveryId> = env
        .storage()
        .instance()
        .get(&page_key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    page.push_back(delivery_id);
    env.storage().instance().set(&page_key, &page);
    env.storage()
        .instance()
        .set(&DataKey::DisputeIndexLen, &(len + 1));
}

#[contract]
pub struct DisputeResolutionContract;

impl DisputeResolutionContract {
    /// Issue #211: every admin resolution entry point requires the linked
    /// escrow to be in `Paused` (i.e. under active dispute) before it acts.
    /// `resolve_dispute_refund_sender`, `resolve_dispute_pay_driver`, and
    /// `resolve_dispute_split_funds` all call this first — before mutating the
    /// dispute record or making any cross-contract side effect — so a
    /// bad-state call fails fast here instead of surfacing as an
    /// `InvalidState` thrown several calls deep inside `escrow_contract`
    /// after a reputation adjustment has already been attempted.
    /// `escrow_contract::resolve_dispute`'s own guard remains the
    /// authoritative check; this is a fail-fast layer with one implementation
    /// shared by all three entry points.
    fn require_escrow_paused(env: &Env, delivery_id: DeliveryId) {
        let escrow_addr: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(env, FaniLabError::NotInitialized));
        let escrow: EscrowRecord = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(env, "get_escrow"),
            soroban_sdk::vec![env, u64::from(delivery_id).into_val(env)],
        );
        if escrow.status != EscrowStatus::Paused {
            panic_with_error!(env, FaniLabError::InvalidState);
        }
    }
}

#[contractimpl]
impl DisputeResolutionContract {
    pub fn init(
        env: Env,
        admin: Address,
        delivery_contract: Address,
        escrow_contract: Address,
        dispute_time_limit: u64,
        dispute_resolution_limit: u64,
    ) {
        // `AdminList` is the real initialization sentinel here: the contract
        // writes that list unconditionally during its first setup, while the
        // delivery-contract pointer is merely a configuration field and must not
        // control whether the contract can be re-initialized.
        if env.storage().instance().has(&DataKey::AdminList) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        if dispute_time_limit < MIN_DISPUTE_TIME_LIMIT {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        if dispute_resolution_limit < MIN_DISPUTE_RESOLUTION_LIMIT {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        env.storage()
            .instance()
            .set(&DataKey::DeliveryContract, &delivery_contract);
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .instance()
            .set(&DataKey::DisputeTimeLimit, &dispute_time_limit);
        env.storage()
            .instance()
            .set(&DataKey::DisputeResolutionLimit, &dispute_resolution_limit);
        env.storage()
            .instance()
            .set(&DataKey::Admin(admin.clone()), &true);

        let mut admin_list = Vec::new(&env);
        admin_list.push_back(admin);
        env.storage()
            .instance()
            .set(&DataKey::AdminList, &admin_list);
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn add_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let mut admin_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env));

        // Issue #464: bound the roster so it can never grow past the point
        // where loading it exceeds Soroban's deserialization budget. Checked
        // before any write so the whole call reverts cleanly.
        if admin_list.len() >= MAX_ADMINS {
            panic_with_error!(&env, FaniLabError::LimitExceeded);
        }

        env.storage()
            .instance()
            .set(&DataKey::Admin(new_admin.clone()), &true);

        if !admin_list.iter().any(|a| a == new_admin) {
            admin_list.push_back(new_admin.clone());
            env.storage()
                .instance()
                .set(&DataKey::AdminList, &admin_list);
        }

        // Issue #212: emit a roster-change event so the remaining admins and
        // any off-chain monitoring can detect additions immediately.
        env.events()
            .publish((Symbol::new(&env, "admin_added"),), (caller, new_admin));
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn remove_admin(env: Env, caller: Address, old_admin: Address) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let admin_list: Vec<Address> = env
            .storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut new_list = Vec::new(&env);
        for admin in admin_list.iter() {
            if admin != old_admin {
                new_list.push_back(admin);
            }
        }

        // Removing the last admin would permanently brick governance — no one
        // would be left who could call add_admin to recover.
        if new_list.is_empty() {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Issue #212: a single admin must not be able to consolidate the
        // roster down to only themselves in one uninterrupted sequence.
        // Reducing the roster to exactly one admin is therefore permitted
        // only when the caller is removing themselves (a deliberate
        // step-down); an admin removing a *different* admin may never leave
        // itself as the sole remaining admin. Legitimate removals still work
        // while at least one other admin remains, and a full hand-off is done
        // by adding the successor first and then stepping down.
        if new_list.len() == 1 && old_admin != caller {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        env.storage()
            .instance()
            .remove(&DataKey::Admin(old_admin.clone()));
        env.storage().instance().set(&DataKey::AdminList, &new_list);

        // Issue #212: emit a roster-change event identifying the caller and
        // the removed address for the remaining admins / off-chain monitors.
        env.events()
            .publish((Symbol::new(&env, "admin_removed"),), (caller, old_admin));
    }

    pub fn is_admin(env: Env, admin: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Admin(admin))
            .unwrap_or(false)
    }

    pub fn list_admins(env: Env) -> Vec<Address> {
        env.storage()
            .instance()
            .get(&DataKey::AdminList)
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn get_delivery_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::DeliveryContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn get_escrow_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_identity_reputation_contract(
        env: Env,
        caller: Address,
        reputation_contract: Address,
    ) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::IdentityReputationContract, &reputation_contract);
        // #382: emit event so off-chain indexers can track when this critical
        // contract pointer is updated by an admin.
        env.events().publish(
            (Symbol::new(&env, "identity_reputation_contract_set"),),
            (caller, reputation_contract),
        );
    }

    pub fn get_identity_reputation_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn get_dispute_time_limit(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeTimeLimit)
            .unwrap_or(0)
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_dispute_reputation_penalty(env: Env, caller: Address, penalty: u32) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        // A penalty at or above the reputation ceiling would zero any driver's
        // score on a single ruling; bound it to a documented fraction of that
        // ceiling and reject anything larger loudly.
        if penalty > MAX_DISPUTE_REPUTATION_PENALTY {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        let old_penalty = Self::get_dispute_reputation_penalty(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::DisputeReputationPenalty, &penalty);
        env.events().publish(
            (Symbol::new(&env, "dispute_penalty_updated"),),
            (caller, old_penalty, penalty),
        );
    }

    pub fn get_dispute_reputation_penalty(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeReputationPenalty)
            .unwrap_or(DEFAULT_DISPUTE_REPUTATION_PENALTY)
    }

    /// Configure the flat reputation credit awarded to the driver when a
    /// dispute resolves in their favour (`resolve_dispute_pay_driver`).
    /// Mirrors `set_dispute_reputation_penalty`: admin-only and bounded by
    /// `MAX_DISPUTE_REPUTATION_PENALTY` so a single ruling can never wipe out
    /// a driver's score (Issue #462).
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_dispute_reputation_reward(env: Env, caller: Address, reward: u32) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if reward > MAX_DISPUTE_REPUTATION_PENALTY {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        let old_reward = Self::get_dispute_reputation_reward(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::DisputeReputationReward, &reward);
        env.events().publish(
            (Symbol::new(&env, "dispute_reward_updated"),),
            (caller, old_reward, reward),
        );
    }

    pub fn get_dispute_reputation_reward(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeReputationReward)
            .unwrap_or(DEFAULT_DISPUTE_REPUTATION_REWARD)
    }

    /// Configure the flat reputation penalty applied to the driver when a
    /// dispute is split (`resolve_dispute_split_funds`). Mirrors
    /// `set_dispute_reputation_penalty`: admin-only and bounded by
    /// `MAX_DISPUTE_REPUTATION_PENALTY` (Issue #462).
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_dispute_split_penalty(env: Env, caller: Address, penalty: u32) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if penalty > MAX_DISPUTE_REPUTATION_PENALTY {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        let old_penalty = Self::get_dispute_split_penalty(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::DisputeSplitPenalty, &penalty);
        env.events().publish(
            (Symbol::new(&env, "dispute_split_penalty_updated"),),
            (caller, old_penalty, penalty),
        );
    }

    pub fn get_dispute_split_penalty(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeSplitPenalty)
            .unwrap_or(DEFAULT_DISPUTE_REPUTATION_SPLIT_PENALTY)
    }

    pub fn get_dispute_resolution_limit(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeResolutionLimit)
            .unwrap_or(0)
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_dispute_resolution_limit(env: Env, caller: Address, new_limit: u64) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        // Same floor init enforces: never let the resolution window drop below
        // one day, or a dispute could be force-resolved out from under the
        // parties before they have any chance to reach a verdict.
        if new_limit < MIN_DISPUTE_RESOLUTION_LIMIT {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        let old_limit = Self::get_dispute_resolution_limit(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::DisputeResolutionLimit, &new_limit);
        // #382: emit event so off-chain indexers can track admin changes to
        // this security-relevant parameter.
        env.events().publish(
            (Symbol::new(&env, "dispute_resolution_limit_updated"),),
            (caller, old_limit, new_limit),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn update_dispute_time_limit(env: Env, caller: Address, new_limit: u64) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        // The one-day floor is an invariant, not just a construction-time check:
        // enforce it here too, or an admin could set it to 0 and close the
        // post-delivery dispute window for every delivery in a single call.
        if new_limit < MIN_DISPUTE_TIME_LIMIT {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }
        let old_limit = Self::get_dispute_time_limit(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::DisputeTimeLimit, &new_limit);
        env.events().publish(
            (Symbol::new(&env, "dispute_time_limit_updated"),),
            (caller, old_limit, new_limit),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn raise_dispute(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();

        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );

        // Verify the caller is sender, recipient, or driver
        if caller != delivery.sender
            && caller != delivery.recipient
            && Some(caller.clone()) != delivery.driver
        {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        // Verify state and time limit
        match delivery.status {
            DeliveryStatus::Delivered => {
                let delivered_at = delivery.delivered_at.unwrap_or(0);
                let current_time = env.ledger().timestamp();
                let dispute_limit = Self::get_dispute_time_limit(env.clone());
                if current_time > delivered_at.saturating_add(dispute_limit) {
                    panic_with_error!(&env, FaniLabError::InvalidState);
                }
                // Call delivery contract to transition to Disputed
                let _: () = env.invoke_contract(
                    &delivery_contract_addr,
                    &Symbol::new(&env, "raise_dispute"),
                    soroban_sdk::vec![&env, caller.into_val(&env), delivery_id.into_val(&env)],
                );
            }
            DeliveryStatus::Active | DeliveryStatus::InTransit => {
                // Call delivery contract to transition to Disputed and pause escrow
                let _: () = env.invoke_contract(
                    &delivery_contract_addr,
                    &Symbol::new(&env, "raise_dispute"),
                    soroban_sdk::vec![&env, caller.into_val(&env), delivery_id.into_val(&env)],
                );
            }
            _ => {
                panic_with_error!(&env, FaniLabError::InvalidState);
            }
        }

        let escrow_addr = Self::get_escrow_contract(env.clone());
        let _: () = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "freeze_funds"),
            soroban_sdk::vec![
                &env,
                env.current_contract_address().into_val(&env),
                u64::from(delivery_id).into_val(&env),
            ],
        );

        let dispute_key = DataKey::Dispute(delivery_id);
        if env.storage().persistent().has(&dispute_key) {
            panic_with_error!(&env, FaniLabError::DuplicateDelivery);
        }

        let dispute = DisputeCase {
            delivery_id,
            status: DisputeStatus::Open,
            raised_at: env.ledger().timestamp(),
            raised_by: caller.clone(),
            evidence_hashes: Vec::new(&env),
            resolved_at: None,
            resolved_by: None,
        };

        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );
        // #380: extend instance TTL so contract configuration (contract pointers,
        // limits) does not archive while the contract is actively being used.
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        // Issue #313: maintain enumeration index so admins can page through
        // all disputes without prior knowledge of delivery IDs.
        dispute_index_push(&env, delivery_id);

        env.events().publish(
            (events::dispute_raised(&env), delivery_id),
            DisputeRaisedEvent {
                delivery_id: u64::from(delivery_id),
                caller,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn add_evidence_hash(
        env: Env,
        caller: Address,
        delivery_id: DeliveryId,
        evidence_hash: BytesN<32>,
    ) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );

        if caller != delivery.sender
            && caller != delivery.recipient
            && Some(caller.clone()) != delivery.driver
        {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        // Enforce the cap per submitting party, and reject a hash this party has
        // already submitted. A shared cap let one party fill the whole budget
        // with duplicate/junk hashes and lock the counterparty out for the life
        // of the dispute; a per-party quota removes that race entirely.
        let mut party_count: u32 = 0;
        for entry in dispute.evidence_hashes.iter() {
            if entry.submitter == caller {
                if entry.hash == evidence_hash {
                    panic_with_error!(&env, FaniLabError::InvalidState);
                }
                party_count += 1;
            }
        }
        if party_count >= MAX_EVIDENCE_HASHES_PER_PARTY {
            panic_with_error!(&env, FaniLabError::LimitExceeded);
        }

        dispute.evidence_hashes.push_back(EvidenceEntry {
            submitter: caller.clone(),
            hash: evidence_hash.clone(),
        });
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::evidence_added(&env), delivery_id),
            (caller, delivery_id, evidence_hash),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn resolve_dispute_refund_sender(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();
        require_escrow_not_paused(&env);
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Issue #211: reject a non-Paused escrow before any state mutation or
        // cross-contract side effect (the reputation adjustment below).
        Self::require_escrow_paused(&env, delivery_id);

        dispute.status = DisputeStatus::ResolvedRefund;
        dispute.resolved_at = Some(env.ledger().timestamp());
        dispute.resolved_by = Some(caller.clone());
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );
        let driver = delivery
            .driver
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        let penalty = Self::get_dispute_reputation_penalty(env.clone());

        if let Some(reputation_addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::IdentityReputationContract)
        {
            let _: () = env.invoke_contract(
                &reputation_addr,
                &Symbol::new(&env, "decrease_reputation"),
                soroban_sdk::vec![
                    &env,
                    env.current_contract_address().into_val(&env),
                    driver.clone().into_val(&env),
                    penalty.into_val(&env),
                ],
            );
        }

        let escrow_addr = Self::get_escrow_contract(env.clone());

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "resolve_dispute"),
            soroban_sdk::vec![
                &env,
                caller.into_val(&env),
                u64::from(delivery_id).into_val(&env),
                false.into_val(&env),
            ],
        );

        env.events().publish(
            (events::dispute_resolved_refund(&env), delivery_id),
            DisputeResolvedRefundEvent {
                delivery_id: u64::from(delivery_id),
                caller,
                driver,
                penalty,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn resolve_dispute_split_funds(
        env: Env,
        caller: Address,
        delivery_id: DeliveryId,
        sender_share_bps: u32,
    ) {
        caller.require_auth();
        require_escrow_not_paused(&env);
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Issue #473: validate sender_share_bps before any state mutation or
        // cross-contract call. Although escrow_contract::resolve_dispute_split
        // also checks this bound, a value > 10000 here would cause
        // sender_amount > record.amount, draining the protocol's pooled
        // balance into the sender address. Fail fast in the dispute contract
        // so the escrow state is never touched when the input is invalid.
        if sender_share_bps > 10000 {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Issue #211: reject a non-Paused escrow before any state mutation or
        // cross-contract side effect, via the same shared precondition used by
        // the other two resolution entry points.
        Self::require_escrow_paused(&env, delivery_id);

        dispute.status = DisputeStatus::Split;
        dispute.resolved_at = Some(env.ledger().timestamp());
        dispute.resolved_by = Some(caller.clone());
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let escrow_addr = Self::get_escrow_contract(env.clone());

        // Apply a partial reputation penalty to the driver for a split outcome
        let split_penalty = Self::get_dispute_split_penalty(env.clone());
        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );
        if let Some(driver) = delivery.driver {
            if let Some(reputation_addr) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::IdentityReputationContract)
            {
                let _: () = env.invoke_contract(
                    &reputation_addr,
                    &Symbol::new(&env, "decrease_reputation"),
                    soroban_sdk::vec![
                        &env,
                        env.current_contract_address().into_val(&env),
                        driver.clone().into_val(&env),
                        split_penalty.into_val(&env),
                    ],
                );
            }
        }
        let _: () = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "resolve_dispute_split"),
            soroban_sdk::vec![
                &env,
                caller.into_val(&env),
                u64::from(delivery_id).into_val(&env),
                sender_share_bps.into_val(&env),
            ],
        );

        env.events().publish(
            (events::dispute_resolved_split(&env), delivery_id),
            DisputeResolvedSplitEvent {
                delivery_id: u64::from(delivery_id),
                caller,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn resolve_dispute_pay_driver(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();
        require_escrow_not_paused(&env);
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Issue #211: reject a non-Paused escrow before any state mutation or
        // cross-contract side effect (the reputation adjustment below).
        Self::require_escrow_paused(&env, delivery_id);

        dispute.status = DisputeStatus::ResolvedPayout;
        dispute.resolved_at = Some(env.ledger().timestamp());
        dispute.resolved_by = Some(caller.clone());
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let escrow_addr = Self::get_escrow_contract(env.clone());

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "resolve_dispute"),
            soroban_sdk::vec![
                &env,
                caller.into_val(&env),
                u64::from(delivery_id).into_val(&env),
                true.into_val(&env),
            ],
        );

        // Award the driver a flat reputation credit when they are vindicated.
        // This uses `award_reputation`, not `increase_reputation`: a dispute
        // ruling is not a delivery completion, so it must not increment
        // `deliveries_completed` (that would double-count if the delivery is
        // later confirmed) and must not derive points from cargo attributes.
        // The reputation contract caps the resulting score at its maximum.
        // The credit amount is the admin-configurable reward (Issue #462).
        let reward = Self::get_dispute_reputation_reward(env.clone());
        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );
        if let Some(driver) = delivery.driver {
            if let Some(reputation_addr) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::IdentityReputationContract)
            {
                let _: () = env.invoke_contract(
                    &reputation_addr,
                    &Symbol::new(&env, "award_reputation"),
                    soroban_sdk::vec![
                        &env,
                        env.current_contract_address().into_val(&env),
                        driver.clone().into_val(&env),
                        reward.into_val(&env),
                    ],
                );
            }
        }

        env.events().publish(
            (events::dispute_resolved_payout(&env), delivery_id),
            DisputeResolvedPayoutEvent {
                delivery_id: u64::from(delivery_id),
                caller,
            },
        );
    }

    /// Force-resolve a dispute that has been Open past the configured resolution window.
    /// Any party (sender, recipient, or driver) may call this once the window has elapsed.
    /// Applies a 50/50 default split as the automatic fallback outcome.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn force_resolve_dispute(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();
        require_escrow_not_paused(&env);

        let dispute_key = DataKey::Dispute(delivery_id);
        let dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound));

        // Check 1: Dispute must be Open
        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Check 2: Verify caller is a party to the delivery
        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery: shared_types::DeliveryRecord = env.invoke_contract(
            &delivery_contract_addr,
            &Symbol::new(&env, "get_delivery"),
            soroban_sdk::vec![&env, delivery_id.into_val(&env)],
        );
        if caller != delivery.sender
            && caller != delivery.recipient
            && Some(caller.clone()) != delivery.driver
        {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        // Check 3: Verify the resolution window has elapsed
        let resolution_limit = Self::get_dispute_resolution_limit(env.clone());
        let current_time = env.ledger().timestamp();
        if current_time <= dispute.raised_at.saturating_add(resolution_limit) {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // Check 4: Verify escrow is Paused (precondition before state mutation)
        let escrow_addr = Self::get_escrow_contract(env.clone());
        let escrow: EscrowRecord = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "get_escrow"),
            soroban_sdk::vec![&env, u64::from(delivery_id).into_val(&env)],
        );
        if escrow.status != EscrowStatus::Paused {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        // All checks passed; now apply effects following checks-effects-interactions pattern
        const DEFAULT_SENDER_SHARE_BPS: u32 = 5000;
        let mut updated_dispute = dispute.clone();
        updated_dispute.status = DisputeStatus::Split;
        updated_dispute.resolved_at = Some(current_time);
        updated_dispute.resolved_by = Some(caller.clone());
        env.storage().persistent().set(&dispute_key, &updated_dispute);
        env.storage().persistent().extend_ttl(
            &dispute_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );
        // #380: extend instance TTL so contract configuration remains alive
        // under active usage (force_resolve_dispute is a high-traffic entry point).
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        // Perform external interactions
        // Pass this contract's address as the caller so the escrow contract's
        // require_admin check succeeds; the actual party (sender/recipient/driver)
        // only needs to authorize this call, not the subsequent escrow call.

        // #381: apply the same split reputation penalty as the standard
        // resolve_dispute_split_funds path — a forced split must not let the
        // driver evade the reputational consequence of an unresolved dispute.
        if let Some(driver) = delivery.driver {
            if let Some(reputation_addr) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::IdentityReputationContract)
            {
                let _: () = env.invoke_contract(
                    &reputation_addr,
                    &Symbol::new(&env, "decrease_reputation"),
                    soroban_sdk::vec![
                        &env,
                        env.current_contract_address().into_val(&env),
                        driver.clone().into_val(&env),
                        DISPUTE_REPUTATION_SPLIT_PENALTY.into_val(&env),
                    ],
                );
            }
        }

        let _: () = env.invoke_contract(
            &escrow_addr,
            &Symbol::new(&env, "resolve_dispute_split"),
            soroban_sdk::vec![
                &env,
                env.current_contract_address().into_val(&env),
                u64::from(delivery_id).into_val(&env),
                DEFAULT_SENDER_SHARE_BPS.into_val(&env),
            ],
        );

        env.events().publish(
            (Symbol::new(&env, "dispute_force_resolved"), delivery_id),
            (delivery_id, DEFAULT_SENDER_SHARE_BPS),
        );
    }

    /// Returns `true` if a dispute record exists for the given delivery ID,
    /// `false` otherwise.  Never panics for an unknown ID.  No authorization
    /// is required.  Use this to check presence before calling `get_dispute`
    /// if you want to avoid the panic that accessor raises for missing records
    /// (Issue #312).
    pub fn has_dispute(env: Env, delivery_id: DeliveryId) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Dispute(delivery_id))
    }

    /// Return a page of dispute delivery IDs from the enumeration index
    /// (Issue #313).  `offset` is the zero-based start position; `limit` is
    /// the maximum number of IDs to return (capped at 100).  The caller
    /// can resolve each returned ID to a full `DisputeCase` via `get_dispute`.
    ///
    /// The index is append-only and ordered by raise time.  Status filtering
    /// is left to the caller: load each `DisputeCase` and inspect
    /// `DisputeCase.status` to distinguish `Open` from resolved entries.
    ///
    /// No authorization is required.
    pub fn get_disputes_page(
        env: Env,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<DeliveryId> {
        let len: u32 = env
            .storage()
            .instance()
            .get(&DataKey::DisputeIndexLen)
            .unwrap_or(0);
        let mut out = soroban_sdk::Vec::new(&env);
        let cap = limit.min(100);
        let end = len.min(offset.saturating_add(cap));
        for i in offset.min(len)..end {
            let page: soroban_sdk::Vec<DeliveryId> = env
                .storage()
                .instance()
                .get(&DataKey::DisputeIndex(i / DISPUTE_INDEX_PAGE))
                .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
            if let Some(id) = page.get(i % DISPUTE_INDEX_PAGE) {
                out.push_back(id);
            }
        }
        out
    }

    /// Return the total number of disputes ever recorded (monotonically
    /// increasing).  Combined with `get_disputes_page` this lets callers
    /// paginate the full index without an extra sentinel call.
    pub fn get_dispute_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeIndexLen)
            .unwrap_or(0)
    }

    pub fn get_dispute(env: Env, delivery_id: DeliveryId) -> DisputeCase {
        let dispute_key = DataKey::Dispute(delivery_id);
        env.storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::DeliveryNotFound))
    }
}

#[cfg(test)]
mod test;

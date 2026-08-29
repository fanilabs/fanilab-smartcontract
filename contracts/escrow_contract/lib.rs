#![no_std]

use shared_types::{
    escrow_key, events, is_admin, ttl, EscrowRecord, EscrowStatus, FaniLabError, ProtocolConfig,
    StorageKey,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    IntoVal, Symbol,
};

pub mod constants {
    /// On-chain protocol version for the escrow contract's stored
    /// `ProtocolConfig` and `get_protocol_version()`.
    ///
    /// This is an **independent** identifier from the crate/package version
    /// declared in `contracts/*/Cargo.toml` and from the git release tag
    /// (`vX.Y.Z`). The crate version and release tag track the source and
    /// published-artifact history and must agree with each other (enforced by
    /// `.github/workflows/release.yml`). `PROTOCOL_VERSION` instead tracks the
    /// on-chain data/behaviour contract and only bumps when that changes in a
    /// way on-chain or off-chain consumers must branch on. A crate release
    /// with no protocol-observable change leaves this value untouched.
    pub const PROTOCOL_VERSION: u32 = 1;
    pub const MAX_BATCH_SIZE: u32 = 100;
    pub const DEFAULT_ESCROW_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days
    pub const MAX_PLATFORM_FEE_BPS: u32 = 1000;
    pub const SETTLEMENT_CONTRACT_TIMELOCK_SECONDS: u64 = 3 * 24 * 60 * 60; // 3 days
    /// Default time a recipient has, after confirming delivery (i.e. after
    /// `mark_holdback_escrow` moves an escrow into `Holdback`), to either
    /// release it to the driver or raise a dispute before the driver may
    /// claim it permissionlessly via `release_expired_holdback`.
    /// Admin-configurable via `set_holdback_window`, subject to
    /// `MIN_HOLDBACK_WINDOW_SECONDS` (Issue #192).
    pub const DEFAULT_HOLDBACK_WINDOW_SECONDS: u64 = 3 * 24 * 60 * 60; // 3 days
    /// Minimum permitted holdback window. Mirrors the
    /// `MIN_DISPUTE_TIME_LIMIT` precedent in `dispute_resolution_contract`:
    /// it stops an admin from configuring a window so short it defeats the
    /// recipient's realistic opportunity to release or dispute the escrow
    /// before the driver can claim it unilaterally.
    pub const MIN_HOLDBACK_WINDOW_SECONDS: u64 = 24 * 60 * 60; // 1 day
}

fn require_admin(env: &Env, caller: &Address) {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&StorageKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, FaniLabError::NotInitialized));
    if *caller != stored_admin {
        panic_with_error!(env, FaniLabError::Unauthorized);
    }
}

fn is_protocol_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

fn require_not_paused(env: &Env) {
    if is_protocol_paused(env) {
        panic_with_error!(env, FaniLabError::ProtocolPaused);
    }
}

fn load_protocol_config(env: &Env) -> ProtocolConfig {
    env.storage()
        .instance()
        .get(&StorageKey::ProtocolConfig)
        .unwrap_or_else(|| panic_with_error!(env, FaniLabError::NotInitialized))
}

fn save_protocol_config(env: &Env, config: &ProtocolConfig) {
    env.storage()
        .instance()
        .set(&StorageKey::ProtocolConfig, config);
}

fn calculate_fee(amount: i128, platform_fee_bps: u32) -> i128 {
    amount.saturating_mul(platform_fee_bps as i128) / 10_000
}

/// Validates a volume tier list before it is written to storage.
///
/// `get_effective_fee_bps` walks the stored tiers in reverse and returns on
/// the first entry whose `volume_threshold` the sender meets, which is only
/// correct if the list is sorted strictly ascending by `volume_threshold`.
/// This also rejects duplicate thresholds, since a strictly ascending
/// sequence cannot repeat a value. Each tier's `discount_bps` is bounded by
/// `MAX_PLATFORM_FEE_BPS` so a misconfigured tier can never be worse than a
/// silently-saturated 0% fee or an out-of-range value.
fn validate_volume_tiers(env: &Env, tiers: &soroban_sdk::Vec<VolumeTier>) {
    let mut prev_threshold: Option<u32> = None;
    for i in 0..tiers.len() {
        let tier = tiers.get(i).unwrap();
        if tier.discount_bps > constants::MAX_PLATFORM_FEE_BPS {
            panic_with_error!(env, EscrowError::InvalidFee);
        }
        if let Some(prev) = prev_threshold {
            if tier.volume_threshold <= prev {
                panic_with_error!(env, EscrowError::InvalidFee);
            }
        }
        prev_threshold = Some(tier.volume_threshold);
    }
}

fn get_effective_fee_bps(env: &Env, base_fee_bps: u32, sender_volume: u32) -> u32 {
    let tiers: Option<soroban_sdk::Vec<VolumeTier>> =
        env.storage().persistent().get(&DataKey::VolumeTiers);

    if let Some(tier_list) = tiers {
        for i in (0..tier_list.len()).rev() {
            if let Some(tier) = tier_list.get(i) {
                if sender_volume >= tier.volume_threshold {
                    return base_fee_bps.saturating_sub(tier.discount_bps);
                }
            }
        }
    }

    base_fee_bps
}

fn get_settlement_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::SettlementContract)
}

fn get_holdback_window(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::HoldbackWindow)
        .unwrap_or(constants::DEFAULT_HOLDBACK_WINDOW_SECONDS)
}

fn get_fleet_management_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get(&DataKey::FleetManagementContract)
}

fn get_identity_reputation_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get(&DataKey::IdentityReputationContract)
}

fn payout_driver(
    env: &Env,
    token: &Address,
    driver: &Address,
    amount: i128,
    fleet_management_addr: Option<&Address>,
    fleet_id: Option<u64>,
    payout_address: Option<Address>,
) {
    if amount <= 0 {
        return;
    }

    let payout_was_snapshotted = payout_address.is_some();
    let mut payout_address = payout_address.unwrap_or_else(|| driver.clone());

    if !payout_was_snapshotted {
        if let (Some(fleet_addr), Some(fid)) = (fleet_management_addr, fleet_id) {
            let treasury: Address = env.invoke_contract(
                fleet_addr,
                &Symbol::new(env, "get_payout_address"),
                soroban_sdk::vec![env, driver.into_val(env), fid.into_val(env)],
            );
            payout_address = treasury;
        }
    }

    if let Some(settlement_addr) = get_settlement_contract(env) {
        let preferred_asset: Option<Address> = env.invoke_contract(
            &settlement_addr,
            &Symbol::new(env, "get_driver_preference"),
            soroban_sdk::vec![env, driver.into_val(env)],
        );

        if let Some(preferred_asset) = preferred_asset {
            if preferred_asset != token.clone() {
                let slippage_tolerance_bps: u32 = load_protocol_config(env).slippage_tolerance_bps;
                let min_amount_out =
                    amount.saturating_mul(10000 - slippage_tolerance_bps as i128) / 10000;
                let _: () = env.invoke_contract(
                    &settlement_addr,
                    &Symbol::new(env, "execute_settlement_swap"),
                    soroban_sdk::vec![
                        env,
                        env.current_contract_address().into_val(env),
                        token.into_val(env),
                        preferred_asset.into_val(env),
                        payout_address.into_val(env),
                        amount.into_val(env),
                        min_amount_out.into_val(env),
                    ],
                );
                return;
            }
        }
    }

    token::Client::new(env, token).transfer(
        &env.current_contract_address(),
        &payout_address,
        &amount,
    );
}

fn settle_escrow_funds(
    env: &Env,
    delivery_id: u64,
    record: &EscrowRecord,
    fleet_management_addr: Option<Address>,
) {
    let platform_fee_bps: u32 = env
        .storage()
        .instance()
        .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
        .map(|config| config.platform_fee_bps)
        .unwrap_or(0);
    let platform_fee = calculate_fee(record.amount, platform_fee_bps);
    let driver_amount = record.amount.saturating_sub(platform_fee);

    payout_driver(
        env,
        &record.token,
        &record.driver,
        driver_amount,
        fleet_management_addr.as_ref(),
        record.fleet_id,
        env.storage()
            .persistent()
            .get(&DataKey::EscrowPayoutAddress(delivery_id)),
    );

    if platform_fee > 0 {
        let admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .expect("Not initialized");
        token::Client::new(env, &record.token).transfer(
            &env.current_contract_address(),
            &admin,
            &platform_fee,
        );
    }
}

/// Shared settlement logic for moving an escrow out of `Holdback` into
/// `Released` and paying the driver (minus the platform fee). Used by both
/// the recipient/admin-initiated `release_holdback_escrow` and the
/// permissionless `release_expired_holdback` (Issue #192) — the only
/// difference between the two call sites is which authorization/timing
/// checks are performed before this runs. Returns `(driver_amount,
/// platform_fee)` so each caller can emit its own event.
fn settle_holdback_escrow(env: &Env, delivery_id: u64, mut record: EscrowRecord) -> (i128, i128) {
    // Balance verification guard: confirm contract holds sufficient funds before transfer
    let contract_balance =
        token::Client::new(env, &record.token).balance(&env.current_contract_address());
    if contract_balance < record.amount {
        panic_with_error!(env, EscrowError::InsufficientFunds);
    }

    let base_fee_bps: u32 = env
        .storage()
        .instance()
        .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
        .map(|config| config.platform_fee_bps)
        .unwrap_or(0);

    let sender_volume = EscrowContract::get_sender_volume(env.clone(), record.sender.clone());
    let effective_fee_bps = get_effective_fee_bps(env, base_fee_bps, sender_volume);
    let platform_fee = calculate_fee(record.amount, effective_fee_bps);
    let driver_amount = record.amount.saturating_sub(platform_fee);

    let sender_volume_key = DataKey::SenderVolume(record.sender.clone());
    env.storage()
        .persistent()
        .set(&sender_volume_key, &sender_volume.saturating_add(1));

    // Effects (state) are committed before the interaction (transfer)
    // below, per checks-effects-interactions.
    record.status = EscrowStatus::Released;
    save_escrow(env, delivery_id, &record);

    let total_locked_key = DataKey::TotalLocked(record.token.clone());
    let current_total: i128 = env
        .storage()
        .persistent()
        .get(&total_locked_key)
        .unwrap_or(0);
    env.storage().persistent().set(
        &total_locked_key,
        &current_total.saturating_sub(record.amount),
    );

    let fleet_management = get_fleet_management_contract(env);
    settle_escrow_funds(env, delivery_id, &record, fleet_management);

    (driver_amount, platform_fee)
}

fn save_escrow(env: &Env, delivery_id: u64, record: &EscrowRecord) {
    let key = escrow_key(delivery_id);
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(
        &key,
        ttl::LEDGER_TTL_THRESHOLD,
        ttl::LEDGER_TTL_EXTEND_TO,
    );
}

fn load_escrow(env: &Env, delivery_id: u64) -> EscrowRecord {
    let key = escrow_key(delivery_id);
    let record: EscrowRecord = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| panic_with_error!(env, EscrowError::DeliveryNotFound));
    env.storage().persistent().extend_ttl(
        &key,
        ttl::LEDGER_TTL_THRESHOLD,
        ttl::LEDGER_TTL_EXTEND_TO,
    );
    record
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    PendingAdmin,
    SettlementContract,
    PendingSettlementContract,
    EscrowIndex(Address, u32, u32),
    EscrowIndexLen(Address, u32),
    EscrowPayoutAddress(u64),
    Paused,
    FleetManagementContract,
    DisputeResolutionContract,
    IdentityReputationContract,
    /// Track total locked value per token
    TotalLocked(Address),
    /// Track sender volume (number of completed deliveries)
    SenderVolume(Address),
    /// Store tier configuration (volume threshold -> discount bps)
    VolumeTiers,
    /// Admin-configurable window (seconds) an escrow may sit in `Holdback`
    /// before `release_expired_holdback` may settle it to the driver
    /// permissionlessly. Falls back to `DEFAULT_HOLDBACK_WINDOW_SECONDS`
    /// when unset (Issue #192).
    HoldbackWindow,
}

const INDEX_PAGE: u32 = 64;
#[rustfmt::skip]
fn index_push(env: &Env, owner: &Address, kind: u32, id: u64) {
    let len_key = DataKey::EscrowIndexLen(owner.clone(), kind);
    let len: u32 = env.storage().persistent().get(&len_key).unwrap_or(0);
    let key = DataKey::EscrowIndex(owner.clone(), kind, len / INDEX_PAGE);
    let mut page: soroban_sdk::Vec<u64> = env.storage().persistent().get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(env));
    page.push_back(id);
    env.storage().persistent().set(&key, &page);
    env.storage().persistent().set(&len_key, &(len + 1));
}

#[rustfmt::skip]
fn index_page(env: &Env, owner: Address, kind: u32, offset: u32, limit: u32) -> soroban_sdk::Vec<u64> {
    let len: u32 = env.storage().persistent()
        .get(&DataKey::EscrowIndexLen(owner.clone(), kind)).unwrap_or(0);
    let mut out = soroban_sdk::Vec::new(env);
    let end = len.min(offset.saturating_add(limit.min(100)));
    for i in offset.min(len)..end {
        let page: soroban_sdk::Vec<u64> = env.storage().persistent()
            .get(&DataKey::EscrowIndex(owner.clone(), kind, i / INDEX_PAGE))
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));
        if let Some(id) = page.get(i % INDEX_PAGE) { out.push_back(id); }
    }
    out
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum EscrowError {
    InvalidState = 1,
    DeliveryNotFound = 2,
    InsufficientFunds = 3,
    DuplicateDelivery = 4,
    InvalidFee = 5,
    InvalidToken = 6,
    InvalidAmount = 7,
    NoPendingSettlementChange = 8,
    TimelockNotElapsed = 9,
    InvalidDriver = 10,
    InvalidParties = 11,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeUpdated {
    pub old_fee: u32,
    pub new_fee: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolInitialized {
    pub admin: Address,
    pub token: Address,
    pub platform_fee_bps: u32,
    pub protocol_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementContractUpdated {
    pub old_address: Option<Address>,
    pub new_address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingSettlementContract {
    pub settlement_contract: Address,
    pub activates_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettlementContractChangeProposed {
    pub old_address: Option<Address>,
    pub proposed_address: Address,
    pub activates_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VolumeTier {
    pub volume_threshold: u32,
    pub discount_bps: u32,
}

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn init(env: Env, admin: Address, token: Address, platform_fee_bps: u32) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        if platform_fee_bps > constants::MAX_PLATFORM_FEE_BPS {
            panic_with_error!(&env, EscrowError::InvalidFee);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
        save_protocol_config(
            &env,
            &ProtocolConfig {
                token: token.clone(),
                platform_fee_bps,
                protocol_version: constants::PROTOCOL_VERSION,
                slippage_tolerance_bps: 500, // Default 5% slippage tolerance
            },
        );

        env.events().publish(
            (events::protocol_initialized(&env),),
            ProtocolInitialized {
                admin,
                token,
                platform_fee_bps,
                protocol_version: constants::PROTOCOL_VERSION,
            },
        );

        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn update_platform_fee(env: Env, admin: Address, new_fee_bps: u32) {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        admin.require_auth();
        if new_fee_bps > constants::MAX_PLATFORM_FEE_BPS {
            panic_with_error!(&env, EscrowError::InvalidFee);
        }
        let mut config = load_protocol_config(&env);
        let old_fee = config.platform_fee_bps;
        config.platform_fee_bps = new_fee_bps;
        save_protocol_config(&env, &config);
        env.events().publish(
            (events::fee_updated(&env),),
            FeeUpdated {
                old_fee,
                new_fee: new_fee_bps,
            },
        );

        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);
    }

    pub fn get_platform_fee(env: Env) -> u32 {
        load_protocol_config(&env).platform_fee_bps
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn get_token(env: Env) -> Address {
        load_protocol_config(&env).token
    }

    pub fn get_protocol_version(env: Env) -> u32 {
        load_protocol_config(&env).protocol_version
    }

    pub fn update_slippage_tolerance(env: Env, admin: Address, new_slippage_bps: u32) {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        admin.require_auth();
        if new_slippage_bps > 10000 {
            panic_with_error!(&env, EscrowError::InvalidFee);
        }
        let mut config = load_protocol_config(&env);
        config.slippage_tolerance_bps = new_slippage_bps;
        save_protocol_config(&env, &config);
    }

    pub fn get_slippage_tolerance(env: Env) -> u32 {
        load_protocol_config(&env).slippage_tolerance_bps
    }

    /// Propose a new settlement_contract address. The change does not take
    /// effect immediately: it becomes eligible for confirmation only after
    /// `SETTLEMENT_CONTRACT_TIMELOCK_SECONDS` have elapsed, via
    /// `confirm_settlement_contract`. This prevents a compromised or
    /// malicious admin key from silently redirecting every driver payout to
    /// an attacker-controlled contract with no warning. Proposing again
    /// before confirmation overwrites the pending change and restarts the
    /// timelock.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_settlement_contract(env: Env, admin: Address, settlement_contract: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        let old_address = get_settlement_contract(&env);

        let activates_at = env
            .ledger()
            .timestamp()
            .saturating_add(constants::SETTLEMENT_CONTRACT_TIMELOCK_SECONDS);
        let pending = PendingSettlementContract {
            settlement_contract: settlement_contract.clone(),
            activates_at,
        };
        env.storage()
            .instance()
            .set(&DataKey::PendingSettlementContract, &pending);
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        env.events().publish(
            (events::settlement_contract_proposed(&env),),
            SettlementContractChangeProposed {
                old_address,
                proposed_address: settlement_contract,
                activates_at,
            },
        );
    }

    /// Apply a previously proposed settlement_contract change once its
    /// timelock has elapsed.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn confirm_settlement_contract(env: Env, admin: Address) {
        admin.require_auth();
        require_admin(&env, &admin);

        let pending: PendingSettlementContract = env
            .storage()
            .instance()
            .get(&DataKey::PendingSettlementContract)
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::NoPendingSettlementChange));

        if env.ledger().timestamp() < pending.activates_at {
            panic_with_error!(&env, EscrowError::TimelockNotElapsed);
        }

        let old_address = get_settlement_contract(&env);
        env.storage()
            .instance()
            .set(&DataKey::SettlementContract, &pending.settlement_contract);
        env.storage()
            .instance()
            .remove(&DataKey::PendingSettlementContract);
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);

        env.events().publish(
            (events::settlement_contract_updated(&env),),
            SettlementContractUpdated {
                old_address,
                new_address: pending.settlement_contract,
            },
        );
    }

    /// Return the pending settlement_contract change, if any, so off-chain
    /// clients can display the upcoming payout-routing change during its
    /// timelock window.
    pub fn get_pending_settlement_contract(env: Env) -> Option<PendingSettlementContract> {
        env.storage()
            .instance()
            .get(&DataKey::PendingSettlementContract)
    }

    pub fn get_settlement_contract(env: Env) -> Option<Address> {
        get_settlement_contract(&env)
    }

    // Issue #90: allow an admin to unset a previously configured settlement_contract.
    pub fn clear_settlement_contract(env: Env, admin: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .remove(&DataKey::SettlementContract);
        env.storage()
            .instance()
            .remove(&DataKey::PendingSettlementContract);
    }

    pub fn set_fleet_management_contract(env: Env, admin: Address, fleet_contract: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::FleetManagementContract, &fleet_contract);
    }

    pub fn get_fleet_management_contract(env: Env) -> Option<Address> {
        get_fleet_management_contract(&env)
    }

    /// Issue #239: allow an admin to unset a previously configured
    /// fleet_management_contract, mirroring `clear_settlement_contract`.
    /// After clearing, `get_fleet_management_contract` returns `None` and the
    /// `if let (Some(fleet_addr), Some(fid))` guard in `payout_driver` falls
    /// through, so payouts for fleet-linked escrows go directly to the driver
    /// instead of routing through a cross-contract `get_payout_address` call.
    /// Clearing when nothing is configured is a no-op that still succeeds,
    /// matching `clear_settlement_contract`.
    ///
    /// Decision on `set_dispute_resolution_contract` (per issue #239): no
    /// `clear_dispute_resolution_contract` is provided. `freeze_funds` pins
    /// its caller to the configured dispute contract and reads that address
    /// via `NotInitialized`-on-absent, so unsetting it would permanently
    /// disable the protocol's ability to freeze a suspicious escrow. The
    /// intended remedy for a bad dispute contract is to repoint it with
    /// `set_dispute_resolution_contract`, not to remove the integration.
    pub fn clear_fleet_management_contract(env: Env, admin: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .remove(&DataKey::FleetManagementContract);
    }

    pub fn set_dispute_resolution_contract(env: Env, admin: Address, dispute_contract: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::DisputeResolutionContract, &dispute_contract);
    }

    pub fn get_dispute_resolution_contract(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::DisputeResolutionContract)
    }

    pub fn set_identity_reputation_contract(env: Env, admin: Address, identity_contract: Address) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage()
            .instance()
            .set(&DataKey::IdentityReputationContract, &identity_contract);
    }

    pub fn get_identity_contract(env: Env) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::IdentityReputationContract)
    }

    pub fn propose_admin(env: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if stored_admin != current_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::InvalidState));
        if pending != new_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        let old_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        env.storage().instance().set(&StorageKey::Admin, &new_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);
        env.storage()
            .instance()
            .extend_ttl(ttl::LEDGER_TTL_THRESHOLD, ttl::LEDGER_TTL_EXTEND_TO);
        env.events()
            .publish((events::admin_transferred(&env),), (old_admin, new_admin));
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_paused(env: Env, admin: Address, paused: bool) {
        admin.require_auth();
        require_admin(&env, &admin);
        env.storage().instance().set(&DataKey::Paused, &paused);
        env.events().publish(
            (events::protocol_pause_status_changed(&env),),
            (admin, paused),
        );
    }

    pub fn is_paused(env: Env) -> bool {
        is_protocol_paused(&env)
    }

    // ── Escrow lifecycle ──────────────────────────────────────────────────────

    #[allow(deprecated)]
    // events().publish() is deprecated in SDK 27.0.0 but still functional
    // 7 domain parameters (+ env) are all independently meaningful to callers;
    // bundling them into a struct would be a breaking change to a public,
    // already-integrated entry point for no safety benefit.
    #[allow(clippy::too_many_arguments)]
    pub fn create_escrow(
        env: Env,
        sender: Address,
        recipient: Address,
        driver: Address,
        delivery_id: u64,
        token: Address,
        amount: i128,
        fleet_id: Option<u64>,
    ) {
        sender.require_auth();
        require_not_paused(&env);
        if sender == recipient {
            panic_with_error!(&env, EscrowError::InvalidParties);
        }
        if driver == sender || driver == recipient {
            panic_with_error!(&env, EscrowError::InvalidDriver);
        }
        if amount <= 0 {
            panic_with_error!(&env, EscrowError::InvalidAmount);
        }
        if env.storage().persistent().has(&escrow_key(delivery_id)) {
            panic_with_error!(&env, EscrowError::DuplicateDelivery);
        }
        let config = load_protocol_config(&env);
        if token != config.token {
            panic_with_error!(&env, EscrowError::InvalidToken);
        }
        let payout_address: Option<Address> = if let (Some(fleet_addr), Some(fid)) =
            (get_fleet_management_contract(&env), fleet_id)
        {
            Some(env.invoke_contract(
                &fleet_addr,
                &Symbol::new(&env, "get_payout_address"),
                soroban_sdk::vec![&env, driver.clone().into_val(&env), fid.into_val(&env)],
            ))
        } else {
            None
        };

        token::Client::new(&env, &token).transfer(&sender, env.current_contract_address(), &amount);
        let record_token_clone = token.clone();
        let created_at = env.ledger().timestamp();
        let expires_at = created_at.saturating_add(constants::DEFAULT_ESCROW_EXPIRY_SECONDS);
        save_escrow(
            &env,
            delivery_id,
            &EscrowRecord {
                delivery_id,
                sender: sender.clone(),
                recipient: recipient.clone(),
                driver: driver.clone(),
                token,
                amount,
                status: EscrowStatus::Locked,
                created_at,
                expires_at: Some(expires_at),
                disputed_by: None,
                disputed_at: None,
                holdback_started_at: None,
                fleet_id,
            },
        );
        if let Some(payout_address) = payout_address {
            let payout_key = DataKey::EscrowPayoutAddress(delivery_id);
            env.storage().persistent().set(&payout_key, &payout_address);
            env.storage().persistent().extend_ttl(
                &payout_key,
                ttl::LEDGER_TTL_THRESHOLD,
                ttl::LEDGER_TTL_EXTEND_TO,
            );
        }

        index_push(&env, &sender, 0, delivery_id);
        index_push(&env, &recipient, 1, delivery_id);
        index_push(&env, &driver, 2, delivery_id);
        /* Legacy indexes retained for pre-mainnet compatibility.
        let sender_key = DataKey::EscrowsBySender(sender.clone());
        let mut sender_escrows: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&sender_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        sender_escrows.push_back(delivery_id);
        env.storage().persistent().set(&sender_key, &sender_escrows);
        env.storage().persistent().extend_ttl(
            &sender_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let recipient_key = DataKey::EscrowsByRecipient(recipient.clone());
        let mut recipient_escrows: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        recipient_escrows.push_back(delivery_id);
        env.storage()
            .persistent()
            .set(&recipient_key, &recipient_escrows);
        env.storage().persistent().extend_ttl(
            &recipient_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        let driver_key = DataKey::EscrowsByDriver(driver.clone());
        let mut driver_escrows: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&driver_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
        driver_escrows.push_back(delivery_id);
        env.storage().persistent().set(&driver_key, &driver_escrows);
        env.storage().persistent().extend_ttl(
            &driver_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );
        */

        let token_for_tracking = record_token_clone.clone();
        let total_locked_key = DataKey::TotalLocked(token_for_tracking.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&total_locked_key, &current_total.saturating_add(amount));

        env.events().publish(
            (events::escrow_funded(&env),),
            shared_types::EscrowFundedEvent {
                delivery_id,
                sender,
                token: record_token_clone,
                amount,
            },
        );
    }

    /// Create multiple escrows in a single transaction.  Sender must authorize.
    /// Takes a list of (delivery_id, driver, amount, fleet_id) tuples. All escrows use the
    /// configured protocol token. Returns the count of escrows created.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn create_escrows_batch(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        escrow_list: soroban_sdk::Vec<(u64, Address, i128, Option<u64>)>,
    ) -> u32 {
        sender.require_auth();
        require_not_paused(&env);

        if sender == recipient {
            panic_with_error!(&env, EscrowError::InvalidParties);
        }

        if escrow_list.len() > constants::MAX_BATCH_SIZE {
            panic_with_error!(&env, EscrowError::BatchTooLarge);
        }

        /* Legacy index batching (replaced by bounded pages).
        let sender_key = DataKey::EscrowsBySender(sender.clone());
        let mut sender_escrows: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&sender_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));

        let recipient_key = DataKey::EscrowsByRecipient(recipient.clone());
        let mut recipient_escrows: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&recipient_key)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env));

        let mut driver_indexes: soroban_sdk::Map<DataKey, soroban_sdk::Vec<u64>> =
            soroban_sdk::Map::new(&env);
        */

        let mut count = 0u32;
        let mut batch_total: i128 = 0;
        for i in 0..escrow_list.len() {
            if let Some((delivery_id, driver, amount)) = escrow_list.get(i) {
                if driver == sender || driver == recipient {
                    panic_with_error!(&env, EscrowError::InvalidDriver);
                }
                if amount <= 0 {
                    panic_with_error!(&env, EscrowError::InvalidAmount);
                }
                if env.storage().persistent().has(&escrow_key(delivery_id)) {
                    panic_with_error!(&env, EscrowError::DuplicateDelivery);
                }
                token::Client::new(&env, &token).transfer(
                    &sender,
                    env.current_contract_address(),
                    &amount,
                );
                batch_total = batch_total.saturating_add(amount);
                let created_at = env.ledger().timestamp();
                let expires_at =
                    created_at.saturating_add(constants::DEFAULT_ESCROW_EXPIRY_SECONDS);
                save_escrow(
                    &env,
                    delivery_id,
                    &EscrowRecord {
                        delivery_id,
                        sender: sender.clone(),
                        recipient: recipient.clone(),
                        driver: driver.clone(),
                        token: token.clone(),
                        amount,
                        status: EscrowStatus::Locked,
                        created_at,
                        expires_at: Some(expires_at),
                        disputed_by: None,
                        disputed_at: None,
                        holdback_started_at: None,
                        fleet_id,
                    },
                );
                /*
                sender_escrows.push_back(delivery_id);
                recipient_escrows.push_back(delivery_id);
                */
                index_push(&env, &sender, 0, delivery_id);
                index_push(&env, &recipient, 1, delivery_id);
                index_push(&env, &driver, 2, delivery_id);

                /* Legacy driver index.
                let driver_key = DataKey::EscrowsByDriver(driver.clone());
                if !driver_indexes.contains_key(driver_key.clone()) {
                    let existing: soroban_sdk::Vec<u64> = env
                        .storage()
                        .persistent()
                        .get(&driver_key)
                        .unwrap_or_else(|| soroban_sdk::Vec::new(&env));
                    driver_indexes.set(driver_key.clone(), existing);
                }
                if let Some(mut driver_escrows_vec) = driver_indexes.get(driver_key.clone()) {
                    driver_escrows_vec.push_back(delivery_id);
                    driver_indexes.set(driver_key, driver_escrows_vec);
                }
                */

                env.events().publish(
                    (events::escrow_funded(&env),),
                    shared_types::EscrowFundedEvent {
                        delivery_id,
                        sender: sender.clone(),
                        token: token.clone(),
                        amount,
                    },
                );
                count += 1;
            }
        }

        /* Legacy index flush.
        env.storage().persistent().set(&sender_key, &sender_escrows);
        env.storage().persistent().extend_ttl(
            &sender_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.storage()
            .persistent()
            .set(&recipient_key, &recipient_escrows);
        env.storage().persistent().extend_ttl(
            &recipient_key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        for (driver_key, driver_escrows_vec) in driver_indexes.iter() {
            env.storage()
                .persistent()
                .set(&driver_key, &driver_escrows_vec);
            env.storage().persistent().extend_ttl(
                &driver_key,
                ttl::LEDGER_TTL_THRESHOLD,
                ttl::LEDGER_TTL_EXTEND_TO,
            );
        }
        */

        // Maintain the TotalLocked fund-accounting invariant (Issue #188):
        // batch-created escrows must count toward TotalLocked exactly like
        // single-escrow creates, otherwise sweep_untracked_balance would treat
        // the batch funds as untracked surplus and drain them. A batch shares
        // one token, so accumulate in the loop and do a single read-modify-
        // write here instead of one storage round-trip per element.
        let total_locked_key = DataKey::TotalLocked(token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_add(batch_total),
        );

        count
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn mark_holdback_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        require_not_paused(&env);
        let mut record = load_escrow(&env, delivery_id);
        let recipient_authorized = caller == record.recipient;
        if !recipient_authorized {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if record.status != EscrowStatus::Locked {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let timestamp = env.ledger().timestamp();
        record.status = EscrowStatus::Holdback;
        record.holdback_started_at = Some(timestamp);
        save_escrow(&env, delivery_id, &record);
        env.events().publish(
            (Symbol::new(&env, "escrow_holdback_marked"), delivery_id),
            (caller, timestamp),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn release_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        require_not_paused(&env);
        let mut record = load_escrow(&env, delivery_id);
        let admin_authorized = is_admin(&env, &caller);
        let recipient_authorized = caller == record.recipient;
        if !admin_authorized && !recipient_authorized {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if record.status != EscrowStatus::Locked {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        // Balance verification guard: confirm contract holds sufficient funds before transfer
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }

        let base_fee_bps: u32 = env
            .storage()
            .instance()
            .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
            .map(|config| config.platform_fee_bps)
            .unwrap_or(0);

        let sender_volume = Self::get_sender_volume(env.clone(), record.sender.clone());
        let effective_fee_bps = get_effective_fee_bps(&env, base_fee_bps, sender_volume);
        let platform_fee = calculate_fee(record.amount, effective_fee_bps);
        let driver_amount = record.amount.saturating_sub(platform_fee);

        let sender_volume_key = DataKey::SenderVolume(record.sender.clone());
        env.storage()
            .persistent()
            .set(&sender_volume_key, &sender_volume.saturating_add(1));

        // Effects (state) are committed before the interaction (transfer)
        // below, per checks-effects-interactions.
        record.status = EscrowStatus::Released;
        save_escrow(&env, delivery_id, &record);

        let total_locked_key = DataKey::TotalLocked(record.token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_sub(record.amount),
        );

        let fleet_management = get_fleet_management_contract(&env);
        settle_escrow_funds(&env, delivery_id, &record, fleet_management);

        env.events().publish(
            (events::escrow_released(&env),),
            shared_types::EscrowReleasedEvent {
                delivery_id,
                driver: record.driver,
                amount: driver_amount,
                platform_fee,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn refund_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        require_not_paused(&env);
        let mut record = load_escrow(&env, delivery_id);
        let admin_authorized = is_admin(&env, &caller);
        let sender_authorized = caller == record.sender;
        // Two states are past the point where the sender may unilaterally
        // reclaim the funds, so both are admin-only refunds:
        //
        // - `Paused`: the escrow is under active dispute. Letting the sender
        //   self-refund here would bypass dispute resolution entirely and let
        //   them race the outcome the admin/dispute_resolution_contract is
        //   meant to decide (Issue #93).
        // - `Holdback`: the recipient has already confirmed delivery (this is
        //   the only transition into `Holdback`, via `mark_holdback_escrow`),
        //   so the driver has performed and the funds are earmarked for them
        //   pending `release_holdback_escrow`. A sender self-refund here would
        //   let them take back the full amount after the goods were delivered
        //   and after the driver was credited reputation for the delivery,
        //   leaving the driver unpaid. Clawing an escrow back after delivery
        //   confirmation is an arbitration outcome, not a sender privilege.
        //
        // In both cases an admin may still refund, so the protocol keeps its
        // recovery path; only the unilateral sender path is closed.
        if record.status == EscrowStatus::Paused || record.status == EscrowStatus::Holdback {
            if !admin_authorized {
                panic_with_error!(&env, FaniLabError::Unauthorized);
            }
        } else if !admin_authorized && !sender_authorized {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if record.status != EscrowStatus::Locked
            && record.status != EscrowStatus::Paused
            && record.status != EscrowStatus::Holdback
        {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        // Balance verification guard: confirm contract holds sufficient funds before transfer
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }

        // If refunding from Holdback (delivery was confirmed and driver credited reputation),
        // reverse the reputation gain before transferring funds. Use the same penalty mechanism
        // as resolve_dispute_refund_sender for consistency with the accounting invariant:
        // "reputation credited for a delivery implies the driver was paid for it".
        let is_holdback_refund = record.status == EscrowStatus::Holdback;
        if is_holdback_refund {
            if let Some(reputation_addr) = get_identity_reputation_contract(&env) {
                // Query dispute_resolution_contract for the penalty, or use a default if unavailable.
                // This ensures consistency across all refund paths.
                let penalty: u32 = if let Some(dispute_addr) = 
                    env.storage().instance().get::<DataKey, Address>(&DataKey::DisputeResolutionContract)
                {
                    env.invoke_contract(
                        &dispute_addr,
                        &Symbol::new(&env, "get_dispute_reputation_penalty"),
                        soroban_sdk::vec![&env],
                    )
                } else {
                    10u32 // DEFAULT_DISPUTE_REPUTATION_PENALTY
                };

                use soroban_sdk::IntoVal;
                let _: () = env.invoke_contract(
                    &reputation_addr,
                    &Symbol::new(&env, "decrease_reputation"),
                    soroban_sdk::vec![
                        &env,
                        env.current_contract_address().into_val(&env),
                        record.driver.clone().into_val(&env),
                        penalty.into_val(&env),
                    ],
                );
            }
        }

        // Effects (state) are committed before the interaction (transfer)
        // below, per checks-effects-interactions.
        record.status = EscrowStatus::Refunded;
        save_escrow(&env, delivery_id, &record);

        let total_locked_key = DataKey::TotalLocked(record.token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_sub(record.amount),
        );

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.sender,
            &record.amount,
        );

        env.events().publish(
            (events::escrow_refunded(&env),),
            shared_types::EscrowRefundedEvent {
                delivery_id,
                sender: record.sender,
                amount: record.amount,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn raise_dispute(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        require_not_paused(&env);
        let mut record = load_escrow(&env, delivery_id);
        if caller != record.sender && caller != record.recipient && caller != record.driver {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        // Accept both Locked (pre-delivery dispute) and Holdback (post-delivery
        // dispute, after recipient has confirmed but before escrow is released).
        // This unblocks the Delivered → Disputed transition described in issue
        // #193: after confirm_delivery the escrow is in Holdback, not Locked,
        // so rejecting Holdback here made post-delivery disputes unreachable.
        // freeze_funds already treats Locked and Holdback identically, which
        // establishes the precedent that the transition is sound.  Terminal
        // states (Released, Refunded, Split) are still rejected.
        if record.status != EscrowStatus::Locked && record.status != EscrowStatus::Holdback {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let timestamp = env.ledger().timestamp();
        record.status = EscrowStatus::Paused;
        record.disputed_by = Some(caller.clone());
        record.disputed_at = Some(timestamp);
        save_escrow(&env, delivery_id, &record);
        env.events().publish(
            (events::delivery_disputed(&env),),
            shared_types::DeliveryDisputedEvent {
                delivery_id,
                reporter: caller,
                timestamp,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn resolve_dispute(env: Env, caller: Address, delivery_id: u64, release_to_driver: bool) {
        caller.require_auth();
        require_not_paused(&env);
        require_admin(&env, &caller);
        let mut record = load_escrow(&env, delivery_id);
        if record.status != EscrowStatus::Paused {
            panic_with_error!(&env, EscrowError::InvalidState);
        }

        // Balance verification guard: confirm contract holds sufficient funds
        // before any state mutation or transfer.  Runs for both branches so
        // that a shortfall produces the typed InsufficientFunds error in all
        // cases rather than an opaque token-level failure deep inside
        // settle_escrow_funds (release branch) or the token transfer (refund
        // branch).  Matches the pattern used by release_escrow,
        // refund_escrow, release_holdback_escrow, resolve_dispute_split, and
        // reclaim_expired_escrow.  See issue #194.
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }

        // Checks + effects (state) are resolved per-branch first; the actual
        // fund transfer (interaction) happens only after all state below is
        // committed, per checks-effects-interactions.
        let mut platform_fee: i128 = 0;
        let fleet_management: Option<Address> = if release_to_driver {
            let base_fee_bps: u32 = env
                .storage()
                .instance()
                .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
                .map(|config| config.platform_fee_bps)
                .unwrap_or(0);

            let sender_volume = Self::get_sender_volume(env.clone(), record.sender.clone());
            let effective_fee_bps = get_effective_fee_bps(&env, base_fee_bps, sender_volume);
            platform_fee = calculate_fee(record.amount, effective_fee_bps);

            let sender_volume_key = DataKey::SenderVolume(record.sender.clone());
            env.storage()
                .persistent()
                .set(&sender_volume_key, &sender_volume.saturating_add(1));

            record.status = EscrowStatus::Released;
            get_fleet_management_contract(&env)
        } else {
            record.status = EscrowStatus::Refunded;
            None
        };

        save_escrow(&env, delivery_id, &record);

        let total_locked_key = DataKey::TotalLocked(record.token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_sub(record.amount),
        );

        if release_to_driver {
            settle_escrow_funds(&env, delivery_id, &record, fleet_management);
        } else {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.sender,
                &record.amount,
            );
        }

        env.events().publish(
            (events::dispute_resolved(&env),),
            shared_types::DisputeResolvedEvent {
                delivery_id,
                resolver: caller,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn resolve_dispute_split(
        env: Env,
        caller: Address,
        delivery_id: u64,
        sender_share_bps: u32,
    ) {
        caller.require_auth();
        require_not_paused(&env);
        // Allow calls from either the admin or the dispute resolution contract.
        // This enables both admin-initiated resolutions and automatic force-resolve
        // timeouts (where the dispute_resolution_contract calls on its own behalf).
        let is_caller_admin = is_admin(&env, &caller);
        let is_caller_dispute_contract = if let Some(dispute_contract) =
            env.storage()
                .instance()
                .get::<_, Address>(&DataKey::DisputeResolutionContract)
        {
            caller == dispute_contract
        } else {
            false
        };
        if !is_caller_admin && !is_caller_dispute_contract {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if sender_share_bps > 10000 {
            panic_with_error!(&env, EscrowError::InvalidFee);
        }
        let mut record = load_escrow(&env, delivery_id);
        if record.status != EscrowStatus::Paused {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }

        let sender_amount = record.amount.saturating_mul(sender_share_bps as i128) / 10000;
        let driver_share_amount = record.amount.saturating_sub(sender_amount);

        let base_fee_bps: u32 = env
            .storage()
            .instance()
            .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
            .map(|config| config.platform_fee_bps)
            .unwrap_or(0);
        let sender_volume = Self::get_sender_volume(env.clone(), record.sender.clone());
        let effective_fee_bps = get_effective_fee_bps(&env, base_fee_bps, sender_volume);
        let platform_fee = calculate_fee(driver_share_amount, effective_fee_bps);
        let net_driver_amount = driver_share_amount.saturating_sub(platform_fee);

        let sender_volume_key = DataKey::SenderVolume(record.sender.clone());
        env.storage()
            .persistent()
            .set(&sender_volume_key, &sender_volume.saturating_add(1));

        // Effects (state) are committed before the interactions (transfers)
        // below, per checks-effects-interactions.
        record.status = EscrowStatus::Split;
        save_escrow(&env, delivery_id, &record);

        let total_locked_key = DataKey::TotalLocked(record.token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_sub(record.amount),
        );

        if sender_amount > 0 {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.sender,
                &sender_amount,
            );
        }
        if net_driver_amount > 0 {
            let fleet_management = get_fleet_management_contract(&env);
            payout_driver(
                &env,
                &record.token,
                &record.driver,
                net_driver_amount,
                fleet_management.as_ref(),
                record.fleet_id,
                env.storage()
                    .persistent()
                    .get(&DataKey::EscrowPayoutAddress(delivery_id)),
            );
        }
        if platform_fee > 0 {
            let admin: Address = env
                .storage()
                .instance()
                .get(&StorageKey::Admin)
                .expect("Not initialized");
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &admin,
                &platform_fee,
            );
        }

        env.events().publish(
            (events::dispute_resolved(&env),),
            shared_types::DisputeResolvedEvent {
                delivery_id,
                resolver: caller,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn release_holdback_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        require_not_paused(&env);
        let record = load_escrow(&env, delivery_id);
        let admin_authorized = is_admin(&env, &caller);
        let recipient_authorized = caller == record.recipient;
        if !admin_authorized && !recipient_authorized {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        if record.status != EscrowStatus::Holdback {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let driver = record.driver.clone();
        let (driver_amount, platform_fee) = settle_holdback_escrow(&env, delivery_id, record);

        env.events().publish(
            (events::escrow_released(&env), delivery_id),
            (driver, driver_amount, platform_fee),
        );
    }

    /// Permissionless counterpart to `release_holdback_escrow`: once an
    /// escrow has sat in `Holdback` for at least `get_holdback_window()`
    /// seconds since `mark_holdback_escrow` set `holdback_started_at`,
    /// *anyone* may call this to settle it to the driver. Without this, a
    /// recipient who never calls `release_holdback_escrow` (and no dispute is
    /// raised to freeze the escrow) could strand the driver's funds in
    /// `Holdback` indefinitely, since the only other exits require the
    /// recipient or an admin to act (Issue #192).
    ///
    /// A `Paused` escrow — including one frozen out of `Holdback` via
    /// `freeze_funds` — is unaffected: this only ever accepts `Holdback`, so
    /// once an escrow is frozen it falls back to admin arbitration exactly
    /// as before.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn release_expired_holdback(env: Env, delivery_id: u64) {
        require_not_paused(&env);
        let record = load_escrow(&env, delivery_id);
        if record.status != EscrowStatus::Holdback {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        // `mark_holdback_escrow` is the only path into `Holdback` and always
        // sets this, but fall back to a typed rejection rather than a panic
        // on unwrap if that invariant is ever violated.
        let holdback_started_at = record
            .holdback_started_at
            .unwrap_or_else(|| panic_with_error!(&env, EscrowError::InvalidState));
        let deadline = holdback_started_at.saturating_add(get_holdback_window(&env));
        if env.ledger().timestamp() < deadline {
            panic_with_error!(&env, EscrowError::TimelockNotElapsed);
        }
        let driver = record.driver.clone();
        let (driver_amount, platform_fee) = settle_holdback_escrow(&env, delivery_id, record);

        env.events().publish(
            (Symbol::new(&env, "holdback_expired_released"), delivery_id),
            (driver, driver_amount, platform_fee),
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_holdback_window(env: Env, admin: Address, new_window_seconds: u64) {
        admin.require_auth();
        require_admin(&env, &admin);
        if new_window_seconds < constants::MIN_HOLDBACK_WINDOW_SECONDS {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        env.storage()
            .instance()
            .set(&DataKey::HoldbackWindow, &new_window_seconds);
        env.events().publish(
            (Symbol::new(&env, "holdback_window_updated"),),
            (admin, new_window_seconds),
        );
    }

    /// Current holdback window (in seconds), falling back to
    /// `DEFAULT_HOLDBACK_WINDOW_SECONDS` when the admin has never set one.
    pub fn get_holdback_window(env: Env) -> u64 {
        get_holdback_window(&env)
    }

    pub fn get_escrow(env: Env, delivery_id: u64) -> EscrowRecord {
        let key = escrow_key(delivery_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(env, EscrowError::DeliveryNotFound))
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn freeze_funds(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        // Intentionally NOT gated on require_not_paused: this only moves an
        // escrow into the Paused (disputed) state and never transfers funds,
        // so it remains available during a protocol pause — an admin should
        // still be able to freeze a suspicious escrow while the protocol is
        // paused for an unrelated incident. The caller is already restricted
        // to the configured dispute_resolution_contract below.
        let dispute_contract = env
            .storage()
            .instance()
            .get(&DataKey::DisputeResolutionContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if caller != dispute_contract {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        let mut record = load_escrow(&env, delivery_id);
        if record.status == EscrowStatus::Locked || record.status == EscrowStatus::Holdback {
            record.status = EscrowStatus::Paused;
            record.disputed_at = Some(env.ledger().timestamp());
            save_escrow(&env, delivery_id, &record);
            env.events().publish(
                (Symbol::new(&env, "funds_frozen"), delivery_id),
                (caller, env.ledger().timestamp()),
            );
        }
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn reclaim_expired_escrow(env: Env, delivery_id: u64) {
        require_not_paused(&env);
        let mut record = load_escrow(&env, delivery_id);
        if record.status != EscrowStatus::Locked {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        if let Some(expires_at) = record.expires_at {
            let current_timestamp = env.ledger().timestamp();
            if current_timestamp <= expires_at {
                panic_with_error!(&env, EscrowError::InvalidState);
            }
        } else {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }
        // Effects (state) are committed before the interaction (transfer)
        // below, per checks-effects-interactions.
        record.status = EscrowStatus::Refunded;
        save_escrow(&env, delivery_id, &record);

        let total_locked_key = DataKey::TotalLocked(record.token.clone());
        let current_total: i128 = env
            .storage()
            .persistent()
            .get(&total_locked_key)
            .unwrap_or(0);
        env.storage().persistent().set(
            &total_locked_key,
            &current_total.saturating_sub(record.amount),
        );

        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.sender,
            &record.amount,
        );

        env.events().publish(
            (events::escrow_refunded(&env),),
            shared_types::EscrowRefundedEvent {
                delivery_id,
                sender: record.sender,
                amount: record.amount,
            },
        );
    }

    /// Get all escrow delivery IDs for a sender.
    pub fn get_escrows_by_sender(env: Env, sender: Address) -> soroban_sdk::Vec<u64> {
        index_page(&env, sender, 0, 0, 100)
    }

    /// Get all escrow delivery IDs for a recipient.
    pub fn get_escrows_by_recipient(env: Env, recipient: Address) -> soroban_sdk::Vec<u64> {
        index_page(&env, recipient, 1, 0, 100)
    }

    /// Get all escrow delivery IDs for a driver.
    pub fn get_escrows_by_driver(env: Env, driver: Address) -> soroban_sdk::Vec<u64> {
        index_page(&env, driver, 2, 0, 100)
    }

    #[rustfmt::skip]
    pub fn get_escrows_page(env: Env, owner: Address, kind: u32, offset: u32, limit: u32) -> soroban_sdk::Vec<u64> {
        index_page(&env, owner, kind, offset, limit)
    }

    pub fn get_total_locked(env: Env, token: Address) -> i128 {
        let key = DataKey::TotalLocked(token);
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Returns the amount of untracked balance for a given token without executing
    /// a sweep. This is a read-only view function used for visibility and monitoring.
    /// 
    /// Untracked balance = contract_balance - total_locked. If the contract balance
    /// is less than or equal to total_locked, returns 0 (indicating nothing to sweep).
    ///
    /// This allows operators to:
    /// 1. Verify the sweep amount before executing sweep_untracked_balance
    /// 2. Monitor untracked balance as a key metric
    /// 3. Detect misclassified funds (issue #188)
    pub fn get_untracked_balance(env: Env, token: Address) -> i128 {
        let contract_balance =
            token::Client::new(&env, &token).balance(&env.current_contract_address());
        let total_locked = Self::get_total_locked(env, token);
        contract_balance.saturating_sub(total_locked)
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn sweep_untracked_balance(env: Env, admin: Address, token: Address, recipient: Address) -> i128 {
        admin.require_auth();
        require_admin(&env, &admin);
        require_not_paused(&env);

        let contract_balance =
            token::Client::new(&env, &token).balance(&env.current_contract_address());
        let total_locked = Self::get_total_locked(env.clone(), token.clone());

        if contract_balance <= total_locked {
            return 0;
        }

        let untracked_balance = contract_balance.saturating_sub(total_locked);
        token::Client::new(&env, &token).transfer(
            &env.current_contract_address(),
            &recipient,
            &untracked_balance,
        );

        env.events().publish(
            (Symbol::new(&env, "untracked_balance_swept"),),
            (token.clone(), untracked_balance, recipient),
        );

        untracked_balance
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn set_volume_tiers(env: Env, admin: Address, tiers: soroban_sdk::Vec<VolumeTier>) {
        admin.require_auth();
        require_admin(&env, &admin);
        validate_volume_tiers(&env, &tiers);

        env.storage()
            .persistent()
            .set(&DataKey::VolumeTiers, &tiers);
        env.events().publish(
            (Symbol::new(&env, "volume_tiers_updated"),),
            (admin, tiers.len()),
        );
    }

    pub fn get_volume_tiers(env: Env) -> soroban_sdk::Vec<VolumeTier> {
        env.storage()
            .persistent()
            .get(&DataKey::VolumeTiers)
            .unwrap_or_else(|| soroban_sdk::Vec::new(&env))
    }

    pub fn get_sender_volume(env: Env, sender: Address) -> u32 {
        let key = DataKey::SenderVolume(sender);
        env.storage().persistent().get(&key).unwrap_or(0)
    }
}

#[cfg(test)]
mod test;

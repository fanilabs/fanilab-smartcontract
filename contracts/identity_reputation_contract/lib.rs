#![no_std]

use shared_types::{
    events, is_admin, ttl, DriverProfile, DriverRegisteredEvent, FaniLabError,
    KycStatusUpdatedEvent, ReputationAwardedEvent, ReputationDecreasedEvent,
    ReputationIncreasedEvent, StorageKey, UserProfile, UserRegisteredEvent,
};
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ReputationConfig {
    pub base_points: u32,
    pub heavy_cargo_points: u32,
    pub fragile_points: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    UserProfile(Address),
    DriverProfile(Address),
    AuthorizedContract(Address),
    DeliveryContract,
    DisputeContract,
    EscrowContract,
    ReputationConfig,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DriverTier {
    Bronze,
    Silver,
    Gold,
}

const MAX_REPUTATION: u32 = 100;
#[rustfmt::skip]
fn reputation_up(score: u32, points: u32) -> u32 { score.saturating_add(points).min(MAX_REPUTATION) }
#[rustfmt::skip]
fn reputation_down(score: u32, points: u32) -> u32 { score.saturating_sub(points) }
const GOLD_TIER_THRESHOLD: u32 = 75;
// Enterprise eligibility is intentionally tied to reaching the Gold tier.
const ENTERPRISE_THRESHOLD: u32 = GOLD_TIER_THRESHOLD;
// Lower bound of the Silver tier: a driver scoring at or above this value (but
// below GOLD_TIER_THRESHOLD) is Silver; below it they are Bronze.
const SILVER_TIER_THRESHOLD: u32 = 50;
// A newly registered driver intentionally starts at the bottom of the Silver
// tier. Deriving the starting score from SILVER_TIER_THRESHOLD makes that policy
// explicit and keeps the two values coupled: the tier boundary and the starting
// score can only ever move together, so neither can silently reclassify every
// new driver relative to the other.
const INITIAL_REPUTATION_SCORE: u32 = SILVER_TIER_THRESHOLD;
const HEAVY_CARGO_GRAMS: u32 = 5000;
const DEFAULT_BASE_POINTS: u32 = 5;
const DEFAULT_HEAVY_CARGO_POINTS: u32 = 3;
const DEFAULT_FRAGILE_POINTS: u32 = 2;

fn require_escrow_not_paused(env: &Env) {
    let delivery_contract: Address = env
        .storage()
        .instance()
        .get(&DataKey::DeliveryContract)
        .unwrap_or_else(|| panic_with_error!(env, FaniLabError::NotInitialized));
    let escrow_contract: Address = env.invoke_contract(
        &delivery_contract,
        &soroban_sdk::Symbol::new(env, "get_escrow_contract"),
        soroban_sdk::vec![env],
    );
    let paused: bool = env.invoke_contract(
        &escrow_contract,
        &soroban_sdk::Symbol::new(env, "is_paused"),
        soroban_sdk::vec![env],
    );
    if paused {
        panic_with_error!(env, FaniLabError::ProtocolPaused);
    }
}

#[contract]
pub struct IdentityReputationContract;

#[contractimpl]
impl IdentityReputationContract {
    pub fn init(env: Env, admin: Address, delivery_contract: Address, dispute_contract: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::DeliveryContract, &delivery_contract);
        env.storage()
            .instance()
            .set(&DataKey::DisputeContract, &dispute_contract);

        // Register the initial two authorized contracts through the allowlist so
        // they can be revoked or rotated later without a contract migration.
        for contract_addr in [delivery_contract, dispute_contract] {
            let key = DataKey::AuthorizedContract(contract_addr);
            env.storage().persistent().set(&key, &true);
            env.storage().persistent().extend_ttl(
                &key,
                ttl::LEDGER_TTL_THRESHOLD,
                ttl::LEDGER_TTL_EXTEND_TO,
            );
        }
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn set_authorized_contract(
        env: Env,
        admin: Address,
        contract_addr: Address,
        authorized: bool,
    ) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        let key = DataKey::AuthorizedContract(contract_addr);
        if authorized {
            env.storage().persistent().set(&key, &true);
            env.storage().persistent().extend_ttl(
                &key,
                ttl::LEDGER_TTL_THRESHOLD,
                ttl::LEDGER_TTL_EXTEND_TO,
            );
        } else {
            env.storage().persistent().remove(&key);
        }
    }

    pub fn set_reputation_config(env: Env, admin: Address, config: ReputationConfig) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::ReputationConfig, &config);
    }

    pub fn get_reputation_config(env: Env) -> ReputationConfig {
        env.storage()
            .instance()
            .get(&DataKey::ReputationConfig)
            .unwrap_or(ReputationConfig {
                base_points: DEFAULT_BASE_POINTS,
                heavy_cargo_points: DEFAULT_HEAVY_CARGO_POINTS,
                fragile_points: DEFAULT_FRAGILE_POINTS,
            })
    }

    pub fn set_delivery_contract(env: Env, admin: Address, delivery_contract: Address) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::DeliveryContract, &delivery_contract);
    }

    pub fn set_dispute_contract(env: Env, admin: Address, dispute_contract: Address) {
        admin.require_auth();
        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::DisputeContract, &dispute_contract);
    }

    pub fn get_delivery_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::DeliveryContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn get_dispute_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::DisputeContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    pub fn is_authorized_contract(env: Env, contract_addr: Address) -> bool {
        let key = DataKey::AuthorizedContract(contract_addr);
        if env.storage().persistent().get(&key).unwrap_or(false) {
            env.storage().persistent().extend_ttl(
                &key,
                ttl::LEDGER_TTL_THRESHOLD,
                ttl::LEDGER_TTL_EXTEND_TO,
            );
            true
        } else {
            false
        }
    }

    pub fn has_driver_profile(env: Env, driver: Address) -> bool {
        let key = DataKey::DriverProfile(driver);
        env.storage()
            .persistent()
            .get::<_, DriverProfile>(&key)
            .is_some()
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn register_driver(env: Env, driver: Address) {
        driver.require_auth();
        require_escrow_not_paused(&env);
        let key = DataKey::DriverProfile(driver.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }

        let profile = DriverProfile {
            address: driver.clone(),
            deliveries_completed: 0,
            reputation_score: INITIAL_REPUTATION_SCORE,
            registered_at: env.ledger().timestamp(),
            kyc_verified: false,
            status: DriverStatus::Active,
        };

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::driver_registered(&env),),
            DriverRegisteredEvent { driver },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn register_user(env: Env, user: Address) -> UserProfile {
        user.require_auth();
        require_escrow_not_paused(&env);

        let registered_at = env.ledger().timestamp();

        let profile = UserProfile {
            address: user.clone(),
            registered_at,
        };

        let key = DataKey::UserProfile(user.clone());
        if env.storage().persistent().has(&key) {
            return env.storage().persistent().get(&key).unwrap();
        }

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::user_registered(&env),),
            UserRegisteredEvent { user },
        );

        profile
    }

    pub fn get_user_profile(env: Env, user: Address) -> UserProfile {
        let key = DataKey::UserProfile(user);
        let profile: UserProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));
        profile
    }

    pub fn has_user_profile(env: Env, user: Address) -> bool {
        let key = DataKey::UserProfile(user);
        env.storage().persistent().has(&key)
    }

    pub fn get_driver_profile(env: Env, driver: Address) -> DriverProfile {
        let key = DataKey::DriverProfile(driver);
        let profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));
        profile
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn update_driver_kyc_status(env: Env, admin: Address, driver: Address, kyc_verified: bool) {
        admin.require_auth();
        require_escrow_not_paused(&env);

        if !is_admin(&env, &admin) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        profile.kyc_verified = kyc_verified;

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::kyc_status_updated(&env),),
            KycStatusUpdatedEvent {
                driver,
                kyc_verified,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn increase_reputation(
        env: Env,
        caller: Address,
        driver: Address,
        delivery_id: u64,
        weight_grams: u32,
        fragile: bool,
    ) {
        require_escrow_not_paused(&env);
        if !Self::is_authorized_contract(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        caller.require_auth();

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        let config = Self::get_reputation_config(env.clone());

        let mut points: u32 = config.base_points;
        if weight_grams > HEAVY_CARGO_GRAMS {
            points += config.heavy_cargo_points;
        }
        if fragile {
            points += config.fragile_points;
        }

        profile.reputation_score = reputation_up(profile.reputation_score, points);
        profile.deliveries_completed += 1;

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::reputation_increased(&env),),
            ReputationIncreasedEvent {
                driver,
                delivery_id,
                points,
            },
        );
    }

    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn decrease_reputation(env: Env, caller: Address, driver: Address, points: u32) {
        require_escrow_not_paused(&env);
        if !Self::is_authorized_contract(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        caller.require_auth();

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        profile.reputation_score = reputation_down(profile.reputation_score, points);

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::reputation_decreased(&env),),
            ReputationDecreasedEvent { driver, points },
        );
    }

    /// Apply a flat reputation award to a driver, mirroring `decrease_reputation`.
    ///
    /// Unlike `increase_reputation`, this does **not** derive points from cargo
    /// weight/fragility and does **not** increment `deliveries_completed` — a
    /// dispute ruling in the driver's favour is not a delivery completion, and
    /// counting it as one would double-count if the delivery is later confirmed.
    /// The resulting score is still capped at `MAX_REPUTATION`.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional; tracked in SOROBAN_SDK_27_MIGRATION.md#event-system-migration (Issue #114)
    pub fn award_reputation(env: Env, caller: Address, driver: Address, points: u32) {
        if !Self::is_authorized_contract(env.clone(), caller.clone()) {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }
        caller.require_auth();

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        profile.reputation_score = (profile.reputation_score + points).min(MAX_REPUTATION);

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::reputation_awarded(&env),),
            ReputationAwardedEvent { driver, points },
        );
    }

    pub fn get_driver_tier(env: Env, driver: Address) -> DriverTier {
        let profile = Self::get_driver_profile(env, driver);
        let score = profile.reputation_score;
        if score >= GOLD_TIER_THRESHOLD {
            DriverTier::Gold
        } else if score >= SILVER_TIER_THRESHOLD {
            DriverTier::Silver
        } else {
            DriverTier::Bronze
        }
    }

    pub fn is_eligible_for_enterprise(env: Env, driver: Address) -> bool {
        let profile = Self::get_driver_profile(env, driver);
        profile.reputation_score >= ENTERPRISE_THRESHOLD
    }

    // ── Driver suspension lifecycle ───────────────────────────────────────────

    /// Suspend a registered driver.
    ///
    /// Sets `DriverProfile.status` to `DriverStatus::Suspended`.  The profile
    /// record is preserved — history (reputation, deliveries, KYC) is never
    /// erased.  This prevents a suspended driver from calling `register_driver`
    /// again to reset their score, since that function panics when a profile
    /// already exists.
    ///
    /// Gating `assign_driver` on suspension status is a follow-up task in
    /// `delivery_contract`.
    ///
    /// **Authorization:** Admin only.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional
    pub fn suspend_driver(env: Env, admin: Address, driver: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        if profile.status == DriverStatus::Suspended {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        profile.status = DriverStatus::Suspended;
        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::driver_suspended(&env),),
            DriverSuspendedEvent {
                driver,
                admin,
            },
        );
    }

    /// Reinstate a previously suspended driver.
    ///
    /// Sets `DriverProfile.status` back to `DriverStatus::Active`.  All
    /// accumulated reputation and delivery history is retained.
    ///
    /// **Authorization:** Admin only.
    #[allow(deprecated)] // events().publish() is deprecated in SDK 27.0.0 but still functional
    pub fn reinstate_driver(env: Env, admin: Address, driver: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::ProviderNotFound));

        if profile.status == DriverStatus::Active {
            panic_with_error!(&env, FaniLabError::InvalidState);
        }

        profile.status = DriverStatus::Active;
        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(
            &key,
            ttl::LEDGER_TTL_THRESHOLD,
            ttl::LEDGER_TTL_EXTEND_TO,
        );

        env.events().publish(
            (events::driver_reinstated(&env),),
            DriverReinstatedEvent {
                driver,
                admin,
            },
        );
    }

    /// Returns `true` if the driver's profile exists and is currently suspended.
    pub fn is_driver_suspended(env: Env, driver: Address) -> bool {
        let key = DataKey::DriverProfile(driver);
        let profile: Option<DriverProfile> = env.storage().persistent().get(&key);
        matches!(profile.map(|p| p.status), Some(DriverStatus::Suspended))
    }
}

#[cfg(test)]
mod test;

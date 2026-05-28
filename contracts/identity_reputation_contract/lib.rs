#![no_std]

use shared_types::{DriverProfile, SwiftChainError};
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    DriverProfile(Address),
    AuthorizedContract(Address),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DriverTier {
    Bronze,
    Silver,
    Gold,
}

const MAX_REPUTATION: u32 = 100;
const ENTERPRISE_THRESHOLD: u32 = 75;

#[contract]
pub struct IdentityReputationContract;

#[contractimpl]
impl IdentityReputationContract {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized))
    }

    pub fn set_authorized_contract(
        env: Env,
        admin: Address,
        contract_addr: Address,
        authorized: bool,
    ) {
        admin.require_auth();
        let stored_admin = Self::get_admin(env.clone());
        if admin != stored_admin {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        let key = DataKey::AuthorizedContract(contract_addr);
        if authorized {
            env.storage().persistent().set(&key, &true);
        } else {
            env.storage().persistent().remove(&key);
        }
    }

    pub fn is_authorized_contract(env: Env, contract_addr: Address) -> bool {
        let key = DataKey::AuthorizedContract(contract_addr);
        env.storage().persistent().get(&key).unwrap_or(false)
    }

    pub fn register_driver(env: Env, driver: Address) {
        driver.require_auth();
        let key = DataKey::DriverProfile(driver.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }

        let profile = DriverProfile {
            address: driver.clone(),
            deliveries_completed: 0,
            reputation_score: 50,
            registered_at: env.ledger().timestamp(),
            kyc_verified: false,
        };

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events()
            .publish((Symbol::new(&env, "driver_registered"),), (driver,));
    }

    pub fn get_driver_profile(env: Env, driver: Address) -> DriverProfile {
        let key = DataKey::DriverProfile(driver);
        let profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::ProviderNotFound));
        profile
    }

    pub fn update_driver_kyc_status(env: Env, admin: Address, driver: Address, kyc_verified: bool) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));

        if admin != stored_admin {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::ProviderNotFound));

        profile.kyc_verified = kyc_verified;

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "kyc_status_updated"),),
            (driver, kyc_verified),
        );
    }

    pub fn increase_reputation(
        env: Env,
        caller: Address,
        driver: Address,
        delivery_id: u64,
        weight_grams: u32,
        fragile: bool,
    ) {
        caller.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));

        let is_admin = caller == stored_admin;
        let is_authorized = Self::is_authorized_contract(env.clone(), caller.clone());

        if !is_admin && !is_authorized {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::ProviderNotFound));

        let mut points: u32 = 5;
        if weight_grams > 5000 {
            points += 3;
        }
        if fragile {
            points += 2;
        }

        profile.reputation_score = (profile.reputation_score + points).min(MAX_REPUTATION);
        profile.deliveries_completed += 1;

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "reputation_increased"),),
            (driver, delivery_id, points),
        );
    }

    pub fn decrease_reputation(
        env: Env,
        caller: Address,
        driver: Address,
        points: u32,
    ) {
        caller.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));

        let is_admin = caller == stored_admin;
        let is_authorized = Self::is_authorized_contract(env.clone(), caller.clone());

        if !is_admin && !is_authorized {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        let key = DataKey::DriverProfile(driver.clone());
        let mut profile: DriverProfile = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::ProviderNotFound));

        profile.reputation_score = profile.reputation_score.saturating_sub(points);

        env.storage().persistent().set(&key, &profile);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "reputation_decreased"),),
            (driver, points),
        );
    }

    pub fn get_driver_tier(env: Env, driver: Address) -> DriverTier {
        let profile = Self::get_driver_profile(env, driver);
        let score = profile.reputation_score;
        if score >= 75 {
            DriverTier::Gold
        } else if score >= 50 {
            DriverTier::Silver
        } else {
            DriverTier::Bronze
        }
    }

    pub fn is_eligible_for_enterprise(env: Env, driver: Address) -> bool {
        let profile = Self::get_driver_profile(env, driver);
        profile.reputation_score >= ENTERPRISE_THRESHOLD
    }
}

#[cfg(test)]
mod test;

#![no_std]

use shared_types::{DriverProfile, SwiftChainError};
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    DriverProfile(Address),
}

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

    pub fn register_driver(env: Env, driver: Address) {
        driver.require_auth();
        let key = DataKey::DriverProfile(driver.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }

        let profile = DriverProfile {
            address: driver.clone(),
            deliveries_completed: 0,
            reputation_score: 50, // default/base reputation score
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
}

#[cfg(test)]
mod test;

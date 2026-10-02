#![no_std]

use shared_types::{FaniLabError, StorageKey};

// Settlement contract for cross-border currency swaps
// This contract will handle currency conversions between different assets
// during escrow release for international deliveries.
//
// CRITICAL: This contract is currently a stub. Phase 3 implementation is required
// before mainnet deployment. See PRODUCTION_READINESS.md "Next Steps for Mainnet Launch."

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    EscrowContract,
    /// Per-driver preferred payout asset recorded via
    /// `set_driver_preference` (Issue #459).
    DriverPreference(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SettlementError {
    SwapNotImplemented = 1,
}

#[contract]
pub struct SettlementContract;

#[contractimpl]
impl SettlementContract {
    /// Initialize the settlement contract
    pub fn init(env: Env, admin: Address, escrow_contract: Address) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, FaniLabError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage().instance().set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
    }

    /// Get the settlement contract administrator.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized))
    }

    /// Record the asset a driver wants to be paid in.
    ///
    /// Any driver may set (or overwrite) their own preference; the caller must
    /// authenticate as the driver. Stored in persistent storage so the entry
    /// survives independently of the contract's instance entry, and its TTL is
    /// extended on every write (Issue #459).
    pub fn set_driver_preference(env: Env, driver: Address, to_token: Address) {
        driver.require_auth();

        let key = DataKey::DriverPreference(driver);
        env.storage().persistent().set(&key, &to_token);
        env.storage().persistent().extend_ttl(
            &key,
            shared_types::ttl::LEDGER_TTL_THRESHOLD,
            shared_types::ttl::LEDGER_TTL_EXTEND_TO,
        );
    }

    /// Get driver's preferred asset for payment
    pub fn get_driver_preference(env: Env, driver: Address) -> Option<Address> {
        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);
        let key = DataKey::DriverPreference(driver);
        env.storage().persistent().get(&key)
    }

    /// Execute asset swap and transfer to driver.
    ///
    /// PRODUCTION GUARD: This function will panic if called in production before
    /// Phase 3 implementation is complete. This is intentional to prevent silent
    /// no-op swaps on mainnet with unimplemented settlement logic.
    pub fn execute_settlement_swap(
        env: Env,
        caller: Address,
        _from_token: Address,
        _to_token: Address,
        _recipient: Address,
        _amount: i128,
        _min_amount_out: i128,
    ) {
        caller.require_auth();

        env.storage()
            .instance()
            .extend_ttl(shared_types::ttl::LEDGER_TTL_THRESHOLD, shared_types::ttl::LEDGER_TTL_EXTEND_TO);

        let escrow_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, FaniLabError::NotInitialized));

        if caller != escrow_contract {
            panic_with_error!(&env, FaniLabError::Unauthorized);
        }

        // CRITICAL GUARD: Prevent execution of stub function on mainnet.
        // Phase 3 settlement logic must be implemented before production use.
        panic_with_error!(&env, SettlementError::SwapNotImplemented);
    }
}

#[cfg(test)]
mod test;

#![no_std]

use shared_types::{
    escrow_key, events, DeliveryStatus, EscrowRecord, EscrowStatus, ProtocolConfig, StorageKey,
    SwiftChainError,
};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, Env,
    Symbol,
};

pub mod constants {
    pub const ESCROW_TTL_THRESHOLD: u32 = 518400;
    pub const ESCROW_TTL_EXTEND_TO: u32 = 518400;
    pub const PROTOCOL_VERSION: u32 = 1;
}

fn require_admin(env: &Env, caller: &Address) {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&StorageKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, SwiftChainError::NotInitialized));
    if *caller != stored_admin {
        panic_with_error!(env, SwiftChainError::Unauthorized);
    }
}

fn is_admin(env: &Env, caller: &Address) -> bool {
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&StorageKey::Admin)
        .expect("Not initialized");
    *caller == stored_admin
}

fn load_protocol_config(env: &Env) -> ProtocolConfig {
    env.storage()
        .instance()
        .get(&StorageKey::ProtocolConfig)
        .unwrap_or_else(|| panic_with_error!(env, SwiftChainError::NotInitialized))
}

fn save_protocol_config(env: &Env, config: &ProtocolConfig) {
    env.storage().instance().set(&StorageKey::ProtocolConfig, config);
}

fn calculate_fee(amount: i128, platform_fee_bps: u32) -> i128 {
    amount.saturating_mul(platform_fee_bps as i128) / 10_000
}

fn save_escrow(env: &Env, delivery_id: u64, record: &EscrowRecord) {
    let key = escrow_key(delivery_id);
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(
        &key,
        constants::ESCROW_TTL_THRESHOLD,
        constants::ESCROW_TTL_EXTEND_TO,
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
        constants::ESCROW_TTL_THRESHOLD,
        constants::ESCROW_TTL_EXTEND_TO,
    );
    record
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    PendingAdmin,
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

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    pub fn init(env: Env, admin: Address, token: Address, platform_fee_bps: u32) {
        if env.storage().instance().has(&StorageKey::Admin) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
        save_protocol_config(
            &env,
            &ProtocolConfig {
                token: token.clone(),
                platform_fee_bps,
                protocol_version: constants::PROTOCOL_VERSION,
            },
        );

        env.events().publish(
            (Symbol::new(&env, "ProtocolInitialized"),),
            ProtocolInitialized {
                admin,
                token,
                platform_fee_bps,
                protocol_version: constants::PROTOCOL_VERSION,
            },
        );
    }

    pub fn update_platform_fee(env: Env, admin: Address, new_fee_bps: u32) {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));
        if admin != stored_admin {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        admin.require_auth();
        if new_fee_bps > 1000 {
            panic_with_error!(&env, EscrowError::InvalidFee);
        }
        let mut config = load_protocol_config(&env);
        let old_fee = config.platform_fee_bps;
        config.platform_fee_bps = new_fee_bps;
        save_protocol_config(&env, &config);
        env.events().publish(
            (Symbol::new(&env, "FeeUpdated"),),
            FeeUpdated {
                old_fee,
                new_fee: new_fee_bps,
            },
        );
    }

    pub fn get_platform_fee(env: Env) -> u32 {
        load_protocol_config(&env).platform_fee_bps
    }

    pub fn get_status(_env: Env) -> DeliveryStatus {
        DeliveryStatus::Pending
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized))
    }

    pub fn get_token(env: Env) -> Address {
        load_protocol_config(&env).token
    }

    pub fn get_protocol_version(env: Env) -> u32 {
        load_protocol_config(&env).protocol_version
    }

    pub fn propose_admin(env: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));
        if stored_admin != current_admin {
            panic!("caller is not the admin");
        }
        env.storage()
            .instance()
            .set(&DataKey::PendingAdmin, &new_admin);
        env.storage().instance().extend_ttl(
            constants::ESCROW_TTL_THRESHOLD,
            constants::ESCROW_TTL_EXTEND_TO,
        );
    }

    pub fn accept_admin(env: Env, new_admin: Address) {
        new_admin.require_auth();
        let pending: Address = env
            .storage()
            .instance()
            .get(&DataKey::PendingAdmin)
            .expect("no pending admin");
        if pending != new_admin {
            panic!("caller is not the pending admin");
        }
        let old_admin: Address = env
            .storage()
            .instance()
            .get(&StorageKey::Admin)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized));
        env.storage().instance().set(&StorageKey::Admin, &new_admin);
        env.storage().instance().remove(&DataKey::PendingAdmin);
        env.storage().instance().extend_ttl(
            constants::ESCROW_TTL_THRESHOLD,
            constants::ESCROW_TTL_EXTEND_TO,
        );
        env.events().publish(
            (Symbol::new(&env, "AdminTransferred"),),
            (old_admin, new_admin),
        );
    }

    // ── Escrow lifecycle ──────────────────────────────────────────────────────

    pub fn create_escrow(
        env: Env,
        sender: Address,
        recipient: Address,
        driver: Address,
        delivery_id: u64,
        token: Address,
        amount: i128,
    ) {
        sender.require_auth();
        if env.storage().persistent().has(&escrow_key(delivery_id)) {
            panic_with_error!(&env, EscrowError::DuplicateDelivery);
        }
        token::Client::new(&env, &token).transfer(
            &sender,
            &env.current_contract_address(),
            &amount,
        );
        save_escrow(
            &env,
            delivery_id,
            &EscrowRecord {
                sender: sender.clone(),
                recipient: recipient.clone(),
                driver,
                token,
                amount,
                status: EscrowStatus::Locked,
                created_at: env.ledger().timestamp(),
                disputed_by: None,
                disputed_at: None,
            },
        );
        env.events().publish(
            (events::escrow_funded(&env), delivery_id),
            (sender, recipient, amount),
        );
    }

    pub fn release_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        let mut record = load_escrow(&env, delivery_id);
        let admin_authorized = is_admin(&env, &caller);
        let recipient_authorized = caller == record.recipient;
        if !admin_authorized && !recipient_authorized {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
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
        let platform_fee_bps: u32 = env
            .storage()
            .instance()
            .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
            .map(|config| config.platform_fee_bps)
            .unwrap_or(0);
        let platform_fee = calculate_fee(record.amount, platform_fee_bps);
        let driver_amount = record.amount.saturating_sub(platform_fee);

        if driver_amount > 0 {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.driver,
                &driver_amount,
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

        record.status = EscrowStatus::Released;
        save_escrow(&env, delivery_id, &record);
        env.events().publish(
            (events::escrow_released(&env), delivery_id),
            (record.driver, driver_amount, platform_fee),
        );
    }

    pub fn refund_escrow(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        let mut record = load_escrow(&env, delivery_id);
        let admin_authorized = is_admin(&env, &caller);
        let sender_authorized = caller == record.sender;
        if !admin_authorized && !sender_authorized {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        if record.status != EscrowStatus::Locked && record.status != EscrowStatus::Paused {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        // Balance verification guard: confirm contract holds sufficient funds before transfer
        let contract_balance =
            token::Client::new(&env, &record.token).balance(&env.current_contract_address());
        if contract_balance < record.amount {
            panic_with_error!(&env, EscrowError::InsufficientFunds);
        }
        token::Client::new(&env, &record.token).transfer(
            &env.current_contract_address(),
            &record.sender,
            &record.amount,
        );
        record.status = EscrowStatus::Refunded;
        save_escrow(&env, delivery_id, &record);
        env.events().publish(
            (events::escrow_refunded(&env), delivery_id),
            (record.sender, record.amount),
        );
    }

    pub fn raise_dispute(env: Env, caller: Address, delivery_id: u64) {
        caller.require_auth();
        let mut record = load_escrow(&env, delivery_id);
        if caller != record.sender && caller != record.recipient {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        if record.status != EscrowStatus::Locked {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        let timestamp = env.ledger().timestamp();
        record.status = EscrowStatus::Paused;
        record.disputed_by = Some(caller.clone());
        record.disputed_at = Some(timestamp);
        save_escrow(&env, delivery_id, &record);
        env.events().publish(
            (events::delivery_disputed(&env), delivery_id),
            (caller, timestamp),
        );
    }

    pub fn resolve_dispute(env: Env, caller: Address, delivery_id: u64, release_to_driver: bool) {
        caller.require_auth();
        require_admin(&env, &caller);
        let mut record = load_escrow(&env, delivery_id);
        if record.status != EscrowStatus::Paused {
            panic_with_error!(&env, EscrowError::InvalidState);
        }
        if release_to_driver {
            let platform_fee_bps: u32 = env
                .storage()
                .instance()
                .get::<_, ProtocolConfig>(&StorageKey::ProtocolConfig)
                .map(|config| config.platform_fee_bps)
                .unwrap_or(0);
            let platform_fee = calculate_fee(record.amount, platform_fee_bps);
            let driver_amount = record.amount.saturating_sub(platform_fee);

            if driver_amount > 0 {
                token::Client::new(&env, &record.token).transfer(
                    &env.current_contract_address(),
                    &record.driver,
                    &driver_amount,
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

            record.status = EscrowStatus::Released;
        } else {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.sender,
                &record.amount,
            );
            record.status = EscrowStatus::Refunded;
        }

        save_escrow(&env, delivery_id, &record);
        
        env.events().publish(
            (events::dispute_resolved(&env), delivery_id),
            (caller.clone(), caller),
        );
    }

    pub fn resolve_dispute_split(
        env: Env,
        caller: Address,
        delivery_id: u64,
        sender_share_bps: u32,
    ) {
        caller.require_auth();
        require_admin(&env, &caller);
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
        let driver_amount = record.amount.saturating_sub(sender_amount);

        if sender_amount > 0 {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.sender,
                &sender_amount,
            );
        }
        if driver_amount > 0 {
            token::Client::new(&env, &record.token).transfer(
                &env.current_contract_address(),
                &record.driver,
                &driver_amount,
            );
        }

        record.status = EscrowStatus::Refunded;
        save_escrow(&env, delivery_id, &record);

        env.events().publish(
            (events::dispute_resolved(&env), delivery_id),
            (caller.clone(), caller),
        );
    }

    pub fn get_escrow(env: Env, delivery_id: u64) -> EscrowRecord {
        if !env
            .storage()
            .persistent()
            .has(&escrow_key(delivery_id))
        {
            panic_with_error!(&env, EscrowError::DeliveryNotFound);
        }
        load_escrow(&env, delivery_id)
    }
}

#[cfg(test)]
mod test;

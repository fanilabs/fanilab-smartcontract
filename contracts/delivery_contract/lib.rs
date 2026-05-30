#![no_std]

use shared_types::SwiftChainError;
use shared_types::{DeliveryMetadata, DriverProfile};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Symbol,
};

pub type DeliveryId = u64;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Active,
    InTransit,
    Delivered,
    Cancelled,
    Disputed,
}

// Local DeliveryMetadata removed in favor of shared_types::DeliveryMetadata

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
#[derive(Clone)]
pub enum DataKey {
    Delivery(DeliveryId),
    DeliveryCounter,
    Admin,
    EscrowContract,
    DriverProfile(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DeliveryError {
    InvalidState = 1,
}

/// Validate whether a status transition is permitted by the delivery state machine.
///
/// Allowed transitions:
///   Pending   → Active, Cancelled
///   Active    → InTransit, Disputed, Cancelled
///   InTransit → Delivered, Disputed
///   Disputed  → Delivered, Cancelled
///   Delivered, Cancelled → (terminal, no transitions)
pub fn validate_transition(from: DeliveryStatus, to: DeliveryStatus) -> Result<(), DeliveryError> {
    let valid = match (&from, &to) {
        (DeliveryStatus::Pending, DeliveryStatus::Active) => true,
        (DeliveryStatus::Pending, DeliveryStatus::Cancelled) => true,
        (DeliveryStatus::Active, DeliveryStatus::InTransit) => true,
        (DeliveryStatus::Active, DeliveryStatus::Disputed) => true,
        (DeliveryStatus::Active, DeliveryStatus::Cancelled) => true,
        (DeliveryStatus::InTransit, DeliveryStatus::Delivered) => true,
        (DeliveryStatus::InTransit, DeliveryStatus::Disputed) => true,
        (DeliveryStatus::Disputed, DeliveryStatus::Delivered) => true,
        (DeliveryStatus::Disputed, DeliveryStatus::Cancelled) => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(DeliveryError::InvalidState)
    }
}

#[contract]
pub struct DeliveryContract;

#[contractimpl]
impl DeliveryContract {
    pub fn init(env: Env, admin: Address, escrow_contract: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::EscrowContract, &escrow_contract);
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &0u64);

        env.events().publish(
            (Symbol::new(&env, "DeliveryContractInitialized"),),
            (admin, escrow_contract),
        );
    }

    pub fn create_delivery(
        env: Env,
        sender: Address,
        recipient: Address,
        metadata: DeliveryMetadata,
    ) -> DeliveryId {
        sender.require_auth();

        let mut counter: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::DeliveryCounter)
            .unwrap_or(0);
        counter += 1;
        env.storage()
            .persistent()
            .set(&DataKey::DeliveryCounter, &counter);

        let delivery_id = counter;

        let record = DeliveryRecord {
            delivery_id,
            sender: sender.clone(),
            recipient,
            driver: None,
            status: DeliveryStatus::Pending,
            metadata,
            created_at: env.ledger().timestamp(),
            delivered_at: None,
            transit_started_at: None,
        };

        let key = DataKey::Delivery(delivery_id);
        env.storage().persistent().set(&key, &record);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (soroban_sdk::Symbol::new(&env, "delivery_created"),),
            (delivery_id, sender),
        );

        delivery_id
    }

    pub fn cancel_delivery(env: Env, sender: Address, delivery_id: DeliveryId) {
        sender.require_auth();

        let key = DataKey::Delivery(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"));

        if delivery.sender != sender {
            panic!("NotAuthorized");
        }

        validate_transition(delivery.status.clone(), DeliveryStatus::Cancelled)
            .unwrap_or_else(|_| panic!("InvalidState"));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic!("EscrowNotConfigured"));

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &soroban_sdk::Symbol::new(&env, "refund_escrow"),
            soroban_sdk::vec![&env, sender.into_val(&env), delivery_id.into_val(&env)],
        );

        delivery.status = DeliveryStatus::Cancelled;
        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (soroban_sdk::Symbol::new(&env, "delivery_cancelled"),),
            (delivery_id, sender),
        );
    }

    pub fn assign_driver(env: Env, caller: Address, delivery_id: DeliveryId, driver: Address) {
        caller.require_auth();

        let is_admin = Self::is_admin(&env, &caller);
        let is_self_assignment = caller == driver;

        if !is_admin && !is_self_assignment {
            panic!("NotAuthorized");
        }

        let key = DataKey::Delivery(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"));

        validate_transition(delivery.status.clone(), DeliveryStatus::Active)
            .unwrap_or_else(|_| panic!("InvalidState"));

        delivery.driver = Some(driver.clone());
        delivery.status = DeliveryStatus::Active;

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "driver_assigned"),),
            (delivery_id, driver),
        );
    }

    /// Allow the assigned driver to mark a delivery as actively in transit.
    /// Transitions: Active → InTransit. Records the ledger timestamp.
    pub fn mark_in_transit(env: Env, driver: Address, delivery_id: DeliveryId) {
        driver.require_auth();

        let key = DataKey::Delivery(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"));

        // Verify caller is the assigned driver for this delivery
        match &delivery.driver {
            Some(assigned) if *assigned == driver => {}
            _ => panic!("NotAuthorized"),
        }

        validate_transition(delivery.status.clone(), DeliveryStatus::InTransit)
            .unwrap_or_else(|_| panic!("InvalidState"));

        let timestamp = env.ledger().timestamp();
        delivery.status = DeliveryStatus::InTransit;
        delivery.transit_started_at = Some(timestamp);

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "DeliveryInTransit"),),
            (delivery_id, driver, timestamp),
        );
    }

    pub fn confirm_delivery(env: Env, recipient: Address, delivery_id: DeliveryId) {
        recipient.require_auth();

        let key = DataKey::Delivery(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"));

        if recipient != delivery.recipient {
            panic!("NotAuthorized");
        }

        validate_transition(delivery.status.clone(), DeliveryStatus::Delivered)
            .unwrap_or_else(|_| panic!("InvalidState"));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic!("EscrowNotConfigured"));

        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &soroban_sdk::Symbol::new(&env, "release_escrow"),
            soroban_sdk::vec![&env, recipient.into_val(&env), delivery_id.into_val(&env)],
        );

        delivery.status = DeliveryStatus::Delivered;
        delivery.delivered_at = Some(env.ledger().timestamp());

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        if let Some(driver_addr) = &delivery.driver {
            let driver_key = DataKey::DriverProfile(driver_addr.clone());
            let mut profile: DriverProfile = env
                .storage()
                .persistent()
                .get(&driver_key)
                .unwrap_or_else(|| DriverProfile {
                    address: driver_addr.clone(),
                    deliveries_completed: 0,
                    reputation_score: 0,
                    registered_at: env.ledger().timestamp(),
                    kyc_verified: false,
                });

            profile.deliveries_completed += 1;
            profile.reputation_score += 1;

            env.storage().persistent().set(&driver_key, &profile);
            env.storage()
                .persistent()
                .extend_ttl(&driver_key, 518400, 518400);
        }

        env.events().publish(
            (soroban_sdk::Symbol::new(&env, "delivery_confirmed"),),
            (delivery_id, recipient),
        );
    }

    /// Allow sender or recipient to escalate a delivery to Disputed and pause
    /// the escrow via a cross-contract call. The escrow call executes first so
    /// that delivery state is never mutated when the escrow call fails.
    pub fn raise_dispute(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();

        let key = DataKey::Delivery(delivery_id);
        let mut delivery: DeliveryRecord = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"));

        let is_sender = caller == delivery.sender;
        let is_recipient = caller == delivery.recipient;
        if !is_sender && !is_recipient {
            panic!("NotAuthorized");
        }

        validate_transition(delivery.status.clone(), DeliveryStatus::Disputed)
            .unwrap_or_else(|_| panic!("InvalidState"));

        let escrow_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic!("EscrowNotConfigured"));

        // Cross-contract call first: if escrow raises dispute fails, delivery
        // state is not mutated (implicit rollback via propagated panic).
        use soroban_sdk::IntoVal;
        let _: () = env.invoke_contract(
            &escrow_address,
            &Symbol::new(&env, "raise_dispute"),
            soroban_sdk::vec![&env, caller.into_val(&env), delivery_id.into_val(&env)],
        );

        let timestamp = env.ledger().timestamp();
        delivery.status = DeliveryStatus::Disputed;

        env.storage().persistent().set(&key, &delivery);
        env.storage().persistent().extend_ttl(&key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "delivery_disputed"),),
            (delivery_id, caller, timestamp),
        );
    }

    fn is_admin(env: &Env, caller: &Address) -> bool {
        if let Some(admin) = env.storage().instance().get::<_, Address>(&DataKey::Admin) {
            admin == *caller
        } else {
            false
        }
    }

    pub fn get_driver_profile(env: Env, driver: Address) -> DriverProfile {
        let driver_key = DataKey::DriverProfile(driver.clone());
        env.storage()
            .persistent()
            .get(&driver_key)
            .unwrap_or_else(|| DriverProfile {
                address: driver,
                deliveries_completed: 0,
                reputation_score: 0,
                registered_at: env.ledger().timestamp(),
                kyc_verified: false,
            })
    }

    pub fn get_delivery(env: Env, delivery_id: DeliveryId) -> DeliveryRecord {
        let key = DataKey::Delivery(delivery_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic!("DeliveryNotFound"))
    }
}

#[cfg(test)]
mod test;
// TTL management - implementation in progress

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, Address, BytesN, Env, Symbol,
    Vec,
};
use shared_types::{DeliveryId, SwiftChainError};
use delivery_contract::{DeliveryContractClient, DeliveryStatus};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Open,
    ResolvedRefund,
    ResolvedPayout,
    Split,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCase {
    pub delivery_id: DeliveryId,
    pub status: DisputeStatus,
    pub raised_at: u64,
    pub raised_by: Address,
    pub evidence_hashes: Vec<BytesN<32>>,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin(Address),
    DeliveryContract,
    EscrowContract,
    DisputeTimeLimit,
    Dispute(DeliveryId),
}

#[contract]
pub struct DisputeResolutionContract;

#[contractimpl]
impl DisputeResolutionContract {
    pub fn init(
        env: Env,
        admin: Address,
        delivery_contract: Address,
        escrow_contract: Address,
        dispute_time_limit: u64,
    ) {
        if env.storage().instance().has(&DataKey::DeliveryContract) {
            panic_with_error!(&env, SwiftChainError::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::DeliveryContract, &delivery_contract);
        env.storage().instance().set(&DataKey::EscrowContract, &escrow_contract);
        env.storage().instance().set(&DataKey::DisputeTimeLimit, &dispute_time_limit);
        env.storage().instance().set(&DataKey::Admin(admin), &true);
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::Admin(new_admin), &true);
    }

    pub fn remove_admin(env: Env, caller: Address, old_admin: Address) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }
        env.storage().instance().remove(&DataKey::Admin(old_admin));
    }

    pub fn is_admin(env: Env, admin: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Admin(admin))
            .unwrap_or(false)
    }

    pub fn get_delivery_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::DeliveryContract)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized))
    }

    pub fn get_escrow_contract(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::EscrowContract)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::NotInitialized))
    }

    pub fn get_dispute_time_limit(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::DisputeTimeLimit)
            .unwrap_or(0)
    }

    pub fn raise_dispute(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();

        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery_client = DeliveryContractClient::new(&env, &delivery_contract_addr);
        
        // Fetch the delivery record
        let delivery = delivery_client.get_delivery(&delivery_id);

        // Verify the caller is sender or recipient
        if caller != delivery.sender && caller != delivery.recipient {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        // Verify state and time limit
        match delivery.status {
            DeliveryStatus::Delivered => {
                let delivered_at = delivery.delivered_at.unwrap_or(0);
                let current_time = env.ledger().timestamp();
                let dispute_limit = Self::get_dispute_time_limit(env.clone());
                if current_time > delivered_at + dispute_limit {
                    panic_with_error!(&env, SwiftChainError::InvalidState);
                }
            }
            DeliveryStatus::Active | DeliveryStatus::InTransit => {
                // Call delivery contract to transition to Disputed and pause escrow
                delivery_client.raise_dispute(&caller, &delivery_id);
            }
            _ => {
                panic_with_error!(&env, SwiftChainError::InvalidState);
            }
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        if env.storage().persistent().has(&dispute_key) {
            panic_with_error!(&env, SwiftChainError::DuplicateDelivery);
        }

        let dispute = DisputeCase {
            delivery_id,
            status: DisputeStatus::Open,
            raised_at: env.ledger().timestamp(),
            raised_by: caller.clone(),
            evidence_hashes: Vec::new(&env),
        };

        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(&dispute_key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "dispute_raised"), delivery_id),
            (caller, delivery_id),
        );
    }

    pub fn add_evidence_hash(
        env: Env,
        caller: Address,
        delivery_id: DeliveryId,
        evidence_hash: BytesN<32>,
    ) {
        caller.require_auth();

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, SwiftChainError::InvalidState);
        }

        let delivery_contract_addr = Self::get_delivery_contract(env.clone());
        let delivery_client = DeliveryContractClient::new(&env, &delivery_contract_addr);
        let delivery = delivery_client.get_delivery(&delivery_id);

        if caller != delivery.sender && caller != delivery.recipient {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        dispute.evidence_hashes.push_back(evidence_hash.clone());
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(&dispute_key, 518400, 518400);

        env.events().publish(
            (Symbol::new(&env, "evidence_added"), delivery_id),
            (caller, delivery_id, evidence_hash),
        );
    }

    pub fn resolve_dispute_refund_sender(env: Env, caller: Address, delivery_id: DeliveryId) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, SwiftChainError::InvalidState);
        }

        dispute.status = DisputeStatus::ResolvedRefund;
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(&dispute_key, 518400, 518400);

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
            (Symbol::new(&env, "dispute_resolved_refund"), delivery_id),
            (caller, delivery_id),
        );
    }

    pub fn resolve_dispute_split_funds(
        env: Env,
        caller: Address,
        delivery_id: DeliveryId,
        sender_share_bps: u32,
    ) {
        caller.require_auth();
        if !Self::is_admin(env.clone(), caller.clone()) {
            panic_with_error!(&env, SwiftChainError::Unauthorized);
        }

        let dispute_key = DataKey::Dispute(delivery_id);
        let mut dispute: DisputeCase = env
            .storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::DeliveryNotFound));

        if dispute.status != DisputeStatus::Open {
            panic_with_error!(&env, SwiftChainError::InvalidState);
        }

        dispute.status = DisputeStatus::Split;
        env.storage().persistent().set(&dispute_key, &dispute);
        env.storage().persistent().extend_ttl(&dispute_key, 518400, 518400);

        let escrow_addr = Self::get_escrow_contract(env.clone());
        let escrow_client = escrow_contract::EscrowContractClient::new(&env, &escrow_addr);
        let escrow = escrow_client.get_escrow(&u64::from(delivery_id));

        if escrow.status == shared_types::EscrowStatus::Paused {
            use soroban_sdk::IntoVal;
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
        }

        env.events().publish(
            (Symbol::new(&env, "dispute_resolved_split"), delivery_id),
            (caller, delivery_id),
        );
    }

    pub fn get_dispute(env: Env, delivery_id: DeliveryId) -> DisputeCase {
        let dispute_key = DataKey::Dispute(delivery_id);
        env.storage()
            .persistent()
            .get(&dispute_key)
            .unwrap_or_else(|| panic_with_error!(&env, SwiftChainError::DeliveryNotFound))
    }
}

#[cfg(test)]
mod test;


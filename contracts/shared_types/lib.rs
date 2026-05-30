#![no_std]

use soroban_sdk::{contracterror, contracttype, Address, String};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SwiftChainError {
    /// Caller is not authorized to perform the requested action.
    Unauthorized = 1,
    /// Contract or protocol state has already been initialized.
    AlreadyInitialized = 2,
    /// Contract or protocol state has not been initialized yet.
    NotInitialized = 3,
    /// Delivery record or related escrow entry could not be found.
    DeliveryNotFound = 4,
    /// Requested operation is invalid for the current protocol state.
    InvalidState = 5,
    /// Contract balance is too low to complete the requested transfer.
    InsufficientFunds = 6,
    /// Escrow funds are locked and cannot be released or refunded yet.
    EscrowLocked = 7,
    /// Delivery identifier already exists in protocol storage.
    DuplicateDelivery = 8,
    /// Provider or driver record could not be found.
    ProviderNotFound = 9,
    /// Address argument is invalid for the requested operation.
    InvalidAddress = 10,
}

// Event topic constants for on-chain event tracking
pub mod events {
    use soroban_sdk::{Env, Symbol};

    pub const DELIVERY_CREATED: &str = "delivery_created";
    pub const ESCROW_FUNDED: &str = "escrow_funded";
    pub const DRIVER_ASSIGNED: &str = "driver_assigned";
    pub const DELIVERY_CONFIRMED: &str = "delivery_confirmed";
    pub const ESCROW_RELEASED: &str = "escrow_released";
    pub const DELIVERY_DISPUTED: &str = "delivery_disputed";
    pub const ESCROW_REFUNDED: &str = "escrow_refunded";

    pub fn delivery_created(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_CREATED)
    }

    pub fn escrow_funded(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_FUNDED)
    }

    pub fn driver_assigned(env: &Env) -> Symbol {
        Symbol::new(env, DRIVER_ASSIGNED)
    }

    pub fn delivery_confirmed(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_CONFIRMED)
    }

    pub fn escrow_released(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_RELEASED)
    }

    pub fn escrow_refunded(env: &Env) -> Symbol {
        Symbol::new(env, ESCROW_REFUNDED)
    }

    pub fn delivery_disputed(env: &Env) -> Symbol {
        Symbol::new(env, DELIVERY_DISPUTED)
    }

    pub fn dispute_resolved(env: &Env) -> Symbol {
        Symbol::new(env, "dispute_resolved")
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryCreatedEvent {
    /// Unique protocol delivery identifier created by the delivery contract.
    pub delivery_id: u64,
    /// Address that created and funds the delivery request.
    pub sender: Address,
    /// Escrow amount expected for the delivery when known by the emitter.
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowFundedEvent {
    /// Delivery identifier whose escrow was funded.
    pub delivery_id: u64,
    /// Address that transferred tokens into escrow.
    pub sender: Address,
    /// Token contract address used for the escrow balance.
    pub token: Address,
    /// Amount transferred into escrow.
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverAssignedEvent {
    /// Delivery identifier assigned to a driver.
    pub delivery_id: u64,
    /// Driver address assigned to complete the delivery.
    pub driver: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryConfirmedEvent {
    /// Delivery identifier confirmed by the recipient.
    pub delivery_id: u64,
    /// Recipient address that confirmed completion.
    pub recipient: Address,
    /// Ledger timestamp when delivery completion was confirmed.
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowReleasedEvent {
    /// Delivery identifier whose escrow was released.
    pub delivery_id: u64,
    /// Driver address receiving released escrow funds.
    pub driver: Address,
    /// Amount released to the driver.
    pub amount: i128,
    /// Platform fee withheld during release.
    pub platform_fee: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryDisputedEvent {
    /// Delivery identifier moved into dispute handling.
    pub delivery_id: u64,
    /// Address that raised or recorded the dispute.
    pub reporter: Address,
    /// Ledger timestamp when the dispute was recorded.
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRefundedEvent {
    /// Delivery identifier whose escrow was refunded.
    pub delivery_id: u64,
    /// Original sender address receiving refunded funds.
    pub sender: Address,
    /// Amount returned to the sender.
    pub amount: i128,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct DeliveryId(pub u64);

impl DeliveryId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

impl From<u64> for DeliveryId {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl From<DeliveryId> for u64 {
    fn from(value: DeliveryId) -> Self {
        value.0
    }
}

impl PartialEq<u64> for DeliveryId {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}

impl PartialEq<DeliveryId> for u64 {
    fn eq(&self, other: &DeliveryId) -> bool {
        *self == other.0
    }
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum DeliveryStatus {
    Pending,
    Active,
    InTransit,
    Delivered,
    Disputed,
    Cancelled,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum EscrowState {
    Locked,
    Released,
    Refunded,
    Paused,
}

pub type EscrowStatus = EscrowState;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartyAddresses {
    pub sender: Address,
    pub driver: Address,
    pub recipient: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub token: Address,
    pub platform_fee_bps: u32,
    pub protocol_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Instance storage for the shared admin address.
    Admin,
    /// Persistent storage for a delivery record.
    Delivery(DeliveryId),
    /// Persistent storage for an escrow record.
    Escrow(DeliveryId),
    /// Persistent storage for a driver profile.
    DriverProfile(Address),
    /// Instance storage for protocol-wide configuration.
    ProtocolConfig,
}

pub fn delivery_key(id: impl Into<DeliveryId>) -> StorageKey {
    StorageKey::Delivery(id.into())
}

pub fn escrow_key(id: impl Into<DeliveryId>) -> StorageKey {
    StorageKey::Escrow(id.into())
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowRecord {
    pub sender: Address,
    pub recipient: Address,
    pub driver: Address,
    pub token: Address,
    pub amount: i128,
    pub status: EscrowState,
    pub created_at: u64,
    pub disputed_by: Option<Address>,
    pub disputed_at: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryDetails {
    pub id: DeliveryId,
    pub driver: String,
    pub status: DeliveryStatus,
}

#[cfg(test)]
mod test {
    use super::{
        CargoCategory, CargoDescriptor, DeliveryConfirmedEvent, DeliveryCreatedEvent,
        DeliveryDisputedEvent, DeliveryId, DeliveryMetadata, DeliveryStatus, DriverAssignedEvent,
        EscrowFundedEvent, EscrowRefundedEvent, EscrowReleasedEvent, EscrowState, PartyAddresses,
        StorageKey, SwiftChainError, delivery_key, escrow_key,
    };
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    #[test]
    fn delivery_id_wraps_raw_u64() {
        let delivery_id = DeliveryId::new(42);

        assert_eq!(delivery_id, 42);
        assert_eq!(u64::from(delivery_id), 42);
    }

    #[test]
    fn delivery_and_escrow_states_expose_expected_variants() {
        assert_eq!(DeliveryStatus::Pending, DeliveryStatus::Pending);
        assert_eq!(DeliveryStatus::Active, DeliveryStatus::Active);
        assert_eq!(DeliveryStatus::InTransit, DeliveryStatus::InTransit);
        assert_eq!(DeliveryStatus::Delivered, DeliveryStatus::Delivered);
        assert_eq!(DeliveryStatus::Disputed, DeliveryStatus::Disputed);
        assert_eq!(DeliveryStatus::Cancelled, DeliveryStatus::Cancelled);

        assert_eq!(EscrowState::Locked, EscrowState::Locked);
        assert_eq!(EscrowState::Released, EscrowState::Released);
        assert_eq!(EscrowState::Refunded, EscrowState::Refunded);
        assert_eq!(EscrowState::Paused, EscrowState::Paused);
    }

    #[test]
    fn party_addresses_preserve_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let driver = Address::generate(&env);
        let recipient = Address::generate(&env);
        let party_addresses = PartyAddresses {
            sender: sender.clone(),
            driver: driver.clone(),
            recipient: recipient.clone(),
        };

        assert_eq!(party_addresses.sender, sender);
        assert_eq!(party_addresses.driver, driver);
        assert_eq!(party_addresses.recipient, recipient);
    }

    #[test]
    fn storage_key_helpers_construct_expected_variants() {
        let delivery_id = DeliveryId::new(7);

        assert_eq!(delivery_key(delivery_id), StorageKey::Delivery(delivery_id));
        assert_eq!(escrow_key(delivery_id), StorageKey::Escrow(delivery_id));
    }

    #[test]
    fn unauthorized_has_expected_discriminant() {
        assert_eq!(SwiftChainError::Unauthorized as u32, 1);
    }

    #[test]
    fn already_initialized_has_expected_discriminant() {
        assert_eq!(SwiftChainError::AlreadyInitialized as u32, 2);
    }

    #[test]
    fn not_initialized_has_expected_discriminant() {
        assert_eq!(SwiftChainError::NotInitialized as u32, 3);
    }

    #[test]
    fn delivery_not_found_has_expected_discriminant() {
        assert_eq!(SwiftChainError::DeliveryNotFound as u32, 4);
    }

    #[test]
    fn invalid_state_has_expected_discriminant() {
        assert_eq!(SwiftChainError::InvalidState as u32, 5);
    }

    #[test]
    fn insufficient_funds_has_expected_discriminant() {
        assert_eq!(SwiftChainError::InsufficientFunds as u32, 6);
    }

    #[test]
    fn escrow_locked_has_expected_discriminant() {
        assert_eq!(SwiftChainError::EscrowLocked as u32, 7);
    }

    #[test]
    fn duplicate_delivery_has_expected_discriminant() {
        assert_eq!(SwiftChainError::DuplicateDelivery as u32, 8);
    }

    #[test]
    fn provider_not_found_has_expected_discriminant() {
        assert_eq!(SwiftChainError::ProviderNotFound as u32, 9);
    }

    #[test]
    fn invalid_address_has_expected_discriminant() {
        assert_eq!(SwiftChainError::InvalidAddress as u32, 10);
    }

    #[test]
    fn delivery_created_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let event = DeliveryCreatedEvent {
            delivery_id: 1,
            sender: sender.clone(),
            amount: 100,
        };

        assert_eq!(event.delivery_id, 1);
        assert_eq!(event.sender, sender);
        assert_eq!(event.amount, 100);
    }

    #[test]
    fn escrow_funded_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let token = Address::generate(&env);
        let event = EscrowFundedEvent {
            delivery_id: 2,
            sender: sender.clone(),
            token: token.clone(),
            amount: 250,
        };

        assert_eq!(event.delivery_id, 2);
        assert_eq!(event.sender, sender);
        assert_eq!(event.token, token);
        assert_eq!(event.amount, 250);
    }

    #[test]
    fn driver_assigned_event_preserves_fields() {
        let env = Env::default();
        let driver = Address::generate(&env);
        let event = DriverAssignedEvent {
            delivery_id: 3,
            driver: driver.clone(),
        };

        assert_eq!(event.delivery_id, 3);
        assert_eq!(event.driver, driver);
    }

    #[test]
    fn delivery_confirmed_event_preserves_fields() {
        let env = Env::default();
        let recipient = Address::generate(&env);
        let event = DeliveryConfirmedEvent {
            delivery_id: 4,
            recipient: recipient.clone(),
            timestamp: 12345,
        };

        assert_eq!(event.delivery_id, 4);
        assert_eq!(event.recipient, recipient);
        assert_eq!(event.timestamp, 12345);
    }

    #[test]
    fn escrow_released_event_preserves_fields() {
        let env = Env::default();
        let driver = Address::generate(&env);
        let event = EscrowReleasedEvent {
            delivery_id: 5,
            driver: driver.clone(),
            amount: 500,
            platform_fee: 10,
        };

        assert_eq!(event.delivery_id, 5);
        assert_eq!(event.driver, driver);
        assert_eq!(event.amount, 500);
        assert_eq!(event.platform_fee, 10);
    }

    #[test]
    fn delivery_disputed_event_preserves_fields() {
        let env = Env::default();
        let reporter = Address::generate(&env);
        let event = DeliveryDisputedEvent {
            delivery_id: 6,
            reporter: reporter.clone(),
            timestamp: 56789,
        };

        assert_eq!(event.delivery_id, 6);
        assert_eq!(event.reporter, reporter);
        assert_eq!(event.timestamp, 56789);
    }

    #[test]
    fn escrow_refunded_event_preserves_fields() {
        let env = Env::default();
        let sender = Address::generate(&env);
        let event = EscrowRefundedEvent {
            delivery_id: 7,
            sender: sender.clone(),
            amount: 700,
        };

        assert_eq!(event.delivery_id, 7);
        assert_eq!(event.sender, sender);
        assert_eq!(event.amount, 700);
    }

    #[test]
    fn test_cargo_descriptor() {
        let _env = Env::default();
        let desc = CargoDescriptor {
            weight_grams: 500,
            category: CargoCategory::Electronics,
            fragile: true,
        };
        assert_eq!(desc.weight_grams, 500);
        assert_eq!(desc.fragile, true);
        assert_eq!(desc.category, CargoCategory::Electronics);
    }

    #[test]
    fn test_delivery_metadata() {
        let env = Env::default();
        let cargo = CargoDescriptor {
            weight_grams: 1000,
            category: CargoCategory::General,
            fragile: false,
        };
        let metadata = DeliveryMetadata {
            delivery_id: 1,
            origin: String::from_str(&env, "Location A"),
            destination: String::from_str(&env, "Location B"),
            cargo_description: cargo,
            created_at: 1000000,
            estimated_delivery: 2000000,
        };
        assert_eq!(metadata.delivery_id, 1);
        assert_eq!(metadata.created_at, 1000000);
        assert_eq!(metadata.cargo_description.weight_grams, 1000);
    }
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverProfile {
    pub address: Address,
    pub deliveries_completed: u32,
    pub reputation_score: u32,
    pub registered_at: u64,
    pub kyc_verified: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserProfile {
    pub address: Address,
    pub registered_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CargoCategory {
    Documents,
    Electronics,
    Perishables,
    Clothing,
    General,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoDescriptor {
    pub weight_grams: u32,
    pub category: CargoCategory,
    pub fragile: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryMetadata {
    pub delivery_id: u64,
    pub origin: String,
    pub destination: String,
    pub cargo_description: CargoDescriptor,
    pub created_at: u64,
    pub estimated_delivery: u64,
}

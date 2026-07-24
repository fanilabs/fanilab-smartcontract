# FaniLab Centralized Event System

## Overview

All Soroban contracts in the FaniLab platform emit on-chain events via a **centralized event system** defined in the `shared_types` crate. This approach ensures consistent topic strings, typed payloads, and a single source of truth for off-chain indexers and monitoring tools.

Every `env.events().publish()` call across all contracts uses:
1. A **topic helper** from `shared_types::events` — e.g., `events::delivery_created(&env)`
2. A **typed payload struct** from `shared_types` — e.g., `DeliveryCreatedEvent { ... }`

Inline `Symbol::new(env, "string_literal")` calls in event topics are not permitted in contract code. They exist only for non-event purposes such as cross-contract call method names.

---

## Module Layout

```
contracts/shared_types/lib.rs
├── pub mod events          ← all topic Symbol helpers
├── DeliveryCreatedEvent
├── EscrowFundedEvent
├── DriverAssignedEvent
├── DeliveryConfirmedEvent
├── EscrowReleasedEvent
├── DeliveryDisputedEvent
├── EscrowRefundedEvent
├── DisputeResolvedEvent
├── FleetRegisteredEvent
├── FleetTreasuryUpdatedEvent
├── DriverInvitedEvent
├── InviteAcceptedEvent
├── DriverRemovedEvent
├── DisputeRaisedEvent
├── DisputeResolvedRefundEvent
├── DisputeResolvedSplitEvent
├── DisputeResolvedPayoutEvent
├── DriverRegisteredEvent
├── UserRegisteredEvent
├── KycStatusUpdatedEvent
├── ReputationIncreasedEvent
└── ReputationDecreasedEvent
```

---

## Topic Helpers

All topic helpers live in `shared_types::events`. Each helper wraps `Symbol::new(env, "<name>")`:

| Helper | Symbol string | Used by |
|---|---|---|
| `delivery_created` | `"delivery_created"` | `delivery_contract` |
| `delivery_cancelled` | `"delivery_cancelled"` | `delivery_contract` |
| `delivery_in_transit` | `"delivery_in_transit"` | `delivery_contract` |
| `driver_assigned` | `"driver_assigned"` | `delivery_contract` |
| `delivery_confirmed` | `"delivery_confirmed"` | `delivery_contract` |
| `delivery_disputed` | `"delivery_disputed"` | `delivery_contract`, `escrow_contract` |
| `escrow_funded` | `"escrow_funded"` | `escrow_contract` |
| `escrow_released` | `"escrow_released"` | `escrow_contract` |
| `escrow_refunded` | `"escrow_refunded"` | `escrow_contract` |
| `dispute_resolved` | `"dispute_resolved"` | `escrow_contract` |
| `fleet_registered` | `"fleet_registered"` | `fleet_management_contract` |
| `fleet_treasury_updated` | `"fleet_treasury_updated"` | `fleet_management_contract` |
| `driver_invited` | `"driver_invited"` | `fleet_management_contract` |
| `invite_accepted` | `"invite_accepted"` | `fleet_management_contract` |
| `driver_removed` | `"driver_removed"` | `fleet_management_contract` |
| `dispute_raised` | `"dispute_raised"` | `dispute_resolution_contract` |
| `evidence_added` | `"evidence_added"` | `dispute_resolution_contract` |
| `dispute_resolved_refund` | `"dispute_resolved_refund"` | `dispute_resolution_contract` |
| `dispute_resolved_split` | `"dispute_resolved_split"` | `dispute_resolution_contract` |
| `dispute_resolved_payout` | `"dispute_resolved_payout"` | `dispute_resolution_contract` |
| `driver_registered` | `"driver_registered"` | `identity_reputation_contract` |
| `user_registered` | `"user_registered"` | `identity_reputation_contract` |
| `kyc_status_updated` | `"kyc_status_updated"` | `identity_reputation_contract` |
| `reputation_increased` | `"reputation_increased"` | `identity_reputation_contract` |
| `reputation_decreased` | `"reputation_decreased"` | `identity_reputation_contract` |

---

## Payload Structs

Each event topic has a corresponding `#[contracttype]` struct. All structs derive `Clone, Debug, Eq, PartialEq` and are XDR-serializable via the Soroban SDK.

### Delivery Events

#### `DeliveryCreatedEvent`
Emitted when a new delivery record is created.
```rust
pub struct DeliveryCreatedEvent {
    pub delivery_id: u64,   // assigned delivery identifier
    pub sender: Address,    // address that created the delivery
    pub amount: i128,       // escrow amount (0 when emitted from delivery_contract)
}
```

#### `DriverAssignedEvent`
Emitted when a driver is assigned to a delivery.
```rust
pub struct DriverAssignedEvent {
    pub delivery_id: u64,
    pub driver: Address,
}
```

#### `DeliveryConfirmedEvent`
Emitted when the recipient confirms delivery.
```rust
pub struct DeliveryConfirmedEvent {
    pub delivery_id: u64,
    pub recipient: Address,
    pub timestamp: u64,     // ledger timestamp of confirmation
}
```

#### `DeliveryDisputedEvent`
Emitted when a delivery is placed in dispute (by delivery_contract or escrow_contract).
```rust
pub struct DeliveryDisputedEvent {
    pub delivery_id: u64,
    pub reporter: Address,  // sender or recipient that raised the dispute
    pub timestamp: u64,
}
```

> **Note:** `delivery_cancelled` and `delivery_in_transit` use plain tuple payloads `(delivery_id, address)` and `(delivery_id, driver, timestamp)` respectively, as no separate struct is required for these lightweight events.

---

### Escrow Events

#### `EscrowFundedEvent`
Emitted when escrow funds are locked on delivery creation.
```rust
pub struct EscrowFundedEvent {
    pub delivery_id: u64,
    pub sender: Address,
    pub token: Address,     // token contract used for escrow
    pub amount: i128,
}
```

#### `EscrowReleasedEvent`
Emitted when locked escrow is released to the driver.
```rust
pub struct EscrowReleasedEvent {
    pub delivery_id: u64,
    pub driver: Address,
    pub amount: i128,       // net amount after platform fee
    pub platform_fee: i128,
}
```

#### `EscrowRefundedEvent`
Emitted when escrow is refunded to the sender.
```rust
pub struct EscrowRefundedEvent {
    pub delivery_id: u64,
    pub sender: Address,
    pub amount: i128,
}
```

#### `DisputeResolvedEvent`
Emitted by `escrow_contract` when an admin resolves a dispute (full payout or full refund).
```rust
pub struct DisputeResolvedEvent {
    pub delivery_id: u64,
    pub resolver: Address,  // admin address
}
```

---

### Fleet Events

#### `FleetRegisteredEvent`
```rust
pub struct FleetRegisteredEvent {
    pub fleet_id: u64,
    pub owner: Address,
    pub treasury: Address,
}
```

#### `FleetTreasuryUpdatedEvent`
```rust
pub struct FleetTreasuryUpdatedEvent {
    pub fleet_id: u64,
    pub owner: Address,
    pub treasury: Address,  // new treasury address
}
```

#### `DriverInvitedEvent`
```rust
pub struct DriverInvitedEvent {
    pub fleet_id: u64,
    pub driver: Address,
}
```

#### `InviteAcceptedEvent`
```rust
pub struct InviteAcceptedEvent {
    pub fleet_id: u64,
    pub driver: Address,
}
```

#### `DriverRemovedEvent`
```rust
pub struct DriverRemovedEvent {
    pub fleet_id: u64,
    pub driver: Address,
}
```

---

### Dispute Resolution Events

#### `DisputeRaisedEvent`
Emitted by `dispute_resolution_contract` when a dispute case is opened.
```rust
pub struct DisputeRaisedEvent {
    pub delivery_id: u64,
    pub caller: Address,
}
```

> **Note:** `evidence_added` uses a tuple payload `(caller, delivery_id, evidence_hash)` because `BytesN<32>` is a Soroban SDK type that is not practical to embed in a `shared_types` struct (which uses `#![no_std]` and should avoid deep SDK type coupling in payload definitions). The topic still uses the shared helper `events::evidence_added(&env)`.

#### `DisputeResolvedRefundEvent`
Emitted when dispute is resolved in favor of the sender (refund).
```rust
pub struct DisputeResolvedRefundEvent {
    pub delivery_id: u64,
    pub caller: Address,    // admin resolver
    pub driver: Address,    // driver penalized
    pub penalty: u32,       // reputation points deducted
}
```

#### `DisputeResolvedSplitEvent`
Emitted when dispute is resolved with a split payout.
```rust
pub struct DisputeResolvedSplitEvent {
    pub delivery_id: u64,
    pub caller: Address,
}
```

#### `DisputeResolvedPayoutEvent`
Emitted when dispute is resolved in favor of the driver (full payout).
```rust
pub struct DisputeResolvedPayoutEvent {
    pub delivery_id: u64,
    pub caller: Address,
}
```

---

### Identity & Reputation Events

#### `DriverRegisteredEvent`
```rust
pub struct DriverRegisteredEvent {
    pub driver: Address,
}
```

#### `UserRegisteredEvent`
```rust
pub struct UserRegisteredEvent {
    pub user: Address,
}
```

#### `KycStatusUpdatedEvent`
```rust
pub struct KycStatusUpdatedEvent {
    pub driver: Address,
    pub kyc_verified: bool,
}
```

#### `ReputationIncreasedEvent`
```rust
pub struct ReputationIncreasedEvent {
    pub driver: Address,
    pub delivery_id: u64,
    pub points: u32,
}
```

#### `ReputationDecreasedEvent`
```rust
pub struct ReputationDecreasedEvent {
    pub driver: Address,
    pub points: u32,
}
```

---

## Usage Pattern

Every contract that emits events must:

1. Import `shared_types::events` and the relevant payload struct:
   ```rust
   use shared_types::{events, DeliveryCreatedEvent};
   ```

2. Publish with the topic helper and typed payload:
   ```rust
   env.events().publish(
       (events::delivery_created(&env),),
       DeliveryCreatedEvent {
           delivery_id: delivery_id.value(),
           sender,
           amount: 0,
       },
   );
   ```

Do **not** use inline `Symbol::new(env, "delivery_created")` in an event topic. That call belongs inside `shared_types::events` only.

---

## Adding New Events

1. Add a topic helper function to `shared_types::events`:
   ```rust
   pub fn my_new_event(env: &Env) -> Symbol {
       Symbol::new(env, "my_new_event")
   }
   ```

2. Add a payload struct in `shared_types`:
   ```rust
   #[contracttype]
   #[derive(Clone, Debug, Eq, PartialEq)]
   pub struct MyNewEvent {
       pub field: u64,
   }
   ```

3. Use them in the contract:
   ```rust
   use shared_types::{events, MyNewEvent};
   // ...
   env.events().publish(
       (events::my_new_event(&env),),
       MyNewEvent { field: value },
   );
   ```

4. Update this document.

---

## Off-chain Indexing

Off-chain consumers (backend, analytics) should subscribe to these topic symbols. Because all topic strings are centralized here, renaming an event requires only a single change in `shared_types::events` — and this document — not a grep across all contracts.

Events are emitted to the Stellar ledger and can be retrieved via the Soroban RPC `getEvents` endpoint, filtering by `contractId` and `topic`.

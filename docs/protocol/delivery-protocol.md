# Delivery Protocol

## Overview

The `delivery_contract` manages the lifecycle of a logistics package on the FaniLab platform. It tracks delivery status, driver assignments, and metadata throughout the delivery journey — from creation through to proof of delivery.

The delivery protocol is tightly coupled with the **escrow contract** for financial settlement and with the **identity & reputation contract** for driver verification.

**Status**: Implemented (Phase 1)

---

## Delivery State Machine

Every delivery follows a strict state machine enforced by `validate_transition()`:

```
            ┌─────────────────────────────┐
            │          Pending             │
            └──────────┬───────────────────┘
                       │
              ┌────────┴────────┐
              ▼                 ▼
        ┌──────────┐    ┌────────────┐
        │  Active   │    │ Cancelled  │ (terminal)
        └─────┬─────┘    └────────────┘
              │
        ┌─────┴─────┐
        ▼           ▼
  ┌──────────┐ ┌──────────┐
  │ InTransit │ │ Disputed │
  └─────┬─────┘ └─────┬────┘
        │              │
        ▼              ├──────────────┐
  ┌──────────┐        ▼              ▼
  │ Delivered │  ┌──────────┐  ┌────────────┐
  │(terminal) │  │ Delivered │  │ Cancelled  │
  └───────────┘  │(terminal) │  │ (terminal) │
                 └───────────┘  └────────────┘
```

### Allowed Transitions

| From         | To          | Trigger                                                  |
|--------------|-------------|----------------------------------------------------------|
| `Pending`    | `Active`    | Sender funds escrow → delivery goes active               |
| `Pending`    | `Cancelled` | Sender cancels before funding                            |
| `Active`     | `InTransit` | Driver picks up the package and confirms                 |
| `Active`     | `Disputed`  | Sender (only) raises a dispute                           |
| `Active`     | `Cancelled` | Sender or admin cancels the delivery                     |
| `InTransit`  | `Delivered` | Driver confirms delivery (proof of delivery)             |
| `InTransit`  | `Disputed`  | Sender or driver raises a dispute                        |
| `Disputed`   | `Delivered` | Dispute resolved in driver's favor → delivery completes  |
| `Disputed`   | `Cancelled` | Dispute resolved with refund → delivery cancelled        |

`Delivered` and `Cancelled` are **terminal states** — no further transitions are permitted.

### Delivery Record

```rust
pub struct DeliveryRecord {
    pub id: DeliveryId,          // u64 identifier
    pub sender: Address,
    pub recipient: Address,
    pub driver: Option<Address>, // Assigned after Active state
    pub status: DeliveryStatus,
    pub metadata: DeliveryMetadata,
    pub created_at: u64,
    pub updated_at: u64,
}
```

---

## Key Functions

### `init(env, admin, escrow_contract, identity_reputation_contract)`
Initializes the delivery contract with:
- Admin address
- The escrow contract address (for funding/release coordination)
- The identity & reputation contract address (for driver tier verification)

Reverts with `AlreadyInitialized` if called more than once.

### `create_delivery(env, sender, recipient, metadata)`
Creates a new delivery in `Pending` status. The sender:
1. Specifies the recipient address
2. Provides delivery metadata (origin, destination, cargo description, estimated delivery time)
3. Receives a unique `DeliveryId` (auto-incremented counter)

Emits a `delivery_created` event with the delivery ID, sender, and metadata.

### `assign_driver(env, caller, delivery_id, driver)`
Assigns a driver to a delivery. Validation rules:
- Only the **sender** or an **admin** can assign a driver
- Delivery must be in `Active` status
- The driver profile must exist in the identity & reputation contract (via `get_driver_profile`)

Transitions status to `Active` (if it was `Pending`) and stores the driver address.
Emits a `driver_assigned` event.

### `confirm_in_transit(env, driver, delivery_id)`
The assigned driver confirms they have picked up the package. Validations:
- Only the assigned driver can call this
- Delivery must be in `Active` status

Transitions status to `InTransit`.
Emits a `delivery_in_transit` event.

### `confirm_delivery(env, recipient, delivery_id)`
The recipient confirms delivery completion (proof of delivery). Validations:
- Only the delivery recipient can call this
- Delivery must be in `InTransit` status

This call does not pay the driver immediately. It transitions the delivery to `Delivered` and invokes the escrow contract's `mark_holdback_escrow`, which moves the matching escrow into the `Holdback` state. The driver is paid only after a separate `release_holdback_escrow` call by the recipient or an admin.

`Holdback` is the escrow-side intermediate state between confirmation and final settlement: funds are reserved for the driver, but still require the follow-up release step before payout is complete.
Emits a `delivery_confirmed` event.

### `cancel_delivery(env, caller, delivery_id)`
Cancels a delivery. Authorization:
- The **sender** can cancel at any time
- An **admin** can cancel if there's an active dispute

Valid transitions: `Pending` → `Cancelled`, `Active` → `Cancelled`, `Disputed` → `Cancelled`.
Calls the escrow contract's `refund_escrow` to return funds to sender.
Emits a `delivery_cancelled` event.

### `dispute_delivery(env, reporter, delivery_id)`
Raises a dispute on a delivery. Valid transitions:
- `Active` → `Disputed` (only sender can raise)
- `InTransit` → `Disputed` (sender or driver can raise)

Calls the escrow contract's `freeze_funds` to pause the escrow.
Emits a `delivery_disputed` event.

---

## Combined State Synchronization (ADR-010)

The delivery contract provides a `get_combined_state(delivery_id)` view that fetches both the delivery record and the corresponding escrow record, then validates them against the following invariants:

| Delivery Status | Expected Escrow Status |
|-----------------|------------------------|
| `Pending`       | `Locked`               |
| `Active`        | `Locked`               |
| `InTransit`     | `Locked`               |
| `Delivered`     | `Holdback` or `Released` |
| `Disputed`      | `Paused`               |
| `Cancelled`     | `Refunded`             |

Returns `(DeliveryRecord, EscrowRecord, is_synchronized)`. The boolean `is_synchronized` is `false` if any mismatch is detected, enabling off-chain indexers and auditors to detect desynchronization.

---

## Cross-Contract Interactions

```
┌───────────────────┐     ┌─────────────────────┐
│  DeliveryContract  │────►│   EscrowContract    │
│                   │     │  fund_escrow         │
│  create_delivery  │     │  mark_holdback_escrow│
│  confirm_delivery │     │  release_holdback_escrow |
│  cancel_delivery  │     │  refund_escrow       │
│  dispute_delivery │     │  freeze_funds        │
│                   │     └─────────────────────┘
│  assign_driver    │────►│ IdentityReputation   │
│                   │     │  get_driver_profile  │
└───────────────────┘     └─────────────────────┘
```

---

## Events

| Topic                  | Emitted By             | Payload                                              |
|------------------------|------------------------|------------------------------------------------------|
| `delivery_created`     | `create_delivery`      | `(delivery_id, sender, metadata)`                    |
| `driver_assigned`      | `assign_driver`        | `(delivery_id, driver)`                              |
| `delivery_in_transit`  | `confirm_in_transit`   | `(delivery_id, driver, timestamp)`                   |
| `delivery_confirmed`   | `confirm_delivery`     | `(delivery_id, driver, timestamp)`                   |
| `delivery_cancelled`   | `cancel_delivery`      | `(delivery_id, caller)`                              |
| `delivery_disputed`    | `dispute_delivery`     | `(delivery_id, reporter, timestamp)`                 |

All event topics use the centralized helpers in `shared_types::events`. See the [Event System](../architecture/event-system.md) document for details.

---

## Delivery Metadata

Each delivery includes rich metadata:

```rust
pub struct DeliveryMetadata {
    pub delivery_id: u64,
    pub origin: String,              // Max 256 chars
    pub destination: String,         // Max 256 chars
    pub cargo_description: CargoDescriptor,
    pub created_at: u64,
    pub estimated_delivery: u64,
}

pub struct CargoDescriptor {
    pub weight_grams: u32,           // Max 1,000,000g (1,000 kg)
    pub category: CargoCategory,     // Documents, Electronics, Perishables, Clothing, General
    pub fragile: bool,
}
```

---

## Security Considerations

1. **Access Control**: Driver assignments and status transitions are strictly gated by role (sender, driver, admin).
2. **State Validation**: Every transition is validated against the state machine; invalid transitions revert with `InvalidState`.
3. **Escrow Coupling**: Fund movements always go through the escrow contract — the delivery contract never holds funds directly.
4. **Metadata Validation**: Origin/destination lengths and cargo weight are validated on creation to prevent spam or overflow.

---

## Related Documents

- [Smart Contract Architecture](../architecture/smart-contract-architecture.md)
- [Escrow Design](../contract-design/escrow-design.md)
- [Event System](../architecture/event-system.md)
- [ADR-004: State Transition Validation](../ARCHITECTURE_DECISION_RECORDS.md)
- [ADR-010: Delivery-Escrow State Machine Coupling](../ARCHITECTURE_DECISION_RECORDS.md)

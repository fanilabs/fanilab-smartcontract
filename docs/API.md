# FaniLab Smart Contract API Reference

Complete API documentation for all FaniLab smart contracts.

## Table of Contents
- [Indexing & Enumeration](#indexing--enumeration)
- [Escrow Contract](#escrow-contract)
- [Delivery Contract](#delivery-contract)
- [Dispute Resolution Contract](#dispute-resolution-contract)
- [Fleet Management Contract](#fleet-management-contract)
- [Identity Reputation Contract](#identity-reputation-contract)
- [Settlement Contract](#settlement-contract)
- [Shared Types](#shared-types)

---

## Indexing & Enumeration

The protocol supports on-chain secondary indexes to enable efficient queries without requiring off-chain indexers.

### On-Chain Indexes

Secondary indexes are maintained alongside primary records in persistent storage for:
- **Deliveries by Sender** — all delivery IDs initiated by a given sender
- **Deliveries by Recipient** — all delivery IDs with a given recipient
- **Escrows by Sender** — all escrow delivery IDs initiated by a given sender
- **Escrows by Recipient** — all escrow delivery IDs for a given recipient
- **Escrows by Driver** — all escrow delivery IDs assigned to a given driver
- **Fleet Rosters** — all drivers (pending and active) in a fleet

These indexes are automatically maintained by the respective contracts and are bounded to prevent unbounded storage growth (max 10,000 entries per index).

#### Query Functions

**Delivery Contract:**
- `get_deliveries_by_sender(sender: Address) -> Vec<DeliveryId>` — all deliveries by sender
- `get_deliveries_by_recipient(recipient: Address) -> Vec<DeliveryId>` — all deliveries to recipient

**Escrow Contract:**
- `get_escrows_by_sender(sender: Address) -> Vec<u64>` — all escrow delivery IDs by sender
- `get_escrows_by_recipient(recipient: Address) -> Vec<u64>` — all escrow delivery IDs for recipient
- `get_escrows_by_driver(driver: Address) -> Vec<u64>` — all escrow delivery IDs for driver

**Fleet Management Contract:**
- `get_fleet_roster(fleet_id: FleetId) -> Vec<Address>` — all drivers in a fleet

### Interim: Event-Replay Indexing

For off-chain applications requiring advanced queries (e.g., deliveries in a specific date range, by cargo category, or paginated results), the recommended interim approach is **event-replay indexing**:

1. **Collect Events** — Replay all contract events from genesis ledger or a known snapshot
2. **Build Local Index** — Maintain a searchable database (e.g., PostgreSQL, Elasticsearch) of parsed events
3. **Subscribe to New Events** — Listen for new events via Soroban RPC and keep the local index current
4. **Query Locally** — Answer complex queries against the local index

**Key Event Types for Indexing:**
- `delivery_created(delivery_id, sender)` — track creation timestamps, sender, cargo details
- `escrow_funded(delivery_id)` — track escrow amounts, tokens
- `driver_assigned(delivery_id, driver)` — track driver assignments
- `delivery_confirmed(delivery_id, recipient)` — track completion
- `delivery_disputed(delivery_id, reporter)` — track disputes
- `invite_accepted(fleet_id, driver)` — track fleet membership changes
- `driver_removed(fleet_id, driver)` — track driver removal

**Advantages:**
- Full historical data and audit trail
- Advanced queries without modifying contracts
- Efficient pagination and filtering
- Off-chain resilience (local index persists independently)

**Disadvantages:**
- Requires external infrastructure
- Potential sync lag between on-chain and off-chain state
- Higher operational complexity

---

## Escrow Contract

Manages financial security for deliveries through locked funds.

### Initialization

#### `init`
Initialize the escrow contract with admin and platform settings.

**Parameters:**
- `admin: Address` - Admin account with privileged operations
- `token: Address` - Token contract used for escrow payments
- `platform_fee_bps: u32` - Platform fee in basis points (e.g., 250 = 2.5%)

**Authorization:** Contract deployer

**Example:**
```rust
escrow_contract.init(
    &admin_address,
    &token_address,
    250 // 2.5% fee
);
```

### Admin Operations

#### `update_platform_fee`
Update the platform fee percentage.

**Parameters:**
- `admin: Address` - Current admin address
- `new_fee_bps: u32` - New fee in basis points (max 1000 = 10%)

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not admin
- `InvalidFee` - Fee exceeds 10%

#### `propose_admin`
Initiate admin transfer to a new address.

**Parameters:**
- `current_admin: Address` - Current admin address
- `new_admin: Address` - Proposed new admin

**Authorization:** Current admin

#### `accept_admin`
Complete admin transfer (called by proposed admin).

**Parameters:**
- `new_admin: Address` - New admin accepting the role

**Authorization:** Proposed admin

#### `set_settlement_contract`
Configure settlement contract for currency swaps.

**Parameters:**
- `admin: Address` - Admin address
- `settlement_contract: Address` - Settlement contract address

**Authorization:** Admin only

### Escrow Lifecycle

#### `create_escrow`
Lock funds for a delivery.

**Parameters:**
- `sender: Address` - Sender funding the escrow
- `recipient: Address` - Delivery recipient
- `driver: Address` - Assigned driver
- `delivery_id: u64` - Unique delivery identifier
- `token: Address` - Token to lock
- `amount: i128` - Amount to lock

**Authorization:** Sender

**Errors:**
- `DuplicateDelivery` - Escrow already exists for this delivery_id
- `InsufficientFunds` - Sender balance too low

**Events:** `escrow_funded`

#### `release_escrow`
Release funds to driver after successful delivery.

**Parameters:**
- `caller: Address` - Recipient or admin
- `delivery_id: u64` - Delivery identifier

**Authorization:** Recipient or Admin

**Errors:**
- `Unauthorized` - Caller not authorized
- `InvalidState` - Escrow not in Locked state
- `DeliveryNotFound` - No escrow for this delivery
- `InsufficientFunds` - Contract balance insufficient

**Events:** `escrow_released`

**State Changes:**
- Transfers (amount - platform_fee) to driver
- Transfers platform_fee to admin
- Sets escrow status to Released

#### `refund_escrow`
Refund funds to sender (e.g., cancelled delivery).

**Parameters:**
- `caller: Address` - Sender or admin
- `delivery_id: u64` - Delivery identifier

**Authorization:** Sender or Admin

**Errors:**
- `Unauthorized` - Caller not authorized
- `InvalidState` - Escrow not in Locked or Paused state
- `DeliveryNotFound` - No escrow for this delivery
- `InsufficientFunds` - Contract balance insufficient

**Events:** `escrow_refunded`

#### `raise_dispute`
Pause escrow for dispute resolution.

**Parameters:**
- `caller: Address` - Sender or recipient
- `delivery_id: u64` - Delivery identifier

**Authorization:** Sender or Recipient

**Errors:**
- `Unauthorized` - Caller not sender or recipient
- `InvalidState` - Escrow not in Locked state

**Events:** `delivery_disputed`

**State Changes:**
- Sets escrow status to Paused
- Records dispute initiator and timestamp

#### `resolve_dispute`
Admin resolution: release to driver or refund to sender.

**Parameters:**
- `caller: Address` - Admin address
- `delivery_id: u64` - Delivery identifier
- `release_to_driver: bool` - true = release, false = refund

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller not admin
- `InvalidState` - Escrow not in Paused state

**Events:** `dispute_resolved`, `escrow_released` or `escrow_refunded`

#### `resolve_dispute_split`
Admin resolution: split funds between sender and driver.

**Parameters:**
- `caller: Address` - Admin address
- `delivery_id: u64` - Delivery identifier
- `sender_share_bps: u32` - Sender's share in basis points (0-10000)

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller not admin
- `InvalidState` - Escrow not in Paused state
- `InvalidFee` - sender_share_bps > 10000

**Events:** `dispute_resolved`

### Query Functions

#### `get_admin`
Returns current admin address.

**Returns:** `Address`

#### `get_token`
Returns configured token address.

**Returns:** `Address`

#### `get_platform_fee`
Returns current platform fee in basis points.

**Returns:** `u32`

#### `get_protocol_version`
Returns protocol version number.

**Returns:** `u32`

#### `get_settlement_contract`
Returns settlement contract address if configured.

**Returns:** `Option<Address>`

#### `get_escrow`
Retrieve full escrow record.

**Parameters:**
- `delivery_id: u64` - Delivery identifier

**Returns:** `EscrowRecord`

**Errors:**
- `DeliveryNotFound` - No escrow for this delivery

#### `create_escrows_batch`
Create multiple escrows in a single transaction (up to 100 per batch).

**Parameters:**
- `sender: Address` - Sender funding all escrows
- `recipient: Address` - Delivery recipient (shared for all)
- `token: Address` - Token for all escrows
- `escrow_list: Vec<(u64, Address, i128)>` — tuples of (delivery_id, driver, amount)

**Authorization:** Sender

**Returns:** `u32` — count of escrows created

**Errors:**
- `DuplicateDelivery` - Escrow already exists for any delivery_id
- `InvalidState` - Batch size exceeds 100

**Events:** `escrow_funded` (once per escrow)

#### `get_escrows_by_sender`
Get all escrow delivery IDs initiated by a sender.

**Parameters:**
- `sender: Address` - Sender address

**Returns:** `Vec<u64>` — list of delivery IDs

#### `get_escrows_by_recipient`
Get all escrow delivery IDs for a recipient.

**Parameters:**
- `recipient: Address` - Recipient address

**Returns:** `Vec<u64>` — list of delivery IDs

#### `get_escrows_by_driver`
Get all escrow delivery IDs assigned to a driver.

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `Vec<u64>` — list of delivery IDs

---

## Delivery Contract

Manages delivery lifecycle and logistics metadata.

### Initialization

#### `init`
Initialize delivery contract.

**Parameters:**
- `admin: Address` - Admin account
- `escrow_contract: Address` - Escrow contract reference

**Authorization:** Contract deployer

### Delivery Operations

#### `create_delivery`
Create a new delivery request.

**Parameters:**
- `sender: Address` - Sender creating delivery
- `recipient: Address` - Delivery recipient
- `metadata: DeliveryMetadata` - Logistics details

**Authorization:** Sender

**Returns:** `DeliveryId`

**Events:** `delivery_created`

**State Changes:**
- Increments delivery counter
- Stores delivery record with Pending status
- Sets creation timestamp

#### `assign_driver`
Assign a driver to a delivery.

**Parameters:**
- `caller: Address` - Admin or the driver self-assigning
- `delivery_id: DeliveryId` - Delivery identifier
- `driver: Address` - Driver to assign

**Authorization:** Admin or Driver (self-assignment)

**Errors:**
- `NotAuthorized` - Caller not admin or driver
- `DeliveryNotFound` - Invalid delivery_id
- `InvalidState` - Delivery not in Pending state

**Events:** `driver_assigned`

**State Changes:**
- Sets delivery.driver to specified address
- Updates status to Active

#### `mark_in_transit`
Driver marks delivery as actively in transit.

**Parameters:**
- `driver: Address` - Driver address
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Assigned driver only

**Errors:**
- `NotAuthorized` - Caller is not assigned driver
- `InvalidState` - Delivery not in Active state

**Events:** `DeliveryInTransit`

**State Changes:**
- Updates status to InTransit
- Records transit_started_at timestamp

#### `confirm_delivery`
Recipient confirms successful delivery.

**Parameters:**
- `recipient: Address` - Recipient address
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Recipient only

**Errors:**
- `NotAuthorized` - Caller is not recipient
- `InvalidState` - Delivery not in InTransit state
- `EscrowNotConfigured` - Escrow contract not set

**Events:** `delivery_confirmed`

**State Changes:**
- Updates status to Delivered
- Records delivered_at timestamp
- Calls escrow_contract.release_escrow
- Increments driver's deliveries_completed
- Increases driver's reputation_score

#### `cancel_delivery`
Sender cancels a delivery.

**Parameters:**
- `sender: Address` - Sender address
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Sender only

**Errors:**
- `NotAuthorized` - Caller is not sender
- `InvalidState` - Invalid state transition

**Events:** `delivery_cancelled`

**State Changes:**
- Updates status to Cancelled
- Calls escrow_contract.refund_escrow

#### `raise_dispute`
Sender or recipient raises a dispute.

**Parameters:**
- `caller: Address` - Sender or recipient
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Sender or Recipient

**Errors:**
- `NotAuthorized` - Caller not sender or recipient
- `InvalidState` - Cannot transition to Disputed

**Events:** `delivery_disputed`

**State Changes:**
- Updates status to Disputed
- Calls escrow_contract.raise_dispute to pause funds

### Query Functions

#### `get_delivery`
Retrieve full delivery record.

**Parameters:**
- `delivery_id: DeliveryId` - Delivery identifier

**Returns:** `DeliveryRecord`

**Errors:**
- `DeliveryNotFound` - Invalid delivery_id

#### `create_deliveries_batch`
Create multiple deliveries in a single transaction (up to 100 per batch).

**Parameters:**
- `sender: Address` - Sender creating all deliveries
- `recipient: Address` - Recipient for all deliveries (shared)
- `metadata_list: Vec<DeliveryMetadata>` — delivery metadata for each delivery

**Authorization:** Sender

**Returns:** `Vec<DeliveryId>` — list of created delivery IDs

**Errors:**
- `BatchTooLarge` - Metadata list exceeds 100 items

**Events:** `delivery_created` (once per delivery)

**State Changes:**
- Increments delivery counter for each delivery
- Stores delivery records with Pending status
- Updates secondary indexes for sender and recipient

#### `get_deliveries_by_sender`
Get all delivery IDs initiated by a sender.

**Parameters:**
- `sender: Address` - Sender address

**Returns:** `Vec<DeliveryId>` — list of delivery IDs

#### `get_deliveries_by_recipient`
Get all delivery IDs with a specific recipient.

**Parameters:**
- `recipient: Address` - Recipient address

**Returns:** `Vec<DeliveryId>` — list of delivery IDs

#### `get_driver_profile`
Get driver statistics and reputation.

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `DriverProfile`

---

## Fleet Management Contract

Manages driver fleet organization and membership.

### Initialization

#### `init`
Initialize fleet management contract.

**Parameters:**
- `admin: Address` - Admin account

**Authorization:** Contract deployer

### Fleet Operations

#### `register_fleet`
Register a new fleet.

**Parameters:**
- `owner: Address` - Fleet owner (caller)
- `treasury: Address` - Fleet treasury wallet

**Authorization:** Owner

**Returns:** `FleetId` — new fleet identifier

**Events:** `fleet_registered`

#### `get_fleet`
Retrieve fleet profile.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `FleetProfile`

**Errors:**
- `FleetNotFound` - Invalid fleet_id

#### `update_fleet_treasury`
Update treasury wallet for a fleet.

**Parameters:**
- `owner: Address` - Fleet owner (caller)
- `fleet_id: FleetId` - Fleet identifier
- `treasury: Address` - New treasury address

**Authorization:** Fleet owner

**Errors:**
- `FleetNotFound` - Invalid fleet_id
- `Unauthorized` - Caller not fleet owner

**Events:** `fleet_treasury_updated`

#### `add_driver_to_fleet`
Invite a driver to a fleet (owner only).

**Parameters:**
- `caller: Address` - Fleet owner
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver address

**Authorization:** Fleet owner

**Errors:**
- `FleetNotFound` - Invalid fleet_id
- `Unauthorized` - Caller not fleet owner
- `DriverAlreadyInvited` - Driver already invited
- `DriverAlreadyActive` - Driver already active

**Events:** `driver_invited`

#### `accept_fleet_invite`
Accept pending fleet invite (driver-initiated).

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver address (caller)

**Authorization:** Driver

**Errors:**
- `FleetNotFound` - Invalid fleet_id
- `InviteNotFound` - No pending invite
- `DriverAlreadyActive` - Driver already active

**Events:** `invite_accepted`

**State Changes:**
- Transitions driver status from Pending → Active
- Increments fleet's total_active_drivers
- Adds driver to fleet roster

#### `remove_driver_from_fleet`
Remove driver from fleet (owner or driver can initiate).

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `caller: Address` - Fleet owner or driver
- `driver: Address` - Driver to remove

**Authorization:** Fleet owner or driver

**Errors:**
- `FleetNotFound` - Invalid fleet_id
- `InviteNotFound` - Driver not in fleet
- `Unauthorized` - Caller not owner or driver

**Events:** `driver_removed`

**State Changes:**
- Deletes driver's fleet record
- Decrements fleet's total_active_drivers (if active)
- Removes driver from fleet roster

#### `get_payout_address`
Determine where payout funds should go for a driver in a fleet.

**Parameters:**
- `driver: Address` - Driver address
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `Address` — fleet treasury if active, else driver's own address

#### `get_driver_fleet_status`
Get driver's status in a fleet.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver address

**Returns:** `Option<DriverFleetStatus>` — Pending, Active, or None

### Enumeration

#### `get_fleet_roster`
Get all drivers in a fleet (both pending and active).

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `Vec<Address>` — list of driver addresses

---

## Shared Types

### Enums

#### `DeliveryStatus`
```rust
pub enum DeliveryStatus {
    Pending,    // Created, awaiting driver
    Active,     // Driver assigned
    InTransit,  // Driver confirmed pickup
    Delivered,  // Recipient confirmed
    Disputed,   // Under dispute resolution
    Cancelled,  // Cancelled by sender
}
```

**Valid Transitions:**
- Pending → Active, Cancelled
- Active → InTransit, Disputed, Cancelled
- InTransit → Delivered, Disputed
- Disputed → Delivered, Cancelled
- Delivered, Cancelled → (terminal states)

#### `EscrowState`
```rust
pub enum EscrowState {
    Locked,    // Funds secured, awaiting release/refund
    Released,  // Funds paid to driver
    Refunded,  // Funds returned to sender
    Paused,    // Frozen due to dispute
}
```

#### `CargoCategory`
```rust
pub enum CargoCategory {
    Documents,
    Electronics,
    Perishables,
    Clothing,
    General,
}
```

### Structs

#### `DeliveryRecord`
```rust
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
```

#### `EscrowRecord`
```rust
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
```

#### `DeliveryMetadata`
```rust
pub struct DeliveryMetadata {
    pub delivery_id: u64,
    pub origin: String,
    pub destination: String,
    pub cargo_description: CargoDescriptor,
    pub created_at: u64,
    pub estimated_delivery: u64,
}
```

#### `CargoDescriptor`
```rust
pub struct CargoDescriptor {
    pub weight_grams: u32,
    pub category: CargoCategory,
    pub fragile: bool,
}
```

#### `DriverProfile`
```rust
pub struct DriverProfile {
    pub address: Address,
    pub deliveries_completed: u32,
    pub reputation_score: u32,
    pub registered_at: u64,
    pub kyc_verified: bool,
}
```

### Errors

#### `FaniLabError`
```rust
pub enum FaniLabError {
    Unauthorized = 1,           // Not authorized for this operation
    AlreadyInitialized = 2,     // Contract already initialized
    NotInitialized = 3,         // Contract not initialized
    DeliveryNotFound = 4,       // Invalid delivery ID
    InvalidState = 5,           // Invalid state transition
    InsufficientFunds = 6,      // Balance too low
    EscrowLocked = 7,           // Escrow cannot be modified
    DuplicateDelivery = 8,      // Delivery ID exists
    ProviderNotFound = 9,       // Driver not found
    InvalidAddress = 10,        // Invalid address parameter
}
```

### Events

All events are defined in `shared_types::events`:

- `delivery_created` - New delivery created
- `escrow_funded` - Funds locked in escrow
- `driver_assigned` - Driver assigned to delivery
- `delivery_confirmed` - Recipient confirmed delivery
- `escrow_released` - Funds released to driver
- `delivery_disputed` - Dispute raised
- `escrow_refunded` - Funds returned to sender
- `dispute_resolved` - Dispute resolved by admin

---

## Error Handling

All contract functions that can fail return Soroban errors via `panic_with_error!` macro.

**Error Handling Best Practices:**
1. Check return status codes
2. Parse error discriminant from `Status` object
3. Match against error enum values
4. Implement retry logic for network failures
5. Log all errors for debugging

---

## Rate Limits & Constraints

### Soroban Limits
- Max contract size: 64 KB (WASM)
- Max CPU instructions per invocation: configurable
- Max memory: 40 MB
- Max storage entry size: 64 KB
- Max ledger entries per invocation: 256

### FaniLab Constraints
- Platform fee: 0% - 10% (10,000 basis points)
- Delivery ID: u64 (18 quintillion max)
- String fields: Limited by storage entry size
- TTL: 518,400 ledgers (~30 days default)

---

## SDKs and Client Libraries

### JavaScript/TypeScript
```typescript
import { Contract, networks } from '@stellar/stellar-sdk';

const escrow = new Contract(escrowContractId);
await escrow.call('release_escrow', recipient, deliveryId);
```

### Rust
```rust
use escrow_contract::EscrowContractClient;

let client = EscrowContractClient::new(&env, &contract_id);
client.release_escrow(&recipient, &delivery_id);
```

---

**API Version**: 1.0.0  
**Last Updated**: January 2026  
**Soroban SDK**: 22.0.1

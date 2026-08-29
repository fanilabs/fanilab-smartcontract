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
- `get_fleet_roster(fleet_id: FleetId) -> Vec<Address>` — active drivers in a fleet

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

**Example:**
```rust
escrow_contract.update_platform_fee(
    &admin_address,
    500 // 5% fee
);
```

#### `propose_admin`
Initiate admin transfer to a new address.

**Parameters:**
- `current_admin: Address` - Current admin address
- `new_admin: Address` - Proposed new admin

**Authorization:** Current admin

**Example:**
```rust
escrow_contract.propose_admin(
    &current_admin,
    &new_admin_address
);
```

#### `accept_admin`
Complete admin transfer (called by proposed admin).

**Parameters:**
- `new_admin: Address` - New admin accepting the role

**Authorization:** Proposed admin

**Example:**
```rust
escrow_contract.accept_admin(&new_admin_address);
```

#### `set_settlement_contract`
Configure settlement contract for currency swaps.

**Parameters:**
- `admin: Address` - Admin address
- `settlement_contract: Address` - Settlement contract address

**Authorization:** Admin only

#### `confirm_settlement_contract`
Apply a previously proposed settlement-contract change once its timelock has
elapsed. `set_settlement_contract` only *proposes* the change; this function
makes it effective. If the `SETTLEMENT_CONTRACT_TIMELOCK_SECONDS` (3 days)
window has not yet passed, the call fails, so a compromised admin key cannot
silently repoint payouts without the delay window. See
[`set_settlement_contract`](#set_settlement_contract),
[`get_pending_settlement_contract`](#get_pending_settlement_contract), and
[`clear_settlement_contract`](#clear_settlement_contract) for the full timelock
trio.

**Parameters:**
- `admin: Address` - Admin address

**Authorization:** Admin only

**Errors:**
- `NoPendingSettlementChange` - No settlement-contract proposal is awaiting confirmation
- `TimelockNotElapsed` - The 3-day timelock has not yet elapsed

**Events:** `settlement_contract_updated`

**Example:**
```rust
escrow_contract.confirm_settlement_contract(&admin_address);
```

#### `clear_settlement_contract`
Unset a previously configured settlement contract. After clearing,
`get_settlement_contract` returns `None` and payouts stop routing through
settlement swaps. Also removes any pending (timelocked) settlement-contract
change. Clearing when nothing is configured is a no-op that still succeeds.

**Parameters:**
- `admin: Address` - Admin address

**Authorization:** Admin only

**Example:**
```rust
escrow_contract.clear_settlement_contract(&admin_address);
```

#### `set_fleet_management_contract`
Configure the fleet-management contract consulted during driver payouts. When
set, `payout_driver` calls `get_payout_address(driver, fleet_id)` on it for any
escrow that carries a `fleet_id` and sends the driver's earnings to the address
it returns.

**Parameters:**
- `admin: Address` - Admin address
- `fleet_contract: Address` - Fleet-management contract address

**Authorization:** Admin only

**Example:**
```rust
escrow_contract.set_fleet_management_contract(
    &admin_address,
    &fleet_contract_address
);
```

#### `clear_fleet_management_contract`
Unset a previously configured fleet-management contract (Issue #239), mirroring
`clear_settlement_contract`. After clearing, `get_fleet_management_contract`
returns `None` and payouts for fleet-linked escrows fall back to paying the
driver directly instead of routing through a cross-contract
`get_payout_address` call. Clearing when nothing is configured is a no-op that
still succeeds.

**Parameters:**
- `admin: Address` - Admin address

**Authorization:** Admin only

**Note — no `clear_dispute_resolution_contract`:** `set_dispute_resolution_contract`
deliberately has no clearing counterpart. `freeze_funds` pins its caller to the
configured dispute-resolution contract and reads that address expecting it to be
present, so unsetting it would permanently disable the protocol's ability to
freeze a suspicious escrow. The intended remedy for a misbehaving dispute
contract is to repoint it with `set_dispute_resolution_contract`, not to remove
the integration.

#### `set_dispute_resolution_contract`
Configure the dispute-resolution contract consulted by `freeze_funds`. When set,
`freeze_funds` delegates the dispute to it via `get_driver_preference`; the
address is expected to remain present (see the note above — there is no unsetting
counterpart).

**Parameters:**
- `admin: Address` - Admin address
- `dispute_contract: Address` - Dispute-resolution contract address

**Authorization:** Admin only

**Example:**
```rust
escrow_contract.set_dispute_resolution_contract(
    &admin_address,
    &dispute_contract_address
);
```

#### `update_slippage_tolerance`
Update the slippage tolerance (in basis points) used when settlement swaps are
executed. The default is 500 (5%); the maximum is 10000 (100%).

**Parameters:**
- `admin: Address` - Admin address
- `new_slippage_bps: u32` - New slippage tolerance in basis points (max 10000)

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not admin
- `InvalidFee` - Slippage tolerance exceeds 10000

**Example:**
```rust
escrow_contract.update_slippage_tolerance(
    &admin_address,
    300 // 3% slippage tolerance
);
```

#### `set_volume_tiers`
Replace the volume-discount tier table used to compute sender fee discounts.
Each tier maps a sender-volume threshold (see `get_sender_volume`) to a discount
(in basis points) subtracted from the base platform fee. Tiers must be strictly
ascending by `volume_threshold` (no duplicates, no descending runs) and each
`discount_bps` must not exceed the max platform fee (1000 = 10%).

**Parameters:**
- `admin: Address` - Admin address
- `tiers: Vec<VolumeTier>` - Ordered list of `VolumeTier { volume_threshold: u32, discount_bps: u32 }`

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not admin
- `InvalidFee` - A tier has `discount_bps > 1000`, or thresholds are not strictly ascending (duplicate or descending)

**Events:** `volume_tiers_updated`

**Example:**
```rust
let mut tiers = Vec::new(&env);
tiers.push_back(&VolumeTier { volume_threshold: 0,      discount_bps: 0   });
tiers.push_back(&VolumeTier { volume_threshold: 1_000,  discount_bps: 100 });
tiers.push_back(&VolumeTier { volume_threshold: 10_000, discount_bps: 250 });
escrow_contract.set_volume_tiers(&admin_address, &tiers);
```

#### `set_paused`
Emergency circuit breaker. When paused, blocks every operation that creates a
new escrow or moves funds out of the contract.

**Parameters:**
- `admin: Address` - Admin address
- `paused: bool` - New pause state

**Authorization:** Admin only

**Blocked while paused:** `create_escrow`, `create_escrows_batch`,
`mark_holdback_escrow`, `release_escrow`, `refund_escrow`, `resolve_dispute`,
`resolve_dispute_split`, `release_holdback_escrow`, `reclaim_expired_escrow`
— each panics with `ProtocolPaused` (error code 11).

**Remains available while paused:** `freeze_funds` and `raise_dispute`
(neither moves funds — both only transition an escrow into the disputed
`Paused` state, so admins/the dispute contract can still flag a suspicious
escrow during an incident) and `sweep_untracked_balance` (already
restricted to admin-only, used for recovering stray token balances).

**Example:**
```rust
escrow_contract.set_paused(&admin_address, true);  // halt fund movement
escrow_contract.set_paused(&admin_address, false); // resume
```

#### `is_paused`
Returns the current protocol pause state.

**Parameters:** None

**Returns:** `bool`

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

**Example:**
```rust
escrow_contract.create_escrow(
    &sender,
    &recipient,
    &driver,
    1u64,              // delivery_id
    &usdc_token,       // token address
    50_000_000i128     // 50 USDC (6 decimals)
);
```

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

**Example:**
```rust
escrow_contract.release_escrow(
    &recipient,
    1u64  // delivery_id
);
```

#### `refund_escrow`
Refund funds to the sender for an escrow that is still refundable.

**Parameters:**
- `caller: Address` - Sender or admin
- `delivery_id: u64` - Delivery identifier

**Authorization:**
- from `Locked`: sender or admin
- from `Holdback` or `Paused`: admin only
- from `Released`, `Refunded`, or `Split`: rejected with `InvalidState`

**Errors:**
- `Unauthorized` - Caller not authorized for that state
- `InvalidState` - Escrow not in `Locked`, `Paused`, or `Holdback`
- `DeliveryNotFound` - No escrow for this delivery
- `InsufficientFunds` - Contract balance insufficient

**Events:** `escrow_refunded`

**Example:**
```rust
// Locked escrow: sender may self-refund.
escrow_contract.refund_escrow(
    &sender,
    1u64  // delivery_id
);

// Disputed / holdback escrow: admin-only refund.
escrow_contract.refund_escrow(
    &admin,
    2u64  // delivery_id
);
```

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

**Example:**
```rust
escrow_contract.resolve_dispute(
    &admin,
    1u64,   // delivery_id
    true    // true = release to driver
);
```

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

**Example:**
```rust
escrow_contract.resolve_dispute_split(
    &admin,
    1u64,    // delivery_id
    6000     // 60% to sender, 40% to driver
);
```

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

#### `get_pending_settlement_contract`
Returns the pending (timelocked) settlement-contract change, if any, so
off-chain clients can display the upcoming payout-routing change during its
3-day timelock window. See [`set_settlement_contract`](#set_settlement_contract),
[`confirm_settlement_contract`](#confirm_settlement_contract), and
[`clear_settlement_contract`](#clear_settlement_contract) for the full timelock
trio. Returns `None` when no change is pending.

**Returns:** `Option<PendingSettlementContract>` — `{ settlement_contract: Address, activates_at: u64 }`

#### `get_fleet_management_contract`
Returns the configured fleet-management contract address, or `None` if none is
set or it has been cleared with `clear_fleet_management_contract`.

**Returns:** `Option<Address>`

#### `get_dispute_resolution_contract`
Returns the configured dispute-resolution contract address, or `None` if none
is set.

**Returns:** `Option<Address>`

#### `get_slippage_tolerance`
Returns the current slippage tolerance in basis points (default 500 = 5%) used
when settlement swaps are executed.

**Returns:** `u32`

#### `get_total_locked`
Returns the total amount of `token` currently locked across all escrows. This
is the key metric referenced by `docs/MONITORING.md` and the basis for
`get_untracked_balance` / `sweep_untracked_balance`.

**Parameters:**
- `token: Address` - The token to query

**Returns:** `i128`

#### `get_sender_volume`
Returns the sender's payout count, incremented each time a payout is executed
for the sender, used to select the applicable volume-discount tier. Returns `0`
for a sender with no payouts yet.

**Parameters:**
- `sender: Address` - Sender address

**Returns:** `u32`

#### `get_volume_tiers`
Returns the configured volume-discount tier table as an ordered list of
`VolumeTier { volume_threshold, discount_bps }`. Returns an empty `Vec` if none
has been set.

**Returns:** `Vec<VolumeTier>`

#### `get_escrow`
Retrieve full escrow record.

**Parameters:**
- `delivery_id: u64` - Delivery identifier

**Returns:** `EscrowRecord`

**Errors:**
- `DeliveryNotFound` - No escrow for this delivery

#### `create_escrows_batch`
Create multiple escrows in a single transaction (up to 100 per batch). Enforces
the same token and amount validation as `create_escrow`.

**Parameters:**
- `sender: Address` - Sender funding all escrows
- `recipient: Address` - Delivery recipient (shared for all)
- `token: Address` - Token for all escrows; must match the protocol-configured token
- `escrow_list: Vec<(u64, Address, i128)>` — tuples of (delivery_id, driver, amount)

**Authorization:** Sender

**Returns:** `u32` — count of escrows created

**Errors:**
- `InvalidToken` - Token does not match the protocol-configured token
- `InvalidAmount` - Any element's amount is not positive
- `DuplicateDelivery` - Escrow already exists for any delivery_id
- `InvalidState` - Batch size exceeds 100

**Events:** `escrow_funded` (once per escrow)

> **IMPORTANT — Integration Requirement:** This function is designed to pair with `delivery_contract::create_deliveries_batch`. The delivery IDs passed in `escrow_list` must have been created by `create_deliveries_batch` first. Call this function after receiving delivery IDs from the batch delivery creation, passing (delivery_id, driver, amount) tuples for each delivery that needs escrow backing.

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

**Example:**
```rust
use shared_types::{DeliveryMetadata, CargoDescriptor, CargoCategory};

let metadata = DeliveryMetadata {
    delivery_id: 1,
    origin: String::from_str(&env, "New York"),
    destination: String::from_str(&env, "Los Angeles"),
    cargo_description: CargoDescriptor {
        weight_grams: 50000,
        category: CargoCategory::Electronics,
        fragile: true,
    },
    created_at: env.ledger().timestamp(),
    estimated_delivery: env.ledger().timestamp() + 86400 * 3,
};

let delivery_id = delivery_contract.create_delivery(
    &sender,
    &recipient,
    &metadata
);
```

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

**Example:**
```rust
delivery_contract.assign_driver(
    &admin,
    &delivery_id,
    &driver
);
```

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

**Example:**
```rust
delivery_contract.mark_in_transit(&driver, &delivery_id);
```

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

**Example:**
```rust
delivery_contract.confirm_delivery(&recipient, &delivery_id);
```

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

**Example:**
```rust
delivery_contract.cancel_delivery(&sender, &delivery_id);
```

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

**Example:**
```rust
delivery_contract.raise_dispute(&sender, &delivery_id);
```

### Query Functions

#### `get_delivery`
Retrieve full delivery record.

**Parameters:**
- `delivery_id: DeliveryId` - Delivery identifier

**Returns:** `DeliveryRecord`

**Errors:**
- `DeliveryNotFound` - Invalid delivery_id

#### `update_delivery_metadata`
Update a delivery's metadata while it is still `Pending` and only by the original sender.

**Parameters:**
- `sender: Address` - Original sender of the delivery
- `delivery_id: DeliveryId` - Delivery identifier
- `metadata: DeliveryMetadata` - Replacement metadata payload

**Authorization:** Sender only

**Errors:**
- `Unauthorized` - Caller is not the sender
- `InvalidState` - Delivery is not `Pending`
- `InvalidMetadata` - The replacement metadata fails validation
- `DeliveryNotFound` - No delivery exists for this ID

**Events:** `delivery_metadata_updated`

#### `get_combined_state`
Return the delivery record, escrow record, and a synchronization flag for the same delivery ID.

**Parameters:**
- `delivery_id: DeliveryId` - Delivery identifier

**Returns:** `(DeliveryRecord, EscrowRecord, bool)`

**Semantics:**
- `true` means the delivery and escrow states match the protocol invariants
- `false` indicates a mismatch such as a `Disputed` delivery with a non-paused escrow or a `Cancelled` delivery whose escrow is not `Refunded`

**Examples of synchronized pairs:**
- `Pending` ↔ `Locked`
- `Active` ↔ `Locked`
- `InTransit` ↔ `Locked`
- `Delivered` ↔ `Holdback` or `Released`
- `Disputed` ↔ `Paused`
- `Cancelled` ↔ `Refunded`

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

> **IMPORTANT — Integration Requirement:** This function creates delivery records only; it does NOT create escrows. Escrow creation must be performed as a separate operation using `escrow_contract::create_escrows_batch`. The two operations must be paired in sequence:
>
> 1. Call `create_deliveries_batch` → returns `Vec<DeliveryId>`
> 2. Call `escrow_contract::create_escrows_batch` with the returned delivery IDs and (driver, amount) pairs
>
> Deliveries without escrows will fail at driver assignment or confirmation stages with `DeliveryNotFound` errors. The ordering constraint exists because delivery IDs must be known before escrows can reference them.

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

#### `get_escrow_contract`
Return the escrow_contract address this delivery_contract was initialised with.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `get_identity_reputation_contract`
Return the configured identity_reputation_contract address, if any.

**Returns:** `Option<Address>`

---

## Dispute Resolution Contract

Handles the full lifecycle of delivery disputes — evidence submission, resolution
verdicts, and cross-contract calls to freeze/release escrow funds and penalise
driver reputation.

### Types

#### `DisputeStatus`
```rust
pub enum DisputeStatus {
    Open,             // Dispute raised, awaiting admin verdict
    ResolvedRefund,   // Admin resolved: funds returned to sender
    ResolvedPayout,   // Admin resolved: funds released to driver
    Split,            // Admin resolved: funds split between parties
}
```

#### `EvidenceEntry`
```rust
pub struct EvidenceEntry {
    pub submitter: Address,     // Party that submitted this hash
    pub hash:      BytesN<32>,  // SHA-256 hash of the evidence document/image
}
```

#### `DisputeCase`
```rust
pub struct DisputeCase {
    pub delivery_id:     DeliveryId,
    pub status:          DisputeStatus,
    pub raised_at:       u64,
    pub raised_by:       Address,
    pub evidence_hashes: Vec<EvidenceEntry>,  // recorded with the submitting party
    pub resolved_at:     Option<u64>,
    pub resolved_by:     Option<Address>,
}
```

### Initialization

#### `init`
Initialize the dispute resolution contract.

**Parameters:**
- `admin: Address` - Initial admin address
- `delivery_contract: Address` - Address of the delivery contract
- `escrow_contract: Address` - Address of the escrow contract
- `dispute_time_limit: u64` - Seconds after delivery within which a dispute may be raised (must be ≥ `MIN_DISPUTE_TIME_LIMIT`, 1 day)
- `dispute_resolution_limit: u64` - Seconds a dispute may stay `Open` before any party may `force_resolve_dispute` (must be ≥ `MIN_DISPUTE_RESOLUTION_LIMIT`, 1 day)

**Authorization:** Contract deployer

**Errors:**
- `AlreadyInitialized` - Contract has already been initialized
- `InvalidState` - `dispute_time_limit` or `dispute_resolution_limit` is below its floor

### Admin Operations

#### `add_admin`
Grant admin privileges to a new address.

**Parameters:**
- `caller: Address` - Current admin
- `new_admin: Address` - Address to promote

**Authorization:** Existing admin

**Errors:**
- `Unauthorized` - Caller is not an admin

**Example:**
```rust
dispute_contract.add_admin(&current_admin, &new_admin);
```

#### `remove_admin`
Revoke admin privileges from an address.

**Parameters:**
- `caller: Address` - Current admin performing the removal
- `old_admin: Address` - Address to demote

**Authorization:** Existing admin

**Errors:**
- `Unauthorized` - Caller is not an admin

**Example:**
```rust
dispute_contract.remove_admin(&current_admin, &old_admin);
```

#### `set_identity_reputation_contract`
Configure the identity/reputation contract address used for reputation penalties.

**Parameters:**
- `caller: Address` - Admin address
- `reputation_contract: Address` - Address of the identity reputation contract

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin

#### `set_dispute_reputation_penalty`
Set the flat driver reputation penalty applied when a dispute resolves in the sender's favour.

**Parameters:**
- `caller: Address` - Admin address
- `penalty: u32` - New penalty value (must be at most `MAX_DISPUTE_REPUTATION_PENALTY`)

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `InvalidState` - New penalty exceeds the configured maximum

#### `set_dispute_resolution_limit`
Set the dispute auto-resolution window.

**Parameters:**
- `caller: Address` - Admin address
- `new_limit: u64` - New dispute-resolution limit in seconds

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `InvalidState` - `new_limit` is below `MIN_DISPUTE_RESOLUTION_LIMIT` (86400 seconds)

#### `update_dispute_time_limit`
Update the post-delivery dispute window.

**Parameters:**
- `caller: Address` - Admin address
- `new_limit: u64` - New time limit in seconds

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `InvalidState` - `new_limit` is below `MIN_DISPUTE_TIME_LIMIT` (86400 seconds)

### Query Functions

#### `is_admin`
Check whether an address holds admin privileges.

**Parameters:**
- `admin: Address` - Address to query

**Returns:** `bool`

#### `get_delivery_contract`
Return the configured delivery contract address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `get_escrow_contract`
Return the configured escrow contract address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `get_identity_reputation_contract`
Return the configured identity/reputation contract address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Identity contract address not set

#### `get_dispute_time_limit`
Return the dispute time limit in seconds.

**Returns:** `u64`

#### `list_admins`
List the current dispute-resolution admins.

**Returns:** `Vec<Address>`

#### `get_dispute_reputation_penalty`
Return the configured dispute reputation penalty. If unset, the contract falls back to the default of 10 points.

**Returns:** `u32`

#### `get_dispute_resolution_limit`
Return the forced-resolution timeout used by `force_resolve_dispute`.

**Returns:** `u64`

#### `get_dispute`
Retrieve a full dispute record by delivery ID.

**Parameters:**
- `delivery_id: DeliveryId` - Delivery identifier

**Returns:** `DisputeCase`

**Errors:**
- `DeliveryNotFound` - No dispute exists for this delivery

### Dispute Lifecycle

#### `raise_dispute`
Open a dispute for an active, in-transit, or recently delivered delivery.

**Parameters:**
- `caller: Address` - Sender or recipient of the delivery
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Delivery sender or recipient

**Errors:**
- `Unauthorized` - Caller is neither sender nor recipient
- `InvalidState` - Delivery is in a non-disputable state, or the post-delivery dispute window has closed
- `DuplicateDelivery` - A dispute already exists for this delivery

**Events:** `dispute_raised`

**State Changes:**
- Creates a `DisputeCase` record with `DisputeStatus::Open`
- Calls `delivery_contract.raise_dispute` to transition delivery to `Disputed`
- Calls `escrow_contract.freeze_funds` to pause the escrow

#### `add_evidence_hash`
Attach a SHA-256 evidence hash to an open dispute.

**Parameters:**
- `caller: Address` - Sender, recipient, or driver submitting evidence
- `delivery_id: DeliveryId` - Delivery identifier
- `evidence_hash: BytesN<32>` - SHA-256 hash of the evidence document/image

**Authorization:** Delivery sender, recipient, or driver

**Errors:**
- `DeliveryNotFound` - No dispute exists for this delivery
- `InvalidState` - Dispute is not in `Open` status, or the calling party has already submitted this exact hash
- `Unauthorized` - Caller is not a party to the delivery
- `LimitExceeded` - The calling party has reached its per-party quota of 20 evidence hashes for this dispute

**Events:** `evidence_added`

**State Changes:**
- Appends `EvidenceEntry { submitter: caller, hash: evidence_hash }` to `DisputeCase.evidence_hashes`. The 20-hash cap is enforced **per submitting party**, so one party can neither exhaust another's quota nor lock the counterparty out.

#### `resolve_dispute_refund_sender`
Admin verdict: full refund to sender. Applies a reputation penalty to the driver.

**Parameters:**
- `caller: Address` - Admin address
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `DeliveryNotFound` - No dispute exists for this delivery
- `InvalidState` - Dispute is not in `Open` status
- `ProviderNotFound` - No driver assigned to the delivery

**Events:** `dispute_resolved_refund`

**State Changes:**
- Sets `DisputeCase.status` to `ResolvedRefund`
- Calls `identity_reputation_contract.decrease_reputation` (−10 points, if configured)
- Calls `escrow_contract.resolve_dispute` with `release_to_driver = false`

**Example:**
```rust
dispute_contract.resolve_dispute_refund_sender(&admin, &delivery_id);
```

#### `resolve_dispute_pay_driver`
Admin verdict: full payout to driver.

**Parameters:**
- `caller: Address` - Admin address
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `DeliveryNotFound` - No dispute exists for this delivery
- `InvalidState` - Dispute is not in `Open` status

**Events:** `dispute_resolved_payout`

**State Changes:**
- Sets `DisputeCase.status` to `ResolvedPayout`
- Calls `escrow_contract.resolve_dispute` with `release_to_driver = true`

**Example:**
```rust
dispute_contract.resolve_dispute_pay_driver(&admin, &delivery_id);
```

#### `resolve_dispute_split_funds`
Admin verdict: split escrow funds between sender and driver.

**Parameters:**
- `caller: Address` - Admin address
- `delivery_id: DeliveryId` - Delivery identifier
- `sender_share_bps: u32` - Sender's share of the escrow in basis points (0–10 000)

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not an admin
- `DeliveryNotFound` - No dispute exists for this delivery
- `InvalidState` - Dispute is not in `Open` status

**Events:** `dispute_resolved_split`

**State Changes:**
- Sets `DisputeCase.status` to `Split`
- Calls `escrow_contract.resolve_dispute_split` with the specified basis-point split

**Example:**
```rust
dispute_contract.resolve_dispute_split_funds(
    &admin,
    &delivery_id,
    5000  // 50/50 split
);
```

#### `force_resolve_dispute`
Allow any party to the delivery to force a default 50/50 split once the configured resolution window has elapsed.

**Parameters:**
- `caller: Address` - Sender, recipient, or assigned driver
- `delivery_id: DeliveryId` - Delivery identifier

**Authorization:** Any delivery party; the caller must be the sender, recipient, or the assigned driver.

**Errors:**
- `DeliveryNotFound` - No dispute exists for this delivery
- `Unauthorized` - Caller is not a party to the delivery
- `InvalidState` - Dispute is not `Open`, or the resolution limit has not yet elapsed

**Events:** `dispute_force_resolved`

**State Changes:**
- Marks the dispute as `Split`
- Records the forced resolution timestamp and resolver
- Calls `escrow_contract.resolve_dispute_split` with a 50/50 default sender/driver split

**Example:**
```rust
dispute_contract.force_resolve_dispute(&sender, &delivery_id);
```

---

## Fleet Management Contract

Manages fleets of drivers — fleet registration, treasury configuration, driver
invitations, and payout routing.

### Types

#### `FleetId`
```rust
pub type FleetId = u64;
```

#### `DriverFleetStatus`
```rust
pub enum DriverFleetStatus {
    Pending,  // Driver invited but has not yet accepted
    Active,   // Driver accepted and is an active fleet member
    Removed,  // Driver was removed from the fleet history
}
```

#### `FleetProfile`
```rust
pub struct FleetProfile {
    pub fleet_id:             FleetId,
    pub owner:                Address,
    pub treasury:             Address,
    pub active:               bool,
    pub total_active_drivers: u32,
    pub signers:              Vec<Address>,
    pub signature_threshold:  u32,
}
```

### Initialization

#### `init`
Initialize the fleet management contract.

**Parameters:**
- `admin: Address` - Contract administrator

**Authorization:** Contract deployer

**Errors:**
- `AlreadyInitialized` - Contract has already been initialized

**State Changes:**
- Sets the protocol admin
- Resets the fleet counter to `0`

#### `set_identity_contract`
Configure the identity/reputation contract used for automatic driver profile creation on fleet registration.

**Parameters:**
- `admin: Address` - Admin address
- `identity_contract: Address` - Address of the identity reputation contract

**Authorization:** Admin only

**Errors:**
- `NotInitialized` - Contract has not been initialized
- `Unauthorized` - Caller is not the stored admin

### Fleet Operations

#### `register_fleet`
Register a new fleet, returning its assigned fleet ID.

**Parameters:**
- `owner: Address` - Fleet owner (must sign the transaction)
- `treasury: Address` - Wallet receiving driver payouts for this fleet

**Authorization:** Owner (must sign)

**Returns:** `FleetId`

**Errors:**
- `NotInitialized` - Fleet counter not found (contract not initialized)

**Events:** `fleet_registered`

**State Changes:**
- Increments and persists the fleet counter
- Creates and stores a `FleetProfile`
- Calls `identity_reputation_contract.register_driver` for the owner when an identity contract is configured

#### `get_fleet`
Retrieve the stored profile for a fleet.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `FleetProfile`

**Errors:**
- `FleetNotFound` - No fleet with that ID exists

#### `deactivate_fleet`
Deactivate an active fleet. This is a terminal lifecycle step: new invitations are rejected and `get_payout_address` falls back to the driver's own address until a fleet is reactivated or replaced.

**Parameters:**
- `caller: Address` - Fleet owner or protocol admin
- `fleet_id: FleetId` - Fleet identifier

**Authorization:** Fleet owner or contract admin

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is neither the fleet owner nor the admin

**Events:** `fleet_deactivated`

#### `admin_reassign_fleet_owner`
Emergency recovery path for a compromised fleet-owner key. Only the protocol admin may call this.

**Parameters:**
- `admin: Address` - Contract admin
- `fleet_id: FleetId` - Fleet identifier
- `new_owner: Address` - Replacement fleet owner

**Authorization:** Protocol admin only

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is not the admin

**State Changes:**
- Updates `profile.owner` to `new_owner`
- Resets `profile.signers` to `[new_owner]` with `signature_threshold = 1`
- Bypasses the normal fleet-owner timelock flow and the owner's own authorization path

**Events:** `fleet_owner_reassigned`

#### `admin_force_update_treasury`
Emergency override that replaces the fleet treasury without waiting for the owner's timelock. This bypasses the normal owner-initiated treasury-change flow and clears any pending owner-side update so it cannot overwrite the emergency address later.

**Parameters:**
- `admin: Address` - Contract admin
- `fleet_id: FleetId` - Fleet identifier
- `new_treasury: Address` - Replacement treasury address

**Authorization:** Protocol admin only

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is not the admin

**Events:** `fleet_treasury_force_updated`

#### `update_fleet_treasury`
Propose a new treasury wallet for an existing fleet. This does not take effect immediately; the change becomes eligible only after `TREASURY_CHANGE_TIMELOCK_SECONDS` (3 days) have elapsed.

**Parameters:**
- `owner: Address` - Fleet owner (must sign)
- `fleet_id: FleetId` - Fleet identifier
- `treasury: Address` - Proposed new treasury wallet address

**Authorization:** Fleet owner

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is not the fleet owner

**Events:** `fleet_treasury_change_proposed`

#### `confirm_fleet_treasury_update`
Apply a previously proposed treasury change once its timelock has elapsed.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Errors:**
- `NoPendingTreasuryChange` - No treasury change has been proposed for this fleet
- `TimelockNotElapsed` - The proposal's timelock has not yet elapsed
- `FleetNotFound` - No fleet with that ID exists

**Events:** `fleet_treasury_updated`

#### `get_pending_treasury_update`
Return the pending treasury change for a fleet, if any.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `Option<PendingTreasuryChange>`

### Driver Management

#### `add_driver_to_fleet`
Invite a driver to join a fleet.

**Parameters:**
- `caller: Address` - Authorized signer or owner
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver to invite

**Authorization:** A signer authorized under the fleet's configured signer rule

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `FleetInactive` - The fleet has been deactivated
- `Unauthorized` - Caller is not an authorized signer
- `DriverAlreadyInvited` - A pending invite already exists for this driver
- `DriverAlreadyActive` - Driver is already an active member

**Events:** `driver_invited`

**State Changes:**
- Stores `DriverFleetStatus::Pending` for `(fleet_id, driver)`

#### `cancel_invite`
Cancel a driver's pending invite before it has been accepted.

**Parameters:**
- `owner: Address` - Authorized fleet signer
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver whose invite is being withdrawn

**Authorization:** An authorized signer for the fleet

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `InviteNotFound` - No pending invite exists for this driver
- `DriverAlreadyActive` - The driver is already active in the fleet

#### `accept_fleet_invite`
Accept a pending fleet invite.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver accepting the invite (must sign)

**Authorization:** Driver (must sign)

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `InviteNotFound` - No pending invite exists for this driver
- `DriverAlreadyActive` - Driver is already an active member

**Events:** `invite_accepted`

**State Changes:**
- Sets `DriverFleetStatus::Active` for `(fleet_id, driver)`
- Increments `FleetProfile.total_active_drivers`

#### `remove_driver_from_fleet`
Remove a driver from a fleet. Either a signer on the fleet or the driver themselves may initiate the removal.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `caller: Address` - Fleet signer or driver being removed (must sign)
- `driver: Address` - Driver to remove

**Authorization:** Fleet signer or the driver themselves

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is neither an authorized signer nor the driver
- `InviteNotFound` - No fleet record exists for this driver

**Events:** `driver_removed`

**State Changes:**
- Sets the driver's membership status to `Removed`
- Decrements `FleetProfile.total_active_drivers` if the driver was active
- Compacts the roster index so it remains contiguous

#### `get_driver_fleet_status`
Return the fleet membership status of a driver, or `None` if no record exists.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier
- `driver: Address` - Driver address

**Returns:** `Option<DriverFleetStatus>`

#### `get_fleet_roster`
Return the active roster of drivers for a fleet.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `Vec<Address>`

#### `configure_signers`
Configure the signer set and signature threshold for a fleet.

**Parameters:**
- `owner: Address` - Fleet owner
- `fleet_id: FleetId` - Fleet identifier
- `signers: Vec<Address>` - Authorized signer addresses
- `threshold: u32` - Minimum signatures required to perform signer-gated actions

**Authorization:** Fleet owner only

**Errors:**
- `FleetNotFound` - No fleet with that ID exists
- `Unauthorized` - Caller is not the fleet owner
- `InvalidConfiguration` - Threshold is zero or exceeds the signer set size

**Events:** `signers_configured`

#### `get_fleet_signers`
Return the configured signers and threshold for a fleet.

**Parameters:**
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `(Vec<Address>, u32)`

### Payout Routing

#### `get_payout_address`
Return the address the escrow contract should route funds to for a given driver and fleet.

**Parameters:**
- `driver: Address` - Driver address
- `fleet_id: FleetId` - Fleet identifier

**Returns:** `Address` — the fleet treasury if the driver is an active member; otherwise the driver's own address

---

## Identity Reputation Contract

Manages on-chain driver and user profiles, KYC status, and reputation scoring.

### Types

#### `UserProfile`
Defined once in `shared_types` and imported here (no local redeclaration).
```rust
pub struct UserProfile {
    pub address:       Address,
    pub registered_at: u64,
}
```

#### `DriverProfile`
Defined once in `shared_types` and imported here (no local redeclaration).
```rust
pub struct DriverProfile {
    pub address:               Address,
    pub deliveries_completed:  u32,
    pub reputation_score:      u32,   // 0–100
    pub registered_at:         u64,
    pub kyc_verified:          bool,
    pub status:                DriverStatus,
}
```

#### `DriverStatus`
```rust
pub enum DriverStatus {
    Active,     // Registered and eligible to participate
    Suspended,  // Administratively suspended; profile preserved for audit
}
```

#### `DriverTier`
```rust
pub enum DriverTier {
    Bronze,  // score < 50
    Silver,  // 50 ≤ score < 75
    Gold,    // score ≥ 75
}
```

### Initialization

#### `init`
Initialize the contract with only an admin address.

**Parameters:**
- `admin: Address` - Contract administrator

**Authorization:** Contract deployer

**Errors:**
- `AlreadyInitialized` - Contract has already been initialized

#### `initialize`
Initialize the contract with an admin and peer contract addresses (delivery & dispute).

**Parameters:**
- `admin: Address` - Contract administrator (must sign)
- `delivery_contract: Address` - Address of the delivery contract
- `dispute_contract: Address` - Address of the dispute resolution contract

**Authorization:** Admin (must sign)

**Errors:**
- `AlreadyInitialized` - Contract has already been initialized

### Admin Operations

#### `get_admin`
Return the current admin address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `set_reputation_config`
Set the scoring config for delivery completion-based reputation awards.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `config: ReputationConfig` - New reward configuration

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not the stored admin

**Defaults:**
- `base_points = 5`
- `heavy_cargo_points = 3`
- `fragile_points = 2`

#### `get_reputation_config`
Return the configured reputation reward structure.

**Returns:** `ReputationConfig`

#### `set_delivery_contract`
Set the address of the delivery contract that may call into reputation updates.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `delivery_contract: Address` - Address of the delivery contract

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not the stored admin

#### `set_dispute_contract`
Set the address of the dispute-resolution contract that may authorize reputation changes.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `dispute_contract: Address` - Address of the dispute-resolution contract

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not the stored admin

#### `get_delivery_contract`
Return the configured delivery contract address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `get_dispute_contract`
Return the configured dispute contract address.

**Returns:** `Address`

**Errors:**
- `NotInitialized` - Contract has not been initialized

#### `set_authorized_contract`
Grant or revoke cross-contract call authorization.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `contract_addr: Address` - Contract to authorize or deauthorize
- `authorized: bool` - `true` to grant, `false` to revoke

**Authorization:** Admin only

**Errors:**
- `Unauthorized` - Caller is not the stored admin

#### `is_authorized_contract`
Check whether a contract address is authorized to make cross-contract calls.

**Parameters:**
- `contract_addr: Address` - Contract address to check

**Returns:** `bool`

#### `update_driver_kyc_status`
Update a driver's KYC verification status.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `driver: Address` - Driver whose KYC status is being updated
- `kyc_verified: bool` - New KYC verification status

**Authorization:** Admin only

**Errors:**
- `NotInitialized` - Contract has not been initialized
- `Unauthorized` - Caller is not the stored admin
- `ProviderNotFound` - Driver profile does not exist

**Events:** `kyc_status_updated`

**Example:**
```rust
identity_contract.update_driver_kyc_status(&admin, &driver, &true);
```

#### `suspend_driver`
Suspend a registered driver. Sets `DriverProfile.status` to `DriverStatus::Suspended`.

The profile record is **never deleted** — all history (reputation score,
deliveries completed, KYC status) is preserved. This prevents a suspended
driver from calling `register_driver` again to obtain a clean slate, since
that function panics when a profile already exists.

> **Note:** Gating `assign_driver` on driver suspension status is a deliberate
> follow-up task in `delivery_contract` and is out of scope here.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `driver: Address` - Driver to suspend

**Authorization:** Admin only

**Errors:**
- `NotInitialized` - Contract has not been initialized
- `Unauthorized` - Caller is not the stored admin
- `ProviderNotFound` - Driver profile does not exist
- `InvalidState` - Driver is already suspended

**Events:** `driver_suspended`

**State Changes:**
- Sets `DriverProfile.status` to `DriverStatus::Suspended`
- All other profile fields remain unchanged

**Example:**
```rust
identity_contract.suspend_driver(&admin, &driver);
```

#### `reinstate_driver`
Reinstate a previously suspended driver. Sets `DriverProfile.status` back to
`DriverStatus::Active`. All accumulated reputation and delivery history is retained.

**Parameters:**
- `admin: Address` - Admin address (must sign)
- `driver: Address` - Driver to reinstate

**Authorization:** Admin only

**Errors:**
- `NotInitialized` - Contract has not been initialized
- `Unauthorized` - Caller is not the stored admin
- `ProviderNotFound` - Driver profile does not exist
- `InvalidState` - Driver is already active (not suspended)

**Events:** `driver_reinstated`

**State Changes:**
- Sets `DriverProfile.status` to `DriverStatus::Active`

**Example:**
```rust
identity_contract.reinstate_driver(&admin, &driver);
```

#### `is_driver_suspended`
Check whether a driver's profile is currently suspended.

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `bool` — `true` if the profile exists and has `DriverStatus::Suspended`, `false` otherwise

**Example:**
```rust
let suspended: bool = identity_contract.is_driver_suspended(&driver);
```

### Profile Management

#### `register_driver`
Register a new driver profile with a starting reputation score of 50.

**Parameters:**
- `driver: Address` - Driver address (must sign)

**Authorization:** Driver (must sign)

**Errors:**
- `AlreadyInitialized` - A profile already exists for this address

**Events:** `driver_registered`

**State Changes:**
- Creates `DriverProfile` with `reputation_score = 50`, `deliveries_completed = 0`, `kyc_verified = false`

**Example:**
```rust
identity_contract.register_driver(&driver);
```

#### `register_user`
Register a new user (sender/recipient) profile.

**Parameters:**
- `user: Address` - User address (must sign)

**Authorization:** User (must sign)

**Returns:** `UserProfile`

**Errors:**
- `AlreadyInitialized` - A profile already exists for this address

**Events:** `user_registered`

#### `get_driver_profile`
Retrieve a driver's profile.

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `DriverProfile`

**Errors:**
- `ProviderNotFound` - Driver profile does not exist

#### `get_user_profile`
Retrieve a user's profile.

**Parameters:**
- `user: Address` - User address

**Returns:** `UserProfile`

**Errors:**
- `ProviderNotFound` - User profile does not exist

#### `has_user_profile`
Check whether a user profile exists.

**Parameters:**
- `user: Address` - User address

**Returns:** `bool`

#### `has_driver_profile`
Check whether a driver profile already exists.

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `bool`

### Reputation Management

#### `increase_reputation`
Increase a driver's reputation score after a successful delivery.

**Parameters:**
- `caller: Address` - Must be the delivery contract or dispute contract
- `driver: Address` - Driver whose score is being updated
- `delivery_id: u64` - Delivery identifier (for event emission)
- `weight_grams: u32` - Cargo weight in grams (>5 000 g adds bonus points)
- `fragile: bool` - Whether the cargo was fragile (adds bonus points)

**Authorization:** Delivery contract or dispute contract only

**Errors:**
- `NotInitialized` - Contract addresses not configured
- `Unauthorized` - Caller is not an authorized contract
- `ProviderNotFound` - Driver profile does not exist

**Events:** `reputation_increased`

**State Changes:**
- Adds base 5 points + 3 for heavy cargo (>5 000 g) + 2 for fragile cargo
- Caps `reputation_score` at 100
- Increments `deliveries_completed`

**Example:**
```rust
identity_contract.increase_reputation(
    &delivery_contract,
    &driver,
    1u64,      // delivery_id
    6000u32,   // weight_grams (adds bonus)
    true       // fragile (adds bonus)
);
```

#### `decrease_reputation`
Decrease a driver's reputation score following a dispute resolved in the sender's favour.

**Parameters:**
- `caller: Address` - Must be the delivery contract or dispute contract
- `driver: Address` - Driver whose score is being penalised
- `points: u32` - Number of reputation points to deduct

**Authorization:** Delivery contract or dispute contract only

**Errors:**
- `NotInitialized` - Contract addresses not configured
- `Unauthorized` - Caller is not an authorized contract
- `ProviderNotFound` - Driver profile does not exist

**Events:** `reputation_decreased`

**State Changes:**
- Decreases `reputation_score` by `points`, flooring at 0 (saturating subtraction)

**Example:**
```rust
identity_contract.decrease_reputation(
    &dispute_contract,
    &driver,
    10u32  // deduct 10 points
);
```

#### `award_reputation`
Add a flat reputation credit to a driver — used when a dispute is resolved in the
driver's favour. Unlike `increase_reputation`, this does **not** derive points
from cargo attributes and does **not** increment `deliveries_completed` (a dispute
ruling is not a delivery completion, and counting it as one would double-count if
the delivery is later confirmed).

**Parameters:**
- `caller: Address` - Must be the delivery contract or dispute contract
- `driver: Address` - Driver whose score is being credited
- `points: u32` - Number of reputation points to add

**Authorization:** Delivery contract or dispute contract only

**Errors:**
- `Unauthorized` - Caller is not an authorized contract
- `ProviderNotFound` - Driver profile does not exist

**Events:** `reputation_awarded`

**State Changes:**
- Increases `reputation_score` by `points`, capped at 100
- Leaves `deliveries_completed` unchanged

**Example:**
```rust
identity_contract.award_reputation(
    &dispute_contract,
    &driver,
    5u32  // flat dispute reward
);
```

### Tier & Eligibility

#### `get_driver_tier`
Return the driver's current tier based on their reputation score.

| Score range | Tier   |
|-------------|--------|
| 0 – 49      | Bronze |
| 50 – 74     | Silver |
| 75 – 100    | Gold   |

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `DriverTier`

**Errors:**
- `ProviderNotFound` - Driver profile does not exist

#### `is_eligible_for_enterprise`
Check whether a driver's reputation score meets the enterprise threshold (≥ 75).

**Parameters:**
- `driver: Address` - Driver address

**Returns:** `bool`

**Errors:**
- `ProviderNotFound` - Driver profile does not exist

---

## Settlement Contract

> **Phase 3 — Stub implementation.** The Settlement contract is deployed but its
> functions are not yet implemented. Function signatures and intended behaviour
> are documented here for integrator reference; the bodies will be filled in
> during Phase 3 development.

Handles cross-border currency swaps during escrow release, allowing drivers to
receive payment in their preferred asset via the Stellar DEX or liquidity pools.

### Initialization

#### `init`
Initialize the settlement contract.

**Parameters:**
- `admin: Address` - Contract administrator (must sign)

**Authorization:** Admin (must sign)

> **Note:** Phase 3 stub — no state is persisted yet.

### Query Functions

#### `get_driver_preference`
Return the driver's preferred asset for payment, if one has been set.

**Parameters:**
- `_driver: Address` - Driver address

**Returns:** `Option<Address>`

> **Note:** Phase 3 stub — always returns `None`.

### Settlement Operations

#### `execute_settlement_swap`
Execute an asset swap and transfer the output to a recipient.

Intended to integrate with the Stellar DEX or a liquidity pool to convert
`from_token` to `to_token` before crediting `recipient` with at least
`min_amount_out`.

**Parameters:**
- `caller: Address` - Authorized caller (must sign)
- `_from_token: Address` - Source token contract address
- `_to_token: Address` - Target token contract address
- `_recipient: Address` - Address receiving the swapped funds
- `_amount: i128` - Amount of `from_token` to swap
- `_min_amount_out: i128` - Minimum acceptable output (slippage guard)

**Authorization:** Caller (must sign)

> **Note:** Phase 3 stub — no swap is performed.
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
    pub status: DriverStatus,       // Active or Suspended
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
    DuplicateDelivery = 8,      // Delivery ID exists
    ProviderNotFound = 9,       // Driver not found
    ProtocolPaused = 11,        // Protocol paused, fund movements halted
    LimitExceeded = 12,         // A bounded collection is already at its max length
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

### Error Codes by Contract

A raw Soroban error (`Error(Contract, #N)`) only carries a numeric code — the meaning of that
code depends on which contract raised it. Each contract defines its own `#[contracterror]`
enum starting from `1`, so the same number means different things in different contracts.

The full, canonical table of every error variant in the workspace — labeled by originating
contract, so an integrator handling errors from a multi-contract call chain can look up any
`(contract, code)` pair in one place — lives in **[`docs/ERROR_CODES.md`](./ERROR_CODES.md)**.
That file is the single source of truth; update it (not this section) when error variants
change.

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
**Soroban SDK**: 27.0.0

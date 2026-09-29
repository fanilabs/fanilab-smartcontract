# FaniLab Smart Contract Architecture

The FaniLab decentralized logistics economy is powered by a modular, secure, and upgradeable multi-contract architecture built on the Stellar Soroban network.

To ensure separation of concerns, the system is broken down into **6 deployable core smart contracts plus 1 shared (non-deployed) library**.

## 1. `shared_types` (Library)
Houses all shared Enums, Structs, and Data representations across the entire platform.
- `DeliveryStatus` (Pending, Active, InTransit, Delivered, Disputed, Cancelled)
- `EscrowState` (Locked, Holdback, Released, Refunded, Paused, Split)
- `CargoCategory` (Documents, Electronics, Perishables, Clothing, General)
- `FaniLabError` enum with authorization and validation errors
- Core structs: `DeliveryRecord`, `EscrowRecord`, `CargoDescriptor`, `DeliveryMetadata`, `DriverProfile`, `UserProfile`, `ProtocolConfig`
- Event structs: `DeliveryCreatedEvent`, `EscrowFundedEvent`, `DriverAssignedEvent`, `DeliveryConfirmedEvent`, `EscrowReleasedEvent`, `DeliveryDisputedEvent`, `EscrowRefundedEvent`, `DisputeResolvedEvent`

## 2. `delivery_contract`
Manages the lifecycle of a logistics package.
- **Responsibilities**: Creation of delivery, Assignment of drivers, In-Transit updates, and Delivery Confirmation.
  > **Note (Issue #306):** Proof of Delivery (PoD) hashing is **not implemented**. `confirm_delivery` takes only the recipient address and delivery ID; no proof artifact is stored. The closest existing mechanism is `dispute_resolution_contract::add_evidence_hash`, which is dispute-scoped. PoD hashing as a driver-recourse primitive is a planned feature — see the backlog for a dedicated feature issue.
- **Interacts with**: `identity_reputation_contract` (`register_user` on delivery creation, `increase_reputation` on confirmation), `escrow_contract` (calls `get_escrow` to verify funding before marking in-transit, `refund_escrow` on cancellation, `mark_holdback_escrow` on confirmation, `raise_dispute` on dispute, `reclaim_expired_escrow` via the `reclaim_delivery_expired_escrow` entry point).
  > **Note (Issue #307):** The delivery contract does **not** call `get_driver_tier` or perform any tier/KYC-based vetting at assignment. `assign_driver` verifies only that the caller is admin or the driver themselves, and that the driver is not the sender or recipient. Tier-gating at assignment is tracked as a separate backlog item (see closed issue #44).

## 3. `escrow_contract`
Strictly manages the financial security of the platform.
- **Responsibilities**: Locking, releasing, and refunding Stellar assets (XLM, USDC, etc.).
- **Interacts with**: `delivery_contract` (to verify status), `dispute_resolution_contract` (for freezes/slashing), `settlement_contract` (for cross-border FX routing).

## 4. `identity_reputation_contract`
Manages Driver, Fleet, and User profiles.
- **Responsibilities**: Tracking reputation scores (SLA, successful deliveries, dispute frequency) which affect access to high-paying enterprise jobs.
- **Interacts with**: `delivery_contract` (to enforce tier-based job acceptance).

## 5. `fleet_management_contract`
Empowers Enterprise Logistics SMEs.
- **Responsibilities**: Allows enterprises to register fleets, add drivers, and route escrow payouts directly to the fleet owner's treasury wallet instead of the individual driver.
- **Interacts with**: `escrow_contract` (to redirect the payout destination).

## 6. `dispute_resolution_contract`
Handles edge cases like damaged goods or stolen packages.
- **Responsibilities**: Freezes the `escrow_contract` for a specific delivery if an issue is raised. Allows an Admin/Oracle to slash funds, force-refund, or split funds.
- **Interacts with**: `escrow_contract` (to freeze/unfreeze funds), `identity_reputation_contract` (to penalize drivers who lose disputes).

## 7. `settlement_contract`
Handles cross-border trade logic.
- **Responsibilities**: Interacts with Soroban AMMs to swap assets upon escrow release. E.g., Sender locks Nigerian Naira (NGNC) stablecoin, but the driver prefers USDC.
- **Interacts with**: `escrow_contract` (intercepts the payout and converts it via DEX before sending to the driver).

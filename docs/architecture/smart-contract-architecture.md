# FaniLab Smart Contract Architecture

The FaniLab decentralized logistics economy is powered by a modular, secure, and upgradeable multi-contract architecture built on the Stellar Soroban network.

To ensure separation of concerns, the system is broken down into **7 core smart contracts (and libraries)**.

## 1. `shared_types` (Library)
Houses all shared Enums, Structs, and Data representations across the entire platform.
- `DeliveryStatus` (Pending, PickedUp, InTransit, Delivered, Disputed)
- `RoleType` (Sender, Receiver, Driver, FleetOwner, Admin)
- `DisputeStatus` (Open, ResolvedRefund, ResolvedPayout, Split)

## 2. `delivery_contract`
Manages the lifecycle of a logistics package.
- **Responsibilities**: Creation of delivery, Assignment of drivers, In-Transit updates, and Proof of Delivery (PoD) hashing.
- **Interacts with**: `identity_reputation_contract` (to verify driver tier), `escrow_contract` (to trigger payment upon completion).

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

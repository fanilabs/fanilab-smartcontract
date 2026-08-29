# Error Code Reference

Canonical, single-source-of-truth reference for every `#[contracterror]` enum in the
FaniLab workspace. `docs/API.md`'s "Error Codes by Contract" section links here rather
than maintaining a second copy — update this file when an error variant is added,
removed, or renumbered, and both docs stay accurate.

## Why this file exists

A raw Soroban error only carries a numeric code: `Error(Contract, #N)`. That number's
meaning depends entirely on *which contract* raised it — each contract defines its own
`#[contracterror]` enum starting from `1`, so the same code means different things in
different contracts. An integrator handling errors from a multi-contract call chain
(e.g. `delivery_contract` invoking `escrow_contract` invoking `identity_reputation_contract`)
needs to know which contract's enum to look the code up against. This file lists every
enum in the workspace so any `(contract, code)` pair can be resolved in one place.

Two contracts — `dispute_resolution_contract` and `identity_reputation_contract` —
define no local error enum and raise only the shared `FaniLabError` below.

## `FaniLabError` — shared across all six contracts (`shared_types`)

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `Unauthorized` | Caller is not authorized to perform the requested action. |
| 2 | `AlreadyInitialized` | Contract or protocol state has already been initialized. |
| 3 | `NotInitialized` | Contract or protocol state has not been initialized yet. |
| 4 | `DeliveryNotFound` | Delivery record or related escrow entry could not be found. |
| 5 | `InvalidState` | Requested operation is invalid for the current protocol state. |
| 6 | `InsufficientFunds` | Contract balance is too low to complete the requested transfer. |
| 8 | `DuplicateDelivery` | Delivery identifier already exists in protocol storage. |
| 9 | `ProviderNotFound` | Provider or driver record could not be found. |
| 11 | `ProtocolPaused` | Protocol is paused and fund movements are halted. |
| 12 | `LimitExceeded` | Requested operation would exceed a fixed capacity/growth limit (e.g. `dispute_resolution_contract::add_evidence_hash` once the calling party has already submitted its per-party maximum of 20 evidence hashes for that dispute — the cap is enforced per submitting party, so at most `3 × 20` hashes accumulate across the three delivery parties). |

Note: codes `7` and `10` are intentionally unused gaps left by prior variant removals —
they are not reserved for anything and should not be inferred to mean "no error."

## `EscrowError` — `escrow_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `InvalidState` | Requested operation is invalid for the escrow's current state. |
| 2 | `DeliveryNotFound` | No escrow exists for the given delivery ID. |
| 3 | `InsufficientFunds` | Escrow balance is too low to complete the requested transfer. |
| 4 | `DuplicateDelivery` | An escrow already exists for the given delivery ID. |
| 5 | `InvalidFee` | Requested platform fee exceeds the configured maximum. |
| 6 | `InvalidToken` | Token does not match the protocol-configured token. |
| 7 | `InvalidAmount` | Escrow amount is not positive. |
| 8 | `NoPendingSettlementChange` | `confirm_settlement_contract` called with no proposal pending. |
| 9 | `TimelockNotElapsed` | `confirm_settlement_contract` called before the proposal's timelock elapsed. |
| 10 | `BatchTooLarge` | `create_escrows_batch` list exceeds `MAX_BATCH_SIZE` (100). |

`escrow_contract` also raises `FaniLabError` directly for conditions that predate this
crate's own enum (e.g. `Unauthorized`, `NotInitialized`, `ProtocolPaused`) — check both
tables when debugging an error from this contract.

## `DeliveryError` — `delivery_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `InvalidState` | Requested delivery status transition is not permitted. |
| 2 | `InvalidMetadata` | Delivery metadata fails validation (e.g. location or weight limits). |
| 3 | `BatchTooLarge` | `create_deliveries_batch` list exceeds `MAX_BATCH_SIZE` (100). |
| 4 | `InvalidDriver` | Driver address matches the delivery's sender or recipient. |
| 5 | `InvalidParties` | Sender and recipient are the same address. |

`delivery_contract` also raises `FaniLabError` directly (e.g. `Unauthorized`,
`DeliveryNotFound`, `NotInitialized`).

## `FleetError` — `fleet_management_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `AlreadyInitialized` | Contract has already been initialized. |
| 2 | `NotInitialized` | Contract has not been initialized yet. |
| 3 | `Unauthorized` | Caller is not authorized to perform the requested action. |
| 4 | `FleetNotFound` | No fleet exists for the given fleet ID. |
| 5 | `DriverAlreadyInvited` | Driver already has a pending invite to this fleet. |
| 6 | `InviteNotFound` | No pending invite exists for this driver/fleet pair. |
| 7 | `DriverAlreadyActive` | Driver is already an active member of this fleet. |
| 8 | `NoPendingTreasuryChange` | `confirm_fleet_treasury_update` called with no proposal pending. |
| 9 | `TimelockNotElapsed` | `confirm_fleet_treasury_update` called before the proposal's timelock elapsed. |
| 10 | `FleetInactive` | Requested operation is invalid because the fleet has been deactivated. |
| 11 | `InvalidConfiguration` | Signer configuration contains an invalid threshold. |

## `SettlementError` — `settlement_contract`

| Code | Variant | Meaning |
|------|---------|---------|
| 1 | `SwapNotImplemented` | Settlement swaps are unavailable until the Phase 3 implementation is complete. |

## Contracts with no local error enum

`dispute_resolution_contract` and `identity_reputation_contract` raise `FaniLabError`
exclusively — look up any error code they return in the shared table above.

## Interpreting a raw error

Given `Error(Contract, #N)`, resolve it by:

1. Identify which contract's transaction/call actually raised it (the failing invocation
   in the call chain, not necessarily the top-level entry point you called).
2. If that contract has its own table above, check there first — most local enums
   overlap numerically with `FaniLabError` (e.g. code `1` is `InvalidState` in
   `EscrowError` but `Unauthorized` in `FaniLabError`), so the two must not be
   conflated.
3. If no local variant matches, fall back to the shared `FaniLabError` table.

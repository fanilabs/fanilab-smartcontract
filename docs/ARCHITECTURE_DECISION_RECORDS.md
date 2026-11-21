# Architecture Decision Records (ADR)

## Overview
This document records significant architecture decisions for FaniLab Smart Contracts.

## ADR-001: Soroban on Stellar

**Date**: 2024-11-01  
**Status**: Accepted

### Context
Needed to choose blockchain platform for logistics escrow system.

### Decision
Use Stellar Soroban smart contract platform.

### Rationale
- Low transaction costs (~$0.00001 per transaction)
- Fast finality (5-7 seconds)
- Native asset support
- Strong ecosystem for payments
- Rust-based contracts (memory safety)
- WASM compilation for efficiency

### Consequences
**Positive**:
- Affordable for high-volume logistics
- Quick transaction confirmation
- Strong developer tools

**Negative**:
- Smaller ecosystem than Ethereum
- Newer platform (less battle-tested)
- Limited composability initially

---

## ADR-002: Multi-Contract Architecture

**Date**: 2024-11-15  
**Status**: Accepted

### Context
System could be single monolithic contract or multiple specialized contracts.

### Decision
Use 6 specialized contracts with a shared types library.

### Rationale
- Separation of concerns
- Independent upgrades
- Gas optimization per function
- Clear security boundaries
- Easier auditing

### Consequences
**Positive**:
- Smaller contract sizes
- Modular upgrades
- Clear responsibilities

**Negative**:
- Cross-contract call overhead
- More deployment complexity
- Coordination needed for upgrades

---

## ADR-003: Shared Types Library

**Date**: 2024-11-20  
**Status**: Accepted

### Context
Contracts need to share data structures and error types.

### Decision
Create `shared_types` library with all common types.

### Rationale
- Single source of truth
- Consistent error handling
- Type safety across contracts
- Easier maintenance

### Consequences
**Positive**:
- No type mismatches
- Clear cross-contract interface
- Centralized error definitions

**Negative**:
- All contracts must use same shared_types version
- Breaking changes affect all contracts

---

## ADR-004: State Transition Validation

**Date**: 2024-12-01  
**Status**: Accepted

### Context
Delivery status transitions must be controlled and validated.

### Decision
Implement explicit state machine with transition validation.

### Rationale
- Prevents invalid state changes
- Clear business logic
- Easy to audit
- Prevents edge cases

### Consequences
**Positive**:
- No invalid states possible
- Clear transition rules
- Better error messages

**Negative**:
- Slightly more gas usage
- Must update if business logic changes

---

## ADR-005: Two-Step Admin Transfer

**Date**: 2024-12-05  
**Status**: Accepted

### Context
Need secure way to transfer admin role.

### Decision
Implement propose/accept pattern for admin transfer.

### Rationale
- Prevents accidental transfers
- New admin must prove access
- Safer than single-step

### Consequences
**Positive**:
- No admin lockouts
- Explicit acceptance required
- Time to verify new admin

**Negative**:
- Two transactions instead of one
- Slightly more complex

---

## ADR-006: Saturating Math for Fees

**Date**: 2024-12-10  
**Status**: Accepted

### Context
Fee calculations could overflow with extreme values.

### Decision
Use saturating arithmetic for all fee calculations.

### Rationale
- Prevents integer overflow
- Graceful handling of edge cases
- Soroban SDK supports it natively

### Consequences
**Positive**:
- No overflow vulnerabilities
- Predictable behavior

**Negative**:
- Slightly less efficient than unchecked math
- Could hide logic errors (though unlikely)

---

## ADR-007: Event-Driven Architecture

**Date**: 2024-12-15  
**Status**: Accepted

### Context
Off-chain systems need to track contract state changes.

### Decision
Emit comprehensive events for all state changes.

### Rationale
- Enables off-chain indexing
- Auditability
- User notifications
- Analytics

### Consequences
**Positive**:
- Easy off-chain monitoring
- Complete audit trail
- Real-time notifications possible

**Negative**:
- Small gas cost for events
- Must maintain event compatibility

---

## ADR-008: TTL Management Strategy

**Date**: 2025-01-05  
**Status**: Accepted

### Context
Soroban requires explicit TTL management for storage.

### Decision
- Persistent storage with 30-day default TTL
- Auto-extend on access
- Threshold-based extension

### Rationale
- Balance storage costs and availability
- Prevent data expiration during active deliveries
- Reasonable archival policy

### Consequences
**Positive**:
- Active data stays available
- Storage costs controlled
- Old deliveries naturally archive

**Negative**:
- Must remember to extend on reads
- Archived data requires restoration

---

## ADR-009: Settlement Contract Integration

**Date**: 2025-01-10  
**Status**: Accepted

### Context
Drivers may prefer different currencies than senders.

### Decision
Add optional settlement contract for currency swaps.

### Rationale
- Cross-border flexibility
- Driver preference support
- DeFi integration
- Increased adoption

### Consequences
**Positive**:
- Multi-currency support
- Better driver experience
- More use cases

**Negative**:
- Additional complexity
- Slippage risk
- Requires liquidity

---

## ADR-010: Delivery-Escrow State Machine Coupling

**Date**: 2025-01-15  
**Status**: Accepted

### Context
Delivery completion and fund release involve two separate contracts (delivery_contract and escrow_contract) with independent state machines. Without enforced synchronization invariants, these states can silently desynchronize when cross-contract calls fail or are executed in unexpected orders, leading to locked or incorrectly released funds with no audit trail.

### Decision
Implement explicit state-machine invariants that ensure delivery and escrow states remain synchronized:
- **Pending/Active** delivery → Locked escrow
- **InTransit** delivery → Locked escrow  
- **Delivered** delivery → Released escrow
- **Disputed** delivery → Paused escrow
- **Cancelled** delivery → Refunded escrow

Provide a read-only `get_combined_state(delivery_id)` view that fetches both records and validates synchronization, enabling off-chain indexers and auditors to detect desynchronization.

### Rationale
- **Prevents silent failures**: Cross-contract call failures now panic rather than silently leaving escrow unmodified
- **Auditability**: Combined state view enables off-chain detection of mismatched states
- **Explicit invariants**: Clear documentation of expected state relationships
- **Easier debugging**: Audit trail is explicit rather than requiring manual cross-referencing

### Consequences
**Positive**:
- Impossible to reach an invalid state combination
- Off-chain indexers can detect desynchronization instantly
- Clear failure modes for contract coordination issues
- Better protocol reliability

**Negative**:
- Slightly increased gas cost for combined state reads
- Must carefully order cross-contract calls to avoid reversions
- Requires coordination during emergency state corrections

### Alternatives Considered
- Single unified contract: Would increase contract size and reduce modularity
- Event-based state sync: Relies on off-chain consistency which is not guaranteed
- Optional validation: Allows silent desynchronization if not called

---

## ADR-011: Shared Governance Abstraction

**Date**: 2026-01-15  
**Status**: Revised (see Addendum below) — the `AdminManager` module described
in the original decision was built but never adopted by any contract, and
was removed as dead code during the FC-1 cleanup batch (see
`docs/CODEBASE_FINAL_CLEANUP_REPORT.md`). The original Context/Decision/
Rationale sections are kept for history; see the Addendum for what was
actually implemented instead.

### Context
Admin/governance models are implemented inconsistently across the six contracts: escrow_contract, delivery_contract, fleet_management_contract, and identity_reputation_contract each use a single Admin: Address pattern (with rotation support only in escrow_contract), while dispute_resolution_contract uses a multi-admin model with per-address boolean storage. This leads to different security properties and inconsistent APIs across the protocol.

### Decision
Introduce a shared `governance` module in `shared_types` that provides `AdminManager` utility functions for both single-admin and multi-admin patterns, enabling contracts to adopt consistent governance practices.

### Rationale
- **Consistency**: All contracts can share the same governance primitives with well-understood security properties
- **Auditability**: Centralized governance logic is easier to audit and review
- **Maintainability**: Bug fixes to governance logic benefit all contracts
- **Flexibility**: Module supports both single-admin and multi-admin patterns
- **Incremental migration**: Contracts can adopt the shared abstraction independently

### Consequences
**Positive**:
- Single source of truth for governance logic
- Consistent admin APIs across protocol
- Easier to implement complex governance (e.g., rotation, thresholds)
- Better auditability
- Shared tests and documentation

**Negative**:
- Requires updating existing contracts (can be done incrementally)
- May require version bump if breaking changes introduced
- Adds dependency on shared_types governance module

### Alternatives Considered
- Monolithic single-contract approach: Reduces modularity and gas optimization
- No shared abstraction: Inconsistency continues, harder to audit
- Immutable governance patterns: Loses flexibility for future requirements

### Future Work
- Migrate all six contracts to use the shared AdminManager
- Add threshold-based multi-sig support if needed
- Document governance security model in GOVERNANCE.md

### Addendum (FC-1 re-evaluation)
`shared_types::governance::AdminManager` was implemented per this ADR but
never wired into any contract — confirmed via a repo-wide grep finding zero
references outside its own definition. Re-evaluating it against the actual
governance needs across the six contracts found two separate problems it
didn't solve well:

1. **The single-admin case already had a better answer.** A simpler
   `shared_types::is_admin(env, caller) -> bool` function (no `AdminDataKey`
   indirection) already existed and was already adopted by `escrow_contract`
   and `delivery_contract`. `AdminManager::is_single_admin` would have been
   a strictly more complex way to do the same thing. Instead of wiring in
   `AdminManager`, `fleet_management_contract` and
   `identity_reputation_contract` were migrated onto `is_admin` directly,
   deleting their local, duplicate single-admin helpers
   (`is_fleet_admin` and an inline `DataKey::Admin` comparison,
   respectively). Four of six contracts now share one single-admin
   implementation.
2. **The multi-admin case was a downgrade, not an improvement.**
   `dispute_resolution_contract`'s existing multi-admin implementation
   (`DataKey::Admin(Address) -> bool` flags for O(1) membership checks,
   plus a separately maintained `DataKey::AdminList` for enumeration, with
   a last-admin-removal guard added in a prior hardening batch) is more
   efficient than `AdminManager::is_multi_admin`, which stores admins as a
   single `Vec<Address>` and does an O(n) linear scan per check. Migrating
   dispute_resolution_contract onto `AdminManager` would have meant
   rewriting a working, tested, already-hardened implementation to be
   *slower*, for no consistency benefit beyond sharing a struct name.

Multi-admin support is a genuinely different governance model from
single-admin, not a duplicate implementation of the same thing — merging
them into one abstraction would have meant either losing multi-admin
support entirely or forcing every single-admin contract through
multi-admin's less efficient storage shape. `dispute_resolution_contract`
keeping its own implementation is treated as an intentional architectural
boundary, not unaddressed duplication (see Issue #77's disposition in
`docs/CODEBASE_FINAL_CLEANUP_REPORT.md`).

`AdminManager` and its `AdminDataKey` enum were deleted from
`shared_types` accordingly.

---

---

## ADR-009: Residual Allowance Revocation in Settlement Operations

**Date**: 2026-02-15  
**Status**: Accepted

### Context
When executing settlements through router contracts or cross-contract token transfers, allowances granted to intermediary routers can leave residual spend authority if transfer amounts are adjusted or execution is completed below approved ceilings.

### Decision
Implement explicit residual allowance clearance or zero-allowance resets upon final settlement disposition:
1. Contracts executing routed settlements must ensure any excess allowance granted to transfer delegates is zeroed upon completion.
2. In client implementations and automated settlement orchestrators, verify allowance states and reset lingering allowances post-transaction.

### Rationale
- Prevents dormant allowances from being exploited in subsequent contract compromises.
- Enforces strict principle of least privilege for contract spend authorizations.

### Consequences
**Positive**:
- Eliminates attack surface from dangling token spend allowances.
- Guarantees predictable accounting across multi-hop settlement transactions.

**Negative**:
- May require an additional authorization call if exact settlement amounts cannot be predetermined.

## Template for New ADRs

```markdown
## ADR-XXX: Title

**Date**: YYYY-MM-DD  
**Status**: Proposed | Accepted | Deprecated | Superseded

### Context
Why is this decision needed?

### Decision
What was decided?

### Rationale
Why this decision?

### Consequences
**Positive**:
- 

**Negative**:
- 

### Alternatives Considered
- Option 1: Why rejected
- Option 2: Why rejected
```

---

**Last Updated**: January 2026

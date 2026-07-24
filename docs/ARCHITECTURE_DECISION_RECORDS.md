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
Use 7 specialized contracts with shared types library.

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

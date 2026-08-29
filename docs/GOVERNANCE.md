# Governance Model

## Overview

FaniLab Smart Contracts implement a secure, transparent governance model designed for production operations on Stellar.

## Admin Roles

### Contract Admin
- **Responsibilities**: 
  - Update platform fees
  - Resolve disputes
  - Manage contract configurations
  - Emergency operations

- **Security**:
  - Two-step admin transfer process
  - All actions require explicit authorization
  - All admin actions emit events

### Multi-Signature Requirements

For production deployments, we recommend:
- **Testnet**: Single admin (for testing flexibility)
- **Mainnet**: Multi-signature wallet (minimum 2-of-3)

## Governance Operations

### Fee Updates

Platform fees can be adjusted by admin within constraints:
- **Maximum Fee**: 10% (1000 basis points)
- **Recommended Range**: 1-5%
- **Current Default**: 2.5% (250 basis points)

**Process**:
1. Admin proposes fee change
2. Community notified (off-chain)
3. Grace period (48 hours recommended)
4. Fee update executed
5. Event emitted: `fee_updated`

### Admin Transfer

Secure two-step process:
1. Current admin proposes new admin
2. New admin accepts role
3. Transfer completes

This prevents accidental transfers and ensures new admin has access.

### Dispute Resolution Admin Roster

`dispute_resolution_contract` is governed by a **multi-admin roster** rather
than a single admin. Any admin may add or remove admins, subject to two
structural guards:

1. **Last-admin protection.** A removal that would empty the roster is
   rejected (`InvalidState`). Governance can never be bricked.
2. **No self-service consolidation (Issue #212).** A removal that would leave
   the roster with exactly one admin is permitted **only when the caller is
   removing themselves** — a deliberate step-down. An admin may never remove a
   *different* admin in a way that leaves the caller as the sole remaining
   admin. Ordinary removals still work while at least one other admin remains,
   and a full hand-off is done by adding the successor first and then stepping
   down. This stops a single (possibly compromised) admin key from removing
   every other admin one call at a time until it is the sole arbiter of every
   dispute.

**Transparency.** `add_admin` and `remove_admin` each emit an event
(`admin_added` / `admin_removed`) whose payload is `(caller, affected_admin)`,
so the remaining admins and any off-chain monitoring can detect roster changes
immediately. `list_admins` always reflects the current roster.

### Dispute Resolution

Admins can resolve disputes in three ways:
1. **Release to Driver**: Full payment to driver
2. **Refund to Sender**: Full refund to sender
3. **Split**: Configurable split between parties

**Process**:
1. Dispute raised by sender or recipient
2. Escrow paused automatically
3. Admin reviews evidence (off-chain)
4. Admin invokes resolution
5. Funds distributed per decision

## Emergency Procedures

### Contract Pause (Implemented in `escrow_contract`)
The protocol currently exposes an emergency pause switch in `escrow_contract` via `set_paused(admin, paused)` and `is_paused()`. This is a contract-level circuit breaker, not a full protocol-wide stop: the other five contracts have no pause concept and keep operating normally.

**Current behavior:**
1. `set_paused` is admin-only and flips the instance-level pause flag.
2. Every fund-moving / escrow-state-changing path calls `require_not_paused()` before proceeding.
3. `is_paused()` remains readable while the contract is halted.

**Functions gated while paused:** `create_escrow`, `create_escrows_batch`, `mark_holdback_escrow`, `release_escrow`, `refund_escrow`, `resolve_dispute`, `resolve_dispute_split`, `release_holdback_escrow`, and `reclaim_expired_escrow` all revert with `ProtocolPaused` if the flag is set.

**Deliberate exemptions:**
- `freeze_funds` is intentionally not blocked by `require_not_paused()`. It only transitions an escrow from `Locked` or `Holdback` into the disputed `Paused` state and is used to preserve the ability to freeze suspicious funds during an incident.
- `raise_dispute` is also left available while paused so a dispute can still be raised against an escrow under investigation.
- `sweep_untracked_balance` remains admin-only and is not treated as a normal fund-movement path for the pause gate.

**Roadmap note:** A protocol-wide pause across all six contracts remains a future design goal and should be tracked separately from the current `escrow_contract`-only emergency breaker.

### Fund Recovery
If funds stuck due to edge case:
- Admin can manually trigger releases
- Requires thorough off-chain verification
- All operations logged on-chain

## Decentralization Roadmap

### Phase 1 (Current)
- Single admin or multi-sig wallet
- Centralized dispute resolution
- Manual governance

### Phase 2 (6-12 months)
- DAO governance structure
- Token-based voting
- Automated fee adjustments based on metrics

### Phase 3 (12-24 months)
- Fully decentralized governance
- Community-driven upgrades
- Automated dispute resolution via oracles
- Reputation-based voting weight

## Transparency

### On-Chain Events
All governance actions emit events:
- `protocol_initialized`
- `fee_updated`
- `admin_transferred`
- `admin_added` / `admin_removed` (dispute-resolution roster changes)
- `dispute_resolved`

> Topic names are the strings emitted on-chain; payload structs like `ProtocolInitialized` and `FeeUpdated` are Rust type names, not the event topics subscribers should filter on.

### Off-Chain Communication
- Major changes announced on Discord/Twitter
- Governance proposals published on forum
- Monthly transparency reports

## Community Participation

### Proposal Process (Future)
1. **Draft**: Community member creates proposal
2. **Discussion**: 7-day discussion period
3. **Voting**: Token holders vote
4. **Implementation**: If passed, admin executes
5. **Verification**: Community verifies execution

### Feedback Channels
- GitHub Issues for technical proposals
- Discord for community discussion
- Governance forum for formal proposals
- Twitter for announcements

## Security Considerations

### Admin Key Security
- **Testnet**: Standard keypair acceptable
- **Mainnet**: 
  - Hardware wallet (Ledger/Trezor)
  - Multi-signature wallet
  - Key ceremony for initial setup
  - Regular key rotation

### Access Control
- Admin functions explicitly restricted
- No backdoors or hidden privileges
- All privileged operations auditable
- Time-locks on sensitive changes (future)

## Accountability

### Action Logs
All admin actions recorded:
- On-chain via events
- Off-chain in governance log
- Monthly public reports

### Audit Trail
- Transaction hashes for all actions
- Event emissions timestamped
- Decision justifications published

## Conflict Resolution

### Dispute Escalation
1. **Level 1**: Automated system (delivery confirmed)
2. **Level 2**: Admin manual review
3. **Level 3**: Community governance (future)
4. **Level 4**: Legal arbitration (off-chain)

### Appeal Process
Users can appeal admin decisions:
1. Submit appeal with evidence
2. Secondary admin review
3. Community vote (future)
4. Final decision binding

## Updates and Changes

This governance model will evolve. Changes to governance itself require:
- Public announcement (30 days notice)
- Community feedback period
- Formal vote (in DAO phase)
- Code audit if technical changes
- Clear migration path

---

**Last Updated**: January 2026  
**Next Review**: July 2026  
**Version**: 1.0.0

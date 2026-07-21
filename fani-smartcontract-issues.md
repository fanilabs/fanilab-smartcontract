# FaniLab Smart Contracts — Substantive Issues

Derived from a direct read of every contract in `contracts/` (escrow_contract,
delivery_contract, dispute_resolution_contract, fleet_management_contract,
identity_reputation_contract, settlement_contract, shared_types) plus the
project's own `PLAN.md`, `PRODUCTION_READINESS.md`, `Cargo.toml`, and CI
workflow. Every issue below references the specific function and file it was
found in — none are generic placeholders.

## Pushed to GitHub

The 10 highest-severity issues (all 6 Critical + top 4 High) have been filed
on `github.com/fanilabs/fanilab-smartcontract` and removed from this
document to avoid duplication. Track them there:

| GitHub Issue | Title |
|---|---|
| [#7](https://github.com/fanilabs/fanilab-smartcontract/issues/7) | `freeze_funds` has no authorization check — anyone can pause any escrow |
| [#8](https://github.com/fanilabs/fanilab-smartcontract/issues/8) | Post-delivery disputes are structurally unresolvable |
| [#9](https://github.com/fanilabs/fanilab-smartcontract/issues/9) | Driver reputation can only ever decrease — `increase_reputation` is never called |
| [#10](https://github.com/fanilabs/fanilab-smartcontract/issues/10) | Dual initializers leave `identity_reputation_contract` unusable with no recovery path |
| [#11](https://github.com/fanilabs/fanilab-smartcontract/issues/11) | `EscrowContract::init` does not enforce the platform fee ceiling |
| [#12](https://github.com/fanilabs/fanilab-smartcontract/issues/12) | Fleet treasury routing is never wired into the actual payout path |
| [#13](https://github.com/fanilabs/fanilab-smartcontract/issues/13) | `resolve_dispute_split` mislabels the final escrow status as `Refunded` |
| [#14](https://github.com/fanilabs/fanilab-smartcontract/issues/14) | `resolve_dispute`'s refund branch skips the balance-sufficiency guard |
| [#15](https://github.com/fanilabs/fanilab-smartcontract/issues/15) | Zero slippage protection on the settlement-swap payout path |
| [#16](https://github.com/fanilabs/fanilab-smartcontract/issues/16) | Admin can silently repoint `settlement_contract` mid-flight with no timelock |

The remaining 20 issues below are not yet filed.

---

## High — Logic & State-Machine Correctness

### 1. `create_escrow` never validates `amount > 0`
**File:** `contracts/escrow_contract/lib.rs:293-330`
**Recommended Labels:** `bug`
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
Unlike the fee validation elsewhere in this contract, `create_escrow`
accepts any `i128 amount` — including `0` or negative values — and proceeds
straight to `token::Client::transfer`. A `0`-amount escrow silently creates a
permanent, fee-less "delivery" record that can still go through the full
dispute/release lifecycle for no economic reason (storage-cost griefing). A
negative amount will panic deep inside the token contract's `transfer`
rather than surfacing a clean protocol-level error.

**Tasks:**
- Reject `amount <= 0` at the top of `create_escrow` with a dedicated
  `EscrowError` variant (e.g. `InvalidAmount`).
- Add unit tests for `amount == 0` and `amount < 0`.

---

### 2. No expiry/timeout mechanism for `Locked` escrows
**File:** `contracts/escrow_contract/lib.rs` (entire lifecycle)
**Recommended Labels:** `feature`
**Priority:** High
**Estimated Time:** 4 hours

**Description:**
Once an escrow is `Locked`, the only paths out are `release_escrow` (needs
recipient or admin), `refund_escrow` (needs sender or admin), or a dispute.
If a recipient never confirms and neither party raises a dispute — whether
by inaction, disappearance, or key loss — the sender's funds are locked
indefinitely with no automatic recourse. There is no `deadline`/`expires_at`
field on `EscrowRecord` and no permissionless "reclaim after N days of
inactivity" function anywhere in the contract.

**Tasks:**
- Add an optional `expires_at: Option<u64>` to `EscrowRecord`, set at
  `create_escrow` time (configurable by the sender or protocol default).
- Add a permissionless `reclaim_expired_escrow(delivery_id)` that refunds the
  sender once `env.ledger().timestamp() > expires_at` and status is still
  `Locked`.
- Add tests covering both the happy path (confirmed before expiry) and the
  reclaim path (expiry reached, no confirmation).

---

### 3. Delivery and escrow state machines can silently desynchronize
**File:** `contracts/delivery_contract/lib.rs` + `contracts/escrow_contract/lib.rs`
**Recommended Labels:** `enhancement`
**Priority:** High
**Estimated Time:** 4 hours

**Description:**
`DeliveryStatus` (owned by `delivery_contract`) and `EscrowStatus` (owned by
`escrow_contract`) are two independently-mutated state machines that are
only kept in sync by careful ordering of cross-contract calls in specific
functions (`cancel_delivery`, `confirm_delivery`, `raise_dispute`). There is
no invariant-checking function, no shared "delivery+escrow" combined state
read, and (per [GitHub #7](https://github.com/fanilabs/fanilab-smartcontract/issues/7))
at least one code path can mutate escrow state without any corresponding
delivery-contract transition. There is currently no way for an off-chain
indexer or auditor to detect "this delivery_id has mismatched
delivery/escrow states" short of manually cross-referencing both contracts
for every ID.

**Tasks:**
- Add a read-only `get_combined_state(delivery_id)` view (in whichever
  contract is appropriate, or a small aggregator) that fetches both records
  and flags known-invalid combinations.
- Document the intended state-machine coupling in `docs/ARCHITECTURE_DECISION_RECORDS.md`.
- Add property-based tests (per `PLAN.md`'s own stated testing goal) that
  fuzz the call ordering between the two contracts and assert no reachable
  combination violates the documented invariants.

---

### 4. `assign_driver` allows sender/recipient self-assignment, enabling reputation farming
**File:** `contracts/delivery_contract/lib.rs:166-196`
**Recommended Labels:** `security`
**Priority:** High
**Estimated Time:** 2 hours

**Description:**
`assign_driver` permits self-assignment (`caller == driver`) with no check
that `driver` differs from the delivery's `sender` or `recipient`. A user can
create a delivery where they are sender, recipient, *and* driver, then run
themselves through `mark_in_transit → confirm_delivery`, paying themselves
(minus platform fee) and incrementing their own `deliveries_completed`
/`reputation_score` with zero counterparty risk. Combined with
[GitHub #9](https://github.com/fanilabs/fanilab-smartcontract/issues/9)
(reputation only decreases via disputes, never legitimately increases), this
is currently the *only* practical way to raise a driver's reputation score,
which is a strong signal the self-dealing path is being exercised in
practice rather than being a theoretical edge case.

**Tasks:**
- Reject `assign_driver` (and/or `confirm_delivery`) when `driver == sender`
  or `driver == recipient`, unless an explicit "self-delivery" mode is a
  deliberate product decision (in which case it should be flagged and
  excluded from reputation accrual).
- Add a test asserting the self-assignment path is rejected (or, if
  intentional, that it does not affect reputation).

---

### 5. `dispute_time_limit` accepts `0` at init with no minimum enforced
**File:** `contracts/dispute_resolution_contract/lib.rs:47-67`
**Recommended Labels:** `bug`
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
`init(admin, delivery_contract, escrow_contract, dispute_time_limit)` stores
`dispute_time_limit` with no floor. A value of `0` means
`raise_dispute`'s `Delivered` branch check
(`current_time > delivered_at + dispute_time_limit`) evaluates true for
essentially any call after confirmation, silently disabling the entire
post-delivery dispute window without any explicit "disputes disabled" signal
— indistinguishable from a legitimate short window at the API level.

**Tasks:**
- Enforce a sane minimum (e.g. protocol-defined constant) in `init`, or
  require an explicit separate flag to disable post-delivery disputes
  entirely rather than allowing an implicit `0`.
- Add a test for the `0` boundary condition.

---

### 6. `resolve_dispute_split_funds` reports success even when it moves zero funds
**File:** `contracts/dispute_resolution_contract/lib.rs:317-369`
**Recommended Labels:** `bug`
**Priority:** High
**Estimated Time:** 1 hour

**Description:**
The function only calls `escrow_contract::resolve_dispute_split` **if**
`escrow.status == EscrowStatus::Paused`; otherwise it falls through silently.
In both cases it unconditionally sets the local `DisputeCase.status =
DisputeStatus::Split` and emits `dispute_resolved_split`. An admin resolving
a dispute whose underlying escrow is not (or no longer) `Paused` — e.g. it
was already resolved by a different admin call, or hit the dead-end
described in [GitHub #8](https://github.com/fanilabs/fanilab-smartcontract/issues/8)
— receives a success event with **no funds actually moved**, and the
dispute record is now marked `Split` forever with no way to retry or detect
the discrepancy.

**Tasks:**
- Make the escrow-state check a hard precondition (panic with a typed error
  if `escrow.status != Paused`) instead of a silent no-op branch.
- Add a test that attempts to split-resolve a non-`Paused` escrow and
  asserts it fails loudly.

---

## Medium — Code Quality, Consistency & Robustness

### 7. `delivery_contract` uses untyped `panic!("...")` instead of typed contract errors
**File:** `contracts/delivery_contract/lib.rs` (12 call sites: lines 130, 133, 137, 143, 173, 181, 184, 208, 213, 217, 240, 243, 247, 253, 312, 317, 321, 327, 385)
**Recommended Labels:** `refactor`
**Priority:** Medium
**Estimated Time:** 3 hours

**Description:**
Every other contract in the workspace (`escrow_contract`,
`fleet_management_contract`, `identity_reputation_contract`) uses
`#[contracterror]` enums with `panic_with_error!` so that clients/SDKs can
decode structured error codes. `delivery_contract` already defines a
`DeliveryError` enum (used only by `validate_transition`) but every
authorization/lookup failure in the contract's public entry points uses raw
string `panic!("NotAuthorized")`, `panic!("DeliveryNotFound")`,
`panic!("EscrowNotConfigured")`, etc. Off-chain clients cannot distinguish
these failure modes programmatically the way they can for every other
contract in this codebase.

**Tasks:**
- Extend `DeliveryError` (or reuse `shared_types::FaniLabError`) with
  variants for `NotAuthorized`, `DeliveryNotFound`, `EscrowNotConfigured`.
- Replace all raw `panic!` string calls with `panic_with_error!`.
- Update `delivery_contract/test.rs` assertions that currently match on
  panic message strings to match on the typed error instead.

---

### 8. Three divergent `DriverProfile` definitions with no single source of truth
**File:** `contracts/shared_types/lib.rs:540-548`, `contracts/identity_reputation_contract/lib.rs:14-22`, `contracts/delivery_contract/lib.rs` (uses `shared_types::DriverProfile` but mutates it locally)
**Recommended Labels:** `bug`
**Priority:** Medium
**Estimated Time:** 3 hours

**Description:**
`shared_types::DriverProfile` and `identity_reputation_contract::DriverProfile`
are structurally identical (`address`, `deliveries_completed`,
`reputation_score`, `registered_at`, `kyc_verified`) but are two distinct
Rust types stored under two distinct contracts' storage, updated
independently (see [GitHub #9](https://github.com/fanilabs/fanilab-smartcontract/issues/9)).
A driver querying "their profile" gets a different, inconsistent answer
depending on which contract they ask.

**Tasks:**
- Pick one contract as the canonical owner of driver profile data (the
  identity/reputation contract is the natural fit) and have
  `delivery_contract::get_driver_profile` cross-call into it instead of
  maintaining a parallel copy.
- Remove the now-redundant type/storage from the deprecated location.

---

### 9. Instance storage TTL is only extended by two of many admin-writing functions
**File:** `contracts/escrow_contract/lib.rs:245-289` (extends TTL) vs `159-244` (does not)
**Recommended Labels:** `bug`
**Priority:** Medium
**Estimated Time:** 2 hours

**Description:**
`propose_admin` and `accept_admin` both call
`env.storage().instance().extend_ttl(...)`, but `init`,
`update_platform_fee`, and `set_settlement_contract` — all of which write to
instance storage (`Admin`, `ProtocolConfig`, `SettlementContract`) — never
extend the instance TTL. A protocol that only ever adjusts its fee or
settlement contract address (plausible for a stable, low-churn admin
configuration) never touches the admin-transfer functions and could see its
instance storage TTL lapse and the entry archived, requiring an explicit
(and non-trivial) restoration transaction before the contract is usable
again.

**Tasks:**
- Extend instance TTL in every function that touches instance storage, or
  centralize this behind a single helper called at the top of every
  `#[contractimpl]` entry point.
- Add a test that simulates ledger advancement past the TTL threshold with
  only fee-update calls in between and confirms the instance data survives.

---

### 10. `ESCROW_TTL_THRESHOLD == ESCROW_TTL_EXTEND_TO` leaves no safety margin
**File:** `contracts/escrow_contract/lib.rs:14-15`
**Recommended Labels:** `bug`
**Priority:** Medium
**Estimated Time:** 1 hour

**Description:**
Both constants are `518400` ledgers. Soroban's `extend_ttl(threshold,
extend_to)` only bumps TTL when the *current* TTL falls at or below
`threshold`, and then sets it to `extend_to`. With threshold equal to
extend_to, an entry is only ever extended at the exact moment it would
otherwise be one ledger from becoming eligible for archival on the next
access — any delay (e.g., a spike in network activity delaying the
extending transaction by even one ledger) risks the record archiving before
it's renewed. Typical Soroban guidance is to keep meaningful headroom
between the two.

**Tasks:**
- Set `ESCROW_TTL_EXTEND_TO` comfortably above `ESCROW_TTL_THRESHOLD` (e.g.
  threshold at ~30 days, extend-to at ~60-90 days) across all contracts that
  share this pattern (`delivery_contract`, `dispute_resolution_contract`,
  `fleet_management_contract`, `identity_reputation_contract` all hard-code
  `518400, 518400` inline).
- Factor the TTL constants into `shared_types` so all six contracts use one
  consistent, correctly-spaced pair instead of six independent copies of the
  same magic numbers.

---

### 11. No enumeration/pagination API anywhere in the protocol
**File:** all contracts
**Recommended Labels:** `feature`
**Priority:** Medium
**Estimated Time:** 4 hours

**Description:**
Every "get" function in every contract (`get_delivery`, `get_escrow`,
`get_dispute`, `get_fleet`, `get_driver_profile`, ...) requires the caller to
already know the numeric ID or address. There is no way, on-chain, to answer
"what deliveries does sender X have open", "what disputes are currently
open", or "which drivers belong to fleet Y" without an off-chain indexer
replaying every event from genesis. For a protocol whose own `PLAN.md`
identifies "monitoring dashboards" and "SDK wrappers" as roadmap items, this
is a real integration blocker, not a nice-to-have.

**Tasks:**
- At minimum, add secondary indexes (e.g. `Vec<DeliveryId>` per
  sender/recipient/driver) maintained alongside the primary records, with
  bounded-size safeguards to avoid unbounded storage growth.
- Document the recommended event-replay indexing approach in
  `docs/API.md` as an explicit interim solution if on-chain indexes are
  deferred.

---

### 12. No enumerable fleet driver roster
**File:** `contracts/fleet_management_contract/lib.rs`
**Recommended Labels:** `feature`
**Priority:** Medium
**Estimated Time:** 2 hours

**Description:**
`DriverFleet(FleetId, Address)` storage lets you check one driver's status
in one fleet if you already know both, but there is no reverse index (list
of drivers per fleet, or list of fleets per driver). A fleet owner cannot
enumerate their own roster on-chain; `total_active_drivers` is a bare count
with no backing list, so reconciling it (e.g. after issue #10's TTL
expiry-margin problem silently drops a `DriverFleet` entry) is impossible
without an external log of every
`driver_invited`/`invite_accepted`/`driver_removed` event.

**Tasks:**
- Add a `Vec<Address>` (or similar bounded structure) roster per fleet,
  updated in `accept_fleet_invite`/`remove_driver_from_fleet`.
- Add a `get_fleet_roster(fleet_id) -> Vec<Address>` read function.

---

### 13. No batch delivery/escrow creation despite being a named roadmap item
**File:** `contracts/delivery_contract/lib.rs:78-120`, `contracts/escrow_contract/lib.rs:293-330`
**Recommended Labels:** `feature`
**Priority:** Medium
**Estimated Time:** 4 hours

**Description:**
`PLAN.md`'s "New Features" section explicitly lists "Implement batch
delivery creation for enterprise users" as a Medium-High priority item, but
`create_delivery`/`create_escrow` are strictly single-item calls. An
enterprise sender creating hundreds of deliveries currently pays full
transaction overhead per delivery with no amortization.

**Tasks:**
- Add a `create_deliveries_batch(sender, recipient, metadata_list) ->
  Vec<DeliveryId>` entry point with a bounded max-batch-size to keep the
  transaction within Soroban resource limits.
- Mirror with a batched escrow-funding entry point, or document that batch
  deliveries fund escrow individually and why.

---

### 14. `settlement_contract` is a complete no-op stub already wired into the live payout path
**File:** `contracts/settlement_contract/src/lib.rs`, `contracts/escrow_contract/lib.rs:60-93`
**Recommended Labels:** `feature`
**Priority:** Medium
**Estimated Time:** N/A (tracking issue)

**Description:**
The contract's own header comment says "This contract will handle currency
conversions... Implementation to be added in Phase 3" and every function body
is either empty or returns `None`. Yet `escrow_contract` already has a live
`set_settlement_contract`/`get_settlement_contract` admin surface and a
`payout_driver` code path that cross-calls into it on every release. This
means the integration risk described in
[GitHub #15](https://github.com/fanilabs/fanilab-smartcontract/issues/15) and
[GitHub #16](https://github.com/fanilabs/fanilab-smartcontract/issues/16)
is already deployed and live in the escrow contract well before the
settlement logic itself exists — the stub should either be feature-flagged
out of the payout path until Phase 3 lands, or explicitly tracked so it
isn't forgotten.

**Tasks:**
- Add a tracking test/comment that fails CI (or at minimum a loud runtime
  assertion) if `execute_settlement_swap` is ever reached with real logic
  still absent, to prevent an accidental mainnet deployment with silent
  no-op swaps.
- Prioritize implementing or explicitly descoping Phase 3 before any
  mainnet deployment, per `PRODUCTION_READINESS.md`'s own "Next Steps for
  Mainnet Launch."

---

### 15. No emergency pause / circuit breaker across the protocol
**File:** all contracts
**Recommended Labels:** `security`
**Priority:** Medium
**Estimated Time:** 4 hours

**Description:**
There is no protocol-wide "pause all fund movement" primitive. The closest
analog, `freeze_funds`, operates on a single `delivery_id` at a time and
(per [GitHub #7](https://github.com/fanilabs/fanilab-smartcontract/issues/7))
isn't even access-controlled today. If a critical bug is discovered in
production affecting `release_escrow` or `refund_escrow` broadly, there is
currently no way for the admin to halt fund movement protocol-wide while a
fix is prepared and deployed.

**Tasks:**
- Add a `paused: bool` instance-storage flag to `escrow_contract`
  (and optionally `delivery_contract`), gated by an admin-only
  `set_paused` function.
- Check the flag at the top of every fund-moving entry point
  (`release_escrow`, `refund_escrow`, `resolve_dispute`,
  `resolve_dispute_split`) and reject with a typed error when paused.
- Document the pause/unpause runbook in `docs/GOVERNANCE.md`.

---

### 16. No admin setter for `dispute_time_limit` after `init`
**File:** `contracts/dispute_resolution_contract/lib.rs:47-134`
**Recommended Labels:** `enhancement`
**Priority:** Medium
**Estimated Time:** 1 hour

**Description:**
`dispute_time_limit` is set once in `init` and read via
`get_dispute_time_limit`, but there is no corresponding
`update_dispute_time_limit` admin function — unlike `platform_fee_bps` in
`escrow_contract`, which has `update_platform_fee`. If the initial value
turns out to be miscalibrated (see also issue #5, the `0`-limit boundary
case), the only fix is redeployment.

**Tasks:**
- Add an admin-gated `update_dispute_time_limit(caller, new_limit)` with an
  event emission, mirroring `update_platform_fee`'s pattern.

---

### 17. `CargoDescriptor`/`DeliveryMetadata` accept unbounded input with no validation
**File:** `contracts/shared_types/lib.rs:559-584`, `contracts/delivery_contract/lib.rs:78-120`
**Recommended Labels:** `bug`
**Priority:** Medium
**Estimated Time:** 2 hours

**Description:**
`create_delivery` accepts a caller-supplied `DeliveryMetadata` containing
free-text `origin`/`destination` `String`s and a `weight_grams: u32` with no
length or magnitude bounds anywhere in the call path. Since this metadata is
persisted (`extend_ttl`'d for ~30 days per entry), a sender can pad
`origin`/`destination` with large strings to inflate persistent-storage rent
costs for the contract with no corresponding validation ceiling, and
`identity_reputation_contract::increase_reputation`'s bonus-point logic
(`weight_grams > 5000 → +3 points`) is never actually reachable to sanity
check its inputs (see
[GitHub #9](https://github.com/fanilabs/fanilab-smartcontract/issues/9)) so
an unbounded value has no real effect today — but will as soon as that issue
is fixed.

**Tasks:**
- Add explicit max-length checks on `origin`/`destination` and a sane
  `weight_grams` ceiling in `create_delivery`, returning a typed error on
  violation.
- Add tests for the boundary and over-limit cases.

---

## Low — Documentation, Process & Housekeeping

### 18. `PRODUCTION_READINESS.md` claims contradict the codebase's actual state
**File:** `PRODUCTION_READINESS.md`
**Recommended Labels:** `documentation`, `test`
**Priority:** Low
**Estimated Time:** 2 hours

**Description:**
The checklist rates "Security" 10/10 and specifically claims "Test
reentrancy protection mechanisms" (under Section 3) and "Zero critical
security vulnerabilities" (Summary), but this review found an
unauthenticated fund-freezing function
([GitHub #7](https://github.com/fanilabs/fanilab-smartcontract/issues/7)), a
structurally broken dispute path
([GitHub #8](https://github.com/fanilabs/fanilab-smartcontract/issues/8)),
and no reentrancy-specific tests exist in any `test.rs` in the repository
(confirmed via search — no test references reentrancy, malicious callback
contracts, or nested `invoke_contract` attack scenarios). Shipping this
document as-is to an external auditor or investor materially overstates the
project's actual security posture.

**Tasks:**
- Either correct the document to reflect the true current state, or (better)
  close the gaps identified in this issue list first and re-verify each
  checklist claim against actual test coverage before re-publishing.
- Add at least one reentrancy-focused test (e.g. a malicious mock token or
  mock settlement contract that re-enters `release_escrow` mid-transfer) to
  back up the "tested" claim.

---

### 19. Unused `shared_types` dependency in `settlement_contract`
**File:** `contracts/settlement_contract/Cargo.toml`
**Recommended Labels:** `refactor`
**Priority:** Low
**Estimated Time:** 15 minutes

**Description:**
`settlement_contract/Cargo.toml` declares `shared_types = { path =
"../shared_types" }`, but `src/lib.rs` never imports anything from it (only
`soroban_sdk` types are used). This is a small but concrete instance of the
broader "declared surface area exceeds implemented functionality" pattern
documented in issue #14.

**Tasks:**
- Remove the unused dependency, or if it's intentionally reserved for the
  Phase 3 implementation, add a `// TODO(phase-3)` comment explaining why
  it's present despite being unused, so `cargo machete`/`cargo udeps`-style
  audits don't repeatedly flag it as dead weight.

---

### 20. CI runs `cargo outdated` and `cargo audit` but not `cargo machete`/unused-dependency checks, and coverage has no enforced floor
**File:** `.github/workflows/ci.yml`
**Recommended Labels:** `test`
**Priority:** Low
**Estimated Time:** 2 hours

**Description:**
The CI pipeline installs `cargo-tarpaulin` and uploads to Codecov with
`fail_ci_if_error: false`, meaning a coverage regression (or the coverage
step failing outright) never fails the build — directly contradicting
`PRODUCTION_READINESS.md`'s claim of "Test coverage > 80%" being enforced.
Similarly, `cargo outdated` runs with `continue-on-error: true`, so dependency
staleness (relevant given `CHANGELOG.md`/`SOROBAN_SDK_27_MIGRATION.md`
document a recent, non-trivial SDK migration) is informational only and
cannot block a merge, and there is no dependency-usage check (e.g. `cargo
machete`) that would have caught issue #19 automatically.

**Tasks:**
- Set a minimum coverage threshold in `codecov.yml` and make the Codecov
  check required on PRs (or set `fail_ci_if_error: true` if coverage upload
  itself should be a hard gate).
- Add a `cargo machete` (or equivalent) step to catch unused dependencies
  automatically.
- Revisit whether `cargo outdated`'s `continue-on-error: true` should be
  tightened now that the SDK 27.0.0 migration is complete and the dependency
  surface has stabilized.

---

## Summary by Priority

| Priority | Count | Issues |
|----------|-------|--------|
| High (in this doc) | 6 | 1, 2, 3, 4, 5, 6 |
| Medium | 11 | 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17 |
| Low | 3 | 18, 19, 20 |
| Filed on GitHub (Critical + top High) | 10 | #7–#16 (see table above) |

## Summary by Contract

| Contract | Issues in this doc | Filed on GitHub |
|----------|--------------------|-----------------|
| `escrow_contract` | 1, 2, 9, 10, 15 | #7, #11, #12, #13, #14, #15, #16 |
| `delivery_contract` | 3, 4, 7, 8, 11, 17 | — |
| `dispute_resolution_contract` | 5, 6, 16 | #8 |
| `identity_reputation_contract` | 8 | #9, #10 |
| `fleet_management_contract` | 10, 11, 12 | #12 |
| `settlement_contract` | 14, 19 | #15 |
| `shared_types` | 8, 10, 17 | — |
| Cross-cutting / process | 3, 11, 15, 18, 20 | #7, #8 |

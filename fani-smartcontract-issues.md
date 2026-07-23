# FaniLab Smart Contracts — Substantive Issues

Derived from a direct read of every contract in `contracts/` (escrow_contract,
delivery_contract, dispute_resolution_contract, fleet_management_contract,
identity_reputation_contract, settlement_contract, shared_types) plus the
project's own `PLAN.md`, `PRODUCTION_READINESS.md`, `Cargo.toml`, and CI
workflow. Every issue below references the specific function and file it was
found in — none are generic placeholders.

This document consolidates two review passes: an initial pass (all 30 of whose issues — 6 Critical, 4 High, and local issues #11–#30 — have since been filed to GitHub, see below) and a follow-up pass extending coverage to cross-contract architecture, testing gaps, CI/CD, deployment tooling, and documentation accuracy (issues #31–#80, of which #31–#70 have since also been filed to GitHub, see below; #71–#80 are still tracked in this document).

## Pushed to GitHub

70 issues have been filed on `github.com/fanilabs/fanilab-smartcontract` and removed from this document to avoid duplication: the original 10 highest-severity findings (6 Critical + 4 High), the full remaining High/Medium/Low-classified backlog from the initial review pass (local issues #11–#30), and the follow-up pass's architecture/testing/tooling findings (local issues #31–#70). Track them there:

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
| [#17](https://github.com/fanilabs/fanilab-smartcontract/issues/17) | `create_escrow` never validates `amount > 0` |
| [#18](https://github.com/fanilabs/fanilab-smartcontract/issues/18) | No expiry/timeout mechanism for `Locked` escrows |
| [#19](https://github.com/fanilabs/fanilab-smartcontract/issues/19) | Delivery and escrow state machines can silently desynchronize |
| [#20](https://github.com/fanilabs/fanilab-smartcontract/issues/20) | `assign_driver` allows sender/recipient self-assignment, enabling reputation farming |
| [#21](https://github.com/fanilabs/fanilab-smartcontract/issues/21) | `dispute_time_limit` accepts `0` at init with no minimum enforced |
| [#22](https://github.com/fanilabs/fanilab-smartcontract/issues/22) | `resolve_dispute_split_funds` reports success even when it moves zero funds |
| [#23](https://github.com/fanilabs/fanilab-smartcontract/issues/23) | `delivery_contract` uses untyped `panic!("...")` instead of typed contract errors |
| [#24](https://github.com/fanilabs/fanilab-smartcontract/issues/24) | Three divergent `DriverProfile` definitions with no single source of truth |
| [#25](https://github.com/fanilabs/fanilab-smartcontract/issues/25) | Instance storage TTL is only extended by two of many admin-writing functions |
| [#26](https://github.com/fanilabs/fanilab-smartcontract/issues/26) | `ESCROW_TTL_THRESHOLD == ESCROW_TTL_EXTEND_TO` leaves no safety margin |
| [#27](https://github.com/fanilabs/fanilab-smartcontract/issues/27) | No enumeration/pagination API anywhere in the protocol |
| [#28](https://github.com/fanilabs/fanilab-smartcontract/issues/28) | No enumerable fleet driver roster |
| [#29](https://github.com/fanilabs/fanilab-smartcontract/issues/29) | No batch delivery/escrow creation despite being a named roadmap item |
| [#30](https://github.com/fanilabs/fanilab-smartcontract/issues/30) | `settlement_contract` is a complete no-op stub already wired into the live payout path |
| [#31](https://github.com/fanilabs/fanilab-smartcontract/issues/31) | No emergency pause / circuit breaker across the protocol |
| [#32](https://github.com/fanilabs/fanilab-smartcontract/issues/32) | No admin setter for `dispute_time_limit` after `init` |
| [#33](https://github.com/fanilabs/fanilab-smartcontract/issues/33) | `CargoDescriptor`/`DeliveryMetadata` accept unbounded input with no validation |
| [#34](https://github.com/fanilabs/fanilab-smartcontract/issues/34) | `PRODUCTION_READINESS.md` claims contradict the codebase's actual state |
| [#35](https://github.com/fanilabs/fanilab-smartcontract/issues/35) | Unused `shared_types` dependency in `settlement_contract` |
| [#36](https://github.com/fanilabs/fanilab-smartcontract/issues/36) | CI runs `cargo outdated` and `cargo audit` but not `cargo machete`/unused-dependency checks, and coverage has no enforced floor |
| [#37](https://github.com/fanilabs/fanilab-smartcontract/issues/37) | `escrow_contract::get_status` is a dead stub that always returns `Pending` |
| [#38](https://github.com/fanilabs/fanilab-smartcontract/issues/38) | `create_escrow` accepts any token address, making `get_token()` misleading |
| [#39](https://github.com/fanilabs/fanilab-smartcontract/issues/39) | `register_fleet` permanently fails for any owner already registered as a driver |
| [#40](https://github.com/fanilabs/fanilab-smartcontract/issues/40) | `dispute_resolution_contract::remove_admin` can remove the last admin, bricking governance |
| [#41](https://github.com/fanilabs/fanilab-smartcontract/issues/41) | Two divergent `UserProfile` definitions with different field names |
| [#42](https://github.com/fanilabs/fanilab-smartcontract/issues/42) | `DeliveryDetails` and `PartyAddresses` are fully-defined dead types |
| [#43](https://github.com/fanilabs/fanilab-smartcontract/issues/43) | `AuthorizedContract` allowlist is built but never consulted |
| [#44](https://github.com/fanilabs/fanilab-smartcontract/issues/44) | Driver tier system is never wired into `assign_driver` despite being documented |
| [#45](https://github.com/fanilabs/fanilab-smartcontract/issues/45) | `DeliveryMetadata.delivery_id` is never validated against the real `DeliveryId` |
| [#46](https://github.com/fanilabs/fanilab-smartcontract/issues/46) | `FaniLabError::DeliveryNotFound` and `EscrowError::DeliveryNotFound` carry different discriminants |
| [#47](https://github.com/fanilabs/fanilab-smartcontract/issues/47) | Typed event payload structs and topic constants in `shared_types::events` are unused |
| [#48](https://github.com/fanilabs/fanilab-smartcontract/issues/48) | No reputation decay for inactive drivers despite being a named roadmap item |
| [#49](https://github.com/fanilabs/fanilab-smartcontract/issues/49) | `add_evidence_hash` allows unbounded growth of a single storage entry |
| [#50](https://github.com/fanilabs/fanilab-smartcontract/issues/50) | No automated dispute evidence verification system despite being a named roadmap item |
| [#51](https://github.com/fanilabs/fanilab-smartcontract/issues/51) | Dispute resolution's reputation-penalty cross-call is never exercised by any test |
| [#52](https://github.com/fanilabs/fanilab-smartcontract/issues/52) | No `proptest` dependency anywhere despite extensive property-testing documentation |
| [#53](https://github.com/fanilabs/fanilab-smartcontract/issues/53) | No fuzz targets despite `SECURITY_AUDIT.md` prescribing `cargo fuzz` commands |
| [#54](https://github.com/fanilabs/fanilab-smartcontract/issues/54) | Two-step admin transfer (`propose_admin`/`accept_admin`) has zero test coverage |
| [#55](https://github.com/fanilabs/fanilab-smartcontract/issues/55) | `settlement_contract` test suite only exercises `init` |
| [#56](https://github.com/fanilabs/fanilab-smartcontract/issues/56) | `deploy-all-contracts.sh` still builds with the pre-migration `wasm32-unknown-unknown` target |
| [#57](https://github.com/fanilabs/fanilab-smartcontract/issues/57) | `Makefile` targets still use `wasm32-unknown-unknown` and cover only 3 of 6 contracts |
| [#58](https://github.com/fanilabs/fanilab-smartcontract/issues/58) | `initialize-all-contracts.sh` only initializes 2 of the 6 deployed contracts |
| [#59](https://github.com/fanilabs/fanilab-smartcontract/issues/59) | Deploy script's error handling after `cargo build` is unreachable dead code |
| [#60](https://github.com/fanilabs/fanilab-smartcontract/issues/60) | `.env.example` doesn't match the variables `DEPLOYMENT.md` and the scripts actually need |
| [#61](https://github.com/fanilabs/fanilab-smartcontract/issues/61) | Release workflow's "Optimize WASM" step performs no optimization |
| [#62](https://github.com/fanilabs/fanilab-smartcontract/issues/62) | CI reinstalls `cargo-audit`/`cargo-outdated`/`cargo-tarpaulin`/`cargo-deny` from source on every run |
| [#63](https://github.com/fanilabs/fanilab-smartcontract/issues/63) | No CI job enforces `--locked`, despite repeated manual `Cargo.lock` pinning fire-drills |
| [#64](https://github.com/fanilabs/fanilab-smartcontract/issues/64) | `docs/API.md`'s table of contents promises 4 contracts it never documents |
| [#65](https://github.com/fanilabs/fanilab-smartcontract/issues/65) | `docs/API.md` footer claims Soroban SDK 22.0.1, three versions behind actual |
| [#66](https://github.com/fanilabs/fanilab-smartcontract/issues/66) | Three architecture/design docs are committed as completely empty files |
| [#67](https://github.com/fanilabs/fanilab-smartcontract/issues/67) | Docs disagree with themselves on whether the protocol has 6 or 7 contracts |
| [#68](https://github.com/fanilabs/fanilab-smartcontract/issues/68) | `escrow_contract` and `delivery_contract` each hand-roll an identical private `is_admin` helper |
| [#69](https://github.com/fanilabs/fanilab-smartcontract/issues/69) | No admin override/recovery path in `fleet_management_contract` for a compromised owner key |
| [#70](https://github.com/fanilabs/fanilab-smartcontract/issues/70) | `update_fleet_treasury` has no timelock, cooldown, or driver notice |
| [#71](https://github.com/fanilabs/fanilab-smartcontract/issues/71) | No multi-signature support for fleet management despite being a named roadmap item |
| [#72](https://github.com/fanilabs/fanilab-smartcontract/issues/72) | No dynamic, volume-based fee adjustment despite being a named roadmap item |
| [#73](https://github.com/fanilabs/fanilab-smartcontract/issues/73) | No recovery mechanism for tokens sent directly to `escrow_contract` outside `create_escrow` |
| [#74](https://github.com/fanilabs/fanilab-smartcontract/issues/74) | No on-chain aggregate TVL view despite `MONITORING.md` naming it a key metric |
| [#75](https://github.com/fanilabs/fanilab-smartcontract/issues/75) | `register_user`/`UserProfile` are fully implemented but never consumed anywhere |
| [#76](https://github.com/fanilabs/fanilab-smartcontract/issues/76) | No way to enumerate current admins of `dispute_resolution_contract` |

The remaining 10 issues below (#71–#80) are not yet filed.

---

## Index

| # | Title | Labels |
|---|---|---|
| 71 | Admin/governance model is reinvented three different ways across the six contracts | refactor |
| 72 | `docs/API.md` documents 30+ functions but shows a usage example for exactly one | documentation |
| 73 | No translated documentation despite being an explicit, prioritized roadmap item | documentation |
| 74 | No SDK wrapper package despite being a named roadmap item and already-documented usage snippets | feature |
| 75 | No contract migration tooling despite `UPGRADE_GUIDE.md` documenting a `migrate_to_vN` pattern | feature |
| 76 | Fee-calculation-and-payout logic is triplicated across three escrow functions | refactor |
| 77 | `DriverFleetStatus` has no terminal "removed" state, destroying fleet membership history | enhancement |
| 78 | No integration test scaffolding between `fleet_management_contract` and `escrow_contract` | test |
| 79 | Core wire-format types in `shared_types` have no dedicated equality/round-trip tests | test |
| 80 | `CHANGELOG.md`'s `[Unreleased]` section is stale relative to the completed SDK 27 migration | documentation |

---

## Additional Findings — Architecture, Testing, Documentation & Tooling (Issues #71–#80)

A follow-up review pass covering cross-contract architecture, admin/governance models, test-coverage gaps, CI/CD and deployment tooling, and documentation accuracy — building on the initial pass now filed as GitHub #7–#36 and this follow-up pass's own earlier batches, now filed as GitHub #37–#76, without duplicating any of those findings.

---

### 71. Admin/governance model is reinvented three different ways across the six contracts

**Summary:** `escrow_contract`/`delivery_contract`/`fleet_management_contract`/`identity_reputation_contract` each use a single `Admin: Address` instance key with no rotation-in-place multi-admin support, while `dispute_resolution_contract` alone uses an open-ended `Admin(Address) -> bool` multi-admin map — two structurally incompatible governance models coexist in one protocol with no shared abstraction.

**Background:** Confirmed by direct inspection: `escrow_contract/lib.rs:163` (`StorageKey::Admin` single address, with `propose_admin`/`accept_admin` for rotation), `delivery_contract/lib.rs:64` (same single-address pattern, no rotation mechanism at all), `fleet_management_contract/lib.rs:74` (single address, no rotation), `identity_reputation_contract/lib.rs:55,68` (single address, no rotation, and two different init entry points besides — tracked separately as GH #10). `dispute_resolution_contract/lib.rs:33-40,66` uses `DataKey::Admin(Address) -> bool`, an entirely different shape supporting an arbitrary number of admins with no owner-of-record at all.

**Problem Statement:** This isn't just cosmetic inconsistency — it means the *security properties* of "who controls this contract" differ meaningfully depending which of the six contracts you're looking at: four contracts have exactly one admin with a secure two-step-or-nothing rotation story (and no rotation at all for three of the four), while the fifth has an unbounded admin set with the single-point-of-failure risk from GitHub #40 (last-admin removal) that the single-admin contracts structurally cannot have (since none of them support *removing* the sole admin without a replacement already being designated via `propose_admin`/`accept_admin` — except `delivery_contract` and `fleet_management_contract`, which have no rotation mechanism at all and would require a full migration to ever change admins). A protocol-wide governance model, even a simple one, would let every contract share the same well-understood security properties instead of six-going-on-two independently-reasoned-about designs.

**Proposed Solution:** Design one shared governance primitive in `shared_types` (e.g., a multi-admin set with a minimum-count floor and a consistent propose/accept rotation pattern) and migrate all six contracts onto it, documented as a new ADR.

**Acceptance Criteria:**
- [ ] A single, shared admin/governance abstraction exists in `shared_types`.
- [ ] All six contracts use it, with consistent security properties (rotation mechanism, minimum-admin-count floor) across the board.
- [ ] The design is documented as a new entry in `docs/ARCHITECTURE_DECISION_RECORDS.md`.

**Technical Notes:** This is a large, cross-cutting refactor best sequenced after GitHub #40 (last-admin protection) and GitHub #68 (duplicated `is_admin` helper) land individually, since both are natural building blocks toward this consolidation rather than competing with it.

**Relevant Files:** `contracts/escrow_contract/lib.rs:159-289`, `contracts/delivery_contract/lib.rs:60-76`, `contracts/fleet_management_contract/lib.rs:70-96`, `contracts/identity_reputation_contract/lib.rs:51-75`, `contracts/dispute_resolution_contract/lib.rs:33-92`

**Testing Requirements:** Full regression test suite across all six contracts after migration, plus new tests for the shared abstraction itself in `shared_types`.

**Definition of Done:** Every contract in the workspace uses the same governance primitive with the same, well-understood security guarantees.

**Suggested Labels:** `refactor`

---

### 72. `docs/API.md` documents 30+ functions but shows a usage example for exactly one

**Summary:** Of every function documented across `docs/API.md`'s Escrow and Delivery sections, only `init` (`:32-39`) has an actual code example — every other function (roughly 20 across the two documented contracts, before even counting the four undocumented ones from GitHub #64) has a parameter/error list but no example call.

**Background:** `docs/API.md:16-392` documents `init`, `update_platform_fee`, `propose_admin`, `accept_admin`, `set_settlement_contract`, `create_escrow`, `release_escrow`, `refund_escrow`, `raise_dispute`, `resolve_dispute`, `resolve_dispute_split`, six query functions, `create_delivery`, `assign_driver`, `mark_in_transit`, `confirm_delivery`, `cancel_delivery`, `raise_dispute` (delivery), and two more query functions. Exactly one of these (`init`) includes a fenced code block showing how to actually call it.

**Problem Statement:** `PLAN.md:27` explicitly lists "Build interactive API examples using Stellar SDK" as a High-priority documentation task — this gap is a direct, measurable instance of that unaddressed roadmap item. A reference doc with parameter lists but almost no call examples is significantly harder to integrate against than one with worked examples for every state-changing function, especially for functions with several parameters and multiple error cases (e.g., `resolve_dispute_split`'s three parameters and three documented error cases have no example showing correct argument order or types).

**Proposed Solution:** Add a minimal Rust (or TypeScript, matching the doc's existing SDK section) usage example for every state-changing function currently missing one, following the exact style already used for `init`.

**Acceptance Criteria:**
- [ ] Every state-changing function documented in `docs/API.md` has at least one accompanying usage example.

**Technical Notes:** This can be done incrementally per-contract; combining with the section rewrite from GitHub #64 (documenting the four missing contracts) is a natural place to establish "every function gets an example" as the doc's baseline standard going forward.

**Relevant Files:** `docs/API.md:16-392`, `PLAN.md:27`

**Testing Requirements:** N/A (documentation-only); if examples are runnable snippets rather than illustrative pseudocode, consider a doc-test or CI check that they at least compile against the current contract signatures.

**Definition of Done:** Every documented state-changing function in `docs/API.md` has a worked usage example, closing the gap with `PLAN.md`'s stated documentation priority.

**Suggested Labels:** `documentation`

---

### 73. No translated documentation despite being an explicit, prioritized roadmap item

**Summary:** `PLAN.md` lists "Translate documentation to French, Swahili, and Portuguese" as a High-priority documentation task, directly motivated by the project's stated target market (Africa and emerging markets, per `PLAN.md:4`) — no translated documentation of any kind exists in the repository.

**Background:** `PLAN.md:28`. Every file under `docs/` and every root-level doc (`README.md`, `CONTRIBUTING.md`, etc.) is English-only; there is no `docs/fr/`, `docs/sw/`, `docs/pt/`, or equivalent i18n directory structure anywhere in the repository.

**Problem Statement:** `README.md:3` frames FaniLab as serving "individuals and businesses" across a market where French, Swahili, and Portuguese are widely spoken official/regional languages — the roadmap itself recognizes this gap as high priority for the project's stated audience, and nothing has been done toward it yet, including even a scaffolding structure for future translation contributions.

**Proposed Solution:** Establish an i18n directory structure (e.g., mirroring `docs/` under `docs/fr/`, `docs/sw/`, `docs/pt/`) and translate at minimum the highest-traffic entry points — `README.md` and `CONTRIBUTING.md` — as the first concrete step, per the roadmap's own prioritization.

**Acceptance Criteria:**
- [ ] A defined directory convention for translated docs exists.
- [ ] At least `README.md` has a French, Swahili, or Portuguese translation as a first, concrete deliverable (rather than attempting all three languages across all docs in one pass).

**Technical Notes:** This is explicitly a "good first issue"/beginner-friendly task per `PLAN.md:77`'s own skill-level guidance, and a good candidate for community contribution rather than requiring deep protocol expertise.

**Relevant Files:** `README.md`, `CONTRIBUTING.md`, `PLAN.md:28`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** At least one full document has a published, linked translation in one of the three named languages, with a repeatable structure for adding more.

**Suggested Labels:** `documentation`

---

### 74. No SDK wrapper package despite being a named roadmap item and already-documented usage snippets

**Summary:** `PLAN.md` lists "Create SDK wrappers for popular languages (TypeScript, Python)" as a roadmap item, and `docs/API.md` already shows illustrative TypeScript and Rust usage snippets as if such a wrapper exists — but there is no `sdk/`, `bindings/`, `package.json`, or any client-library code anywhere in the repository.

**Background:** `PLAN.md:49`. `docs/API.md:565-581` shows:
```typescript
import { Contract, networks } from '@stellar/stellar-sdk';
const escrow = new Contract(escrowContractId);
await escrow.call('release_escrow', recipient, deliveryId);
```
This snippet uses the generic `@stellar/stellar-sdk` directly, not a FaniLab-specific wrapper — there is no dedicated package anywhere in the repository that provides typed, FaniLab-specific bindings (e.g., a typed `EscrowClient.releaseEscrow(...)` matching each contract's actual function signatures and error types).

**Problem Statement:** Every integrator today must hand-roll their own typed bindings against the raw Stellar SDK, re-deriving each contract's parameter order, types, and error codes from `docs/API.md` (itself incomplete per GitHub #64) rather than importing a maintained package — exactly the friction a dedicated SDK wrapper is meant to remove, and exactly what the roadmap already commits to delivering.

**Proposed Solution:** Scaffold a minimal TypeScript SDK package (e.g., `sdk/typescript/`) providing one typed client class per contract, generated or hand-written from each contract's actual function signatures, starting with `escrow_contract` and `delivery_contract` as the highest-traffic contracts.

**Acceptance Criteria:**
- [ ] A TypeScript package exists providing typed wrapper methods for at least `escrow_contract` and `delivery_contract`'s public functions.
- [ ] The package's methods match the contracts' actual current parameter types and names.

**Technical Notes:** Since Soroban contract WASM includes machine-readable interface metadata, consider whether the SDK bindings can be (partially) code-generated from the built `.wasm` files rather than hand-maintained, to avoid the bindings silently drifting from the contracts the way several other docs in this backlog already have.

**Relevant Files:** `docs/API.md:565-581`, `PLAN.md:49`

**Testing Requirements:** At minimum, a smoke test in the new package invoking each wrapped function against a local/sandbox Soroban environment to confirm the bindings match live contract signatures.

**Definition of Done:** A published or publishable SDK package exists with typed bindings for at least the two most-used contracts, closing the gap with `PLAN.md`'s roadmap and `docs/API.md`'s implied-but-nonexistent tooling.

**Suggested Labels:** `feature`

---

### 75. No contract migration tooling despite `UPGRADE_GUIDE.md` documenting a `migrate_to_vN` pattern

**Summary:** `docs/UPGRADE_GUIDE.md` presents a specific `migrate_to_v2` function pattern as "the" way to handle state-structure changes across a contract upgrade — no contract in the workspace implements any function following this (or any other) migration pattern, and `PLAN.md` separately lists building "contract upgrade migration tools" as an open roadmap item.

**Background:** `docs/UPGRADE_GUIDE.md:81-100` shows:
```rust
pub fn migrate_to_v2(env: Env) {
    let admin = get_admin(&env);
    admin.require_auth();
    let old_data = load_old_format(&env);
    let new_data = transform(old_data);
    save_new_format(&env, new_data);
}
```
as the documented state-migration mechanism. No contract in `contracts/` has a function named `migrate_to_v2` or matching this shape. `PLAN.md:52` separately lists "Create contract upgrade migration tools" under Tooling & Infrastructure as an open task.

**Problem Statement:** The upgrade guide presents a specific, concrete recommendation as though it's an established pattern in this codebase, but it is purely illustrative/aspirational — there is no working example anywhere to base a real migration on, and no tooling exists to help generate or validate one when the time comes. Given several issues in this backlog (e.g., #25's `UserProfile` consolidation, #71's governance-model unification) would themselves require exactly this kind of state migration to land safely on a live deployment, the absence of any real, tested migration pattern is a concrete blocker for shipping those fixes without a full redeploy-and-data-loss cycle.

**Proposed Solution:** Implement one real, tested migration function for an actual pending change already identified in this backlog (e.g., the `UserProfile` field rename from GitHub #41) as a template others can follow, and document any lessons learned back into `docs/UPGRADE_GUIDE.md`.

**Acceptance Criteria:**
- [ ] At least one contract has a real, tested state-migration function following (or knowingly improving upon) the pattern `docs/UPGRADE_GUIDE.md` documents.
- [ ] `docs/UPGRADE_GUIDE.md`'s example is either validated as accurate by this real implementation or corrected to match what actually works in the current SDK version.

**Technical Notes:** Soroban's actual upgrade mechanism (`update_current_contract_wasm`) and storage-schema migration are two distinct concerns — this issue is scoped to the latter (in-place data transformation), which is the part `docs/UPGRADE_GUIDE.md`'s example addresses.

**Relevant Files:** `docs/UPGRADE_GUIDE.md:81-100`, `PLAN.md:52`

**Testing Requirements:** A test that seeds "old-format" state, invokes the migration function, and asserts the new format is correctly populated with no data loss.

**Definition of Done:** At least one real, working, tested migration function exists in the codebase, proving out the pattern the upgrade guide documents (or replacing it with one that actually works).

**Suggested Labels:** `feature`

---

### 76. Fee-calculation-and-payout logic is triplicated across three escrow functions

**Summary:** The exact sequence "calculate platform fee from `record.amount`, pay the driver the remainder via `payout_driver`, transfer the fee to admin if nonzero" is independently duplicated in `release_escrow`, `resolve_dispute` (its `release_to_driver` branch), and is a near-variant in `resolve_dispute_split` — three copies of logic that has already, per GitHub #13/#14, been a source of real bugs from the copies drifting out of sync.

**Background:** `contracts/escrow_contract/lib.rs:349-371` (`release_escrow`), `:438-461` (`resolve_dispute`'s payout branch), and `:496-518` (`resolve_dispute_split`) each independently compute `calculate_fee`, call `payout_driver`, and conditionally transfer the fee to admin — with small, easy-to-miss differences between the three (e.g., `resolve_dispute_split` doesn't route through `payout_driver`'s settlement-contract integration at all, transferring directly via `token::Client` instead, unlike the other two).

**Problem Statement:** GitHub issues #13 and #14 (already filed, both about `resolve_dispute`/`resolve_dispute_split` bugs) are exactly the kind of defect that triplicated, hand-copied logic produces — a fix or invariant enforced in one copy silently doesn't apply to the other two. This issue is the underlying structural cause worth fixing once, rather than continuing to patch three independently-maintained copies of the same financial logic as further bugs are found in each.

**Proposed Solution:** Extract a single private helper, e.g., `fn settle_escrow_funds(env: &Env, record: &EscrowRecord, driver_amount: i128, admin_amount: i128)`, and have all three call sites route through it, eliminating the possibility of the three copies disagreeing on behavior (including whether `resolve_dispute_split` should also integrate with `payout_driver`'s settlement-contract routing, which it currently and perhaps unintentionally does not).

**Acceptance Criteria:**
- [ ] Exactly one implementation of the fee-calculation-and-payout sequence exists.
- [ ] All three call sites use it.
- [ ] Existing tests for all three functions continue to pass unmodified (behavior-preserving refactor), except where the inconsistency itself (e.g., `resolve_dispute_split` bypassing settlement routing) is deliberately resolved as part of this change, in which case new tests cover the corrected behavior.

**Technical Notes:** This refactor should be sequenced after GitHub issues #13/#14 are fixed, so the shared helper is built from the corrected logic rather than propagating an existing known bug into all three call sites at once.

**Relevant Files:** `contracts/escrow_contract/lib.rs:349-371, 438-461, 496-518`

**Testing Requirements:** Full regression pass across `escrow_contract/test.rs`'s existing release/dispute/split tests.

**Definition of Done:** Fee-calculation-and-payout logic exists in exactly one place, shared by all three functions that need it.

**Suggested Labels:** `refactor`

---

### 77. `DriverFleetStatus` has no terminal "removed" state, destroying fleet membership history

**Summary:** `remove_driver_from_fleet` deletes the driver's `DriverFleet` storage entry entirely rather than transitioning it to a terminal "removed" state, so there is no way to distinguish "this driver was never part of this fleet" from "this driver used to be part of this fleet and left/was removed."

**Background:** `contracts/fleet_management_contract/lib.rs:26-33` defines `DriverFleetStatus` with only two variants: `Pending` and `Active`. `remove_driver_from_fleet` (`:300-338`) ends with `env.storage().persistent().remove(&invite_key);` — a full deletion, not a status transition. `get_driver_fleet_status` (`:368-376`) therefore returns `None` both for a driver who was never invited and one who was actively part of the fleet for months before being removed.

**Problem Statement:** This destroys exactly the kind of historical audit trail `docs/GOVERNANCE.md`'s "Accountability"/"Audit Trail" sections describe as important, and makes off-chain reputation/history systems unable to distinguish (from on-chain state alone) a driver with zero fleet history from one with an extensive-but-now-ended fleet history — a distinction that plausibly matters for driver trustworthiness signals. It's also inconsistent with `identity_reputation_contract`'s own approach elsewhere in the protocol, where reputation changes are recorded as adjustments (`decrease_reputation`) rather than erasing prior history.

**Proposed Solution:** Add a `Removed` (or `Left`) terminal variant to `DriverFleetStatus`, and have `remove_driver_from_fleet` transition to it rather than deleting the storage entry, preserving the historical record while still correctly excluding removed drivers from `total_active_drivers` and `get_payout_address`'s active-driver branch.

**Acceptance Criteria:**
- [ ] `DriverFleetStatus` has a terminal state distinguishing "removed" from "never invited."
- [ ] `get_driver_fleet_status` returns the terminal state (not `None`) for a previously-removed driver.
- [ ] `get_payout_address` and `total_active_drivers` accounting are unaffected by this change (removed drivers still correctly route to their own address and don't count as active).
- [ ] `test_roster_re_invite_after_removal` (an existing test in `fleet_management_contract/test.rs:492`) continues to pass, confirming a removed driver can still be re-invited.

**Technical Notes:** This changes what `add_driver_to_fleet`'s "guard: do not overwrite an existing invite" check (`lib.rs:216-226`) needs to handle, since a `Removed` status would now be a third case alongside `Pending`/`Active` that should permit a fresh invite rather than blocking it — verify this doesn't regress the existing re-invite test.

**Relevant Files:** `contracts/fleet_management_contract/lib.rs:26-33, 200-239, 300-338, 366-376`

**Testing Requirements:** New test asserting `get_driver_fleet_status` distinguishes a removed driver from a never-invited one; regression pass on all existing fleet roster tests (`fleet_management_contract/test.rs:151-597`).

**Definition of Done:** Fleet membership history survives driver removal, without breaking active-driver accounting or the ability to re-invite.

**Suggested Labels:** `enhancement`

---

### 78. No integration test scaffolding between `fleet_management_contract` and `escrow_contract`

**Summary:** `fleet_management_contract`'s `Cargo.toml` has no dev-dependency on `escrow_contract` (or any other contract), meaning there is zero integration-test coverage anywhere for how fleet-based payout routing (`get_payout_address`) is meant to interact with the actual escrow release flow — including no scaffolding in place for the GitHub #12 fix (fleet treasury routing never wired into the payout path) to be tested once implemented.

**Background:** `contracts/fleet_management_contract/Cargo.toml`'s `[dev-dependencies]` section lists only `soroban-sdk` — no other contract crate. Compare with `dispute_resolution_contract/Cargo.toml`, which already lists both `delivery_contract` and `escrow_contract` as dev-dependencies specifically to support its own cross-contract integration tests.

**Problem Statement:** `fleet_management_contract::get_payout_address` exists specifically to answer "where should escrow route this driver's payout," but nothing in the test suite actually exercises that question end-to-end against a real `escrow_contract`. Every one of `fleet_management_contract/test.rs`'s existing `get_payout_address` tests (`:512-597`) tests the function in isolation, asserting only the address it *returns* — never that `escrow_contract` actually *uses* that address when paying out. This is precisely the integration surface GitHub #12 already flags as broken, and there is currently no test infrastructure in place to verify a fix for it once landed.

**Proposed Solution:** Add `escrow_contract` (and `identity_reputation_contract`, needed for GitHub #39's fix) as dev-dependencies of `fleet_management_contract`, and write at least one true end-to-end test: register a fleet, add and activate a driver, create and release an escrow for that driver, and assert the funds actually land in the fleet's treasury (once GitHub #12 is fixed) rather than the driver's own address.

**Acceptance Criteria:**
- [ ] `fleet_management_contract/Cargo.toml` declares `escrow_contract` as a dev-dependency.
- [ ] An end-to-end test exists asserting escrow payouts for an active fleet driver actually route through `get_payout_address`'s answer.

**Technical Notes:** This test will necessarily fail (or need to be written as an expected-failure/ignored test) until GitHub #12 itself is fixed — landing the test scaffolding now, even red, makes the eventual fix immediately verifiable rather than requiring test infrastructure to be built at the same time as the fix itself.

**Relevant Files:** `contracts/fleet_management_contract/Cargo.toml`, `contracts/fleet_management_contract/test.rs:512-597`, `contracts/escrow_contract/lib.rs`

**Testing Requirements:** As described in Proposed Solution.

**Definition of Done:** Test infrastructure exists to verify, end-to-end, that escrow payouts respect fleet treasury routing — independent of whether the underlying GitHub #12 bug is fixed yet.

**Suggested Labels:** `test`

---

### 79. Core wire-format types in `shared_types` have no dedicated equality/round-trip tests

**Summary:** `shared_types/lib.rs`'s test module covers `DeliveryId`, status enum variants, `PartyAddresses`, `StorageKey` helpers, `FaniLabError` discriminants, the seven event structs, and `CargoDescriptor`/`DeliveryMetadata` — but has no test at all for `ProtocolConfig`, `DeliveryRecord`, or `EscrowRecord`, the three structs that actually cross contract boundaries most frequently and carry the most business-critical state.

**Background:** `contracts/shared_types/lib.rs:290-538` is the existing test module. A direct read confirms tests exist for every type listed above, but grepping the test module for `ProtocolConfig`, `DeliveryRecord`, and `EscrowRecord` by name returns no matches — these three types are only ever exercised indirectly, through the individual contracts' own integration-style tests (e.g., `escrow_contract/test.rs` checks individual `EscrowRecord` fields after various operations, but there's no dedicated `shared_types`-level test confirming the struct's own field equality/clone/debug behavior in isolation).

**Problem Statement:** `ProtocolConfig`, `DeliveryRecord`, and `EscrowRecord` are precisely the three types most exposed to cross-contract calls and most likely to need a careful field-by-field migration if their shape ever changes (as several other issues in this backlog would require — e.g., issue #71's governance unification touches `ProtocolConfig`-adjacent config). Having no dedicated unit test for these types at the `shared_types` level means a change to field ordering, a missed field in a `Clone`/`PartialEq` derive, or an accidental field rename would only be caught (if at all) by whichever downstream contract test happens to touch the affected field — an indirect, incomplete safety net compared to the direct coverage every other type in this file already has.

**Proposed Solution:** Add direct unit tests for `ProtocolConfig`, `DeliveryRecord`, and `EscrowRecord` in `shared_types/lib.rs`'s test module, following the exact pattern already used for `PartyAddresses`/the event structs (construct, assert every field).

**Acceptance Criteria:**
- [ ] `ProtocolConfig`, `DeliveryRecord`, and `EscrowRecord` each have a dedicated field-equality test in `shared_types/lib.rs`.

**Technical Notes:** This is a small, mechanical, low-risk addition — the goal is closing an obvious coverage gap in an otherwise well-tested file, not designing new test infrastructure.

**Relevant Files:** `contracts/shared_types/lib.rs:206-280, 290-538`

**Testing Requirements:** As described above.

**Definition of Done:** Every non-trivial public struct in `shared_types` that isn't already flagged as dead code (GitHub #42) has direct field-level test coverage at the `shared_types` level.

**Suggested Labels:** `test`

---

### 80. `CHANGELOG.md`'s `[Unreleased]` section is stale relative to the completed SDK 27 migration

**Summary:** `CHANGELOG.md`'s `[Unreleased] > Changed` section reads "Updated Soroban SDK to 22.0.1" — but the workspace has since fully migrated to SDK 27.0.0 (a separate, dedicated migration document exists for it), and the `[Unreleased]` section was never updated or cut into a new version to reflect that, the CI toolchain pin, or the `wasm32v1-none` target change.

**Background:** `CHANGELOG.md:8-38` is the `[Unreleased]` section, whose "Changed" subsection (`:28-32`) still lists "Updated Soroban SDK to 22.0.1" as the current changelog entry for the SDK version. Meanwhile, `SOROBAN_SDK_27_MIGRATION.md` documents a completed migration to SDK 27.0.0 dated 2026-07-14, and recent commit history (`6944bd4 Migrate to Soroban SDK 27.0.0 with wasm32v1-none target`, `38f7c38 ci: update Rust toolchain to stable`, `1e113ea fix: pin Rust to 1.81.0 in CI workflows for compatibility`) shows substantial toolchain work has landed since that changelog entry was written, none of which is reflected anywhere in `CHANGELOG.md`.

**Problem Statement:** `CHANGELOG.md`'s own documented "Release Process" (`:63-69`) describes `[Unreleased]` as the staging area that should be updated with changes and then cut into a versioned release — but real, substantial changes (an entire major SDK migration, a WASM target change, multiple CI toolchain pins) have landed without ever touching this file, leaving it actively wrong about the current SDK version rather than merely incomplete. This is the same underlying fact as GitHub #65 (`docs/API.md`'s stale SDK version), but a distinct process failure: a changelog is supposed to be updated as part of every notable change's own workflow, not corrected after the fact alongside unrelated documentation.

**Proposed Solution:** Update `CHANGELOG.md`'s `[Unreleased]` section to accurately reflect the SDK 27.0.0 migration, the `wasm32v1-none` target change, and the Rust toolchain pinning work, and consider whether this backlog of accumulated `[Unreleased]` changes should finally be cut into a real version per the document's own documented release process.

**Acceptance Criteria:**
- [ ] `CHANGELOG.md` accurately reflects the current SDK version and the toolchain/target changes that have landed since the `[Unreleased]` section was last touched.

**Technical Notes:** Fix this alongside GitHub #65 so both stale "22.0.1" references are corrected in the same pass, but keep them as separately tracked issues since they represent different underlying failures (a stray reference doc line vs. a changelog process gap).

**Relevant Files:** `CHANGELOG.md:8-38`, `SOROBAN_SDK_27_MIGRATION.md`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** `CHANGELOG.md`'s `[Unreleased]` section accurately reflects every notable change that has landed since it was last updated.

**Suggested Labels:** `documentation`

---

## Summary by Label

| Label | Count | Issues |
|---|---|---|
| `documentation` | 3 | 72, 73, 80 |
| `feature` | 2 | 74, 75 |
| `refactor` | 2 | 71, 76 |
| `test` | 2 | 78, 79 |
| `enhancement` | 1 | 77 |

(Several issues carry two labels — e.g. `enhancement`+`security` or `feature`+`security` — and are counted in both rows above. Issues #11–#70 were filed to GitHub across two review passes — see `Pushed to GitHub` above — and no longer appear here.)

## Summary by Contract

| Contract / Area | Issues in this doc | Filed on GitHub |
|---|---|---|
| `escrow_contract` | 76 | #7, #11, #12, #13, #14, #15, #16, #17, #18, #25, #26, #31, #37, #38, #54, #55, #61, #68, #73, #74 |
| `delivery_contract` | — | #19, #20, #23, #24, #27, #33, #44, #45, #68 |
| `dispute_resolution_contract` | 71 | #8, #21, #22, #32, #40, #49, #50, #51, #76 |
| `identity_reputation_contract` | — | #9, #10, #24, #41, #43, #44, #48, #75 |
| `fleet_management_contract` | 77, 78 | #12, #26, #27, #28, #39, #69, #70, #71 |
| `settlement_contract` | — | #15, #30, #35, #55 |
| `shared_types` | 79 | #24, #26, #33, #41, #42, #46, #47 |
| Docs (`docs/`, root `*.md`) | 72, 73, 75, 80 | #64, #65, #66, #67 |
| CI/CD (`.github/workflows/`) | — | #56, #57, #62, #63 |
| Scripts/tooling (`scripts/`, `Makefile`, `.env.example`) | — | #56, #57, #58, #59, #60 |
| Cross-cutting / process | 71, 74 | #7, #8, #19, #27, #31, #34, #36, #46, #47, #52, #53 |

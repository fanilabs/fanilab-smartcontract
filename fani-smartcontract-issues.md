# FaniLab Smart Contracts — Substantive Issues

Derived from a direct read of every contract in `contracts/` (escrow_contract,
delivery_contract, dispute_resolution_contract, fleet_management_contract,
identity_reputation_contract, settlement_contract, shared_types) plus the
project's own `PLAN.md`, `PRODUCTION_READINESS.md`, `Cargo.toml`, and CI
workflow. Every issue below references the specific function and file it was
found in — none are generic placeholders.

This document consolidates two review passes: an initial pass (all 30 of whose issues — 6 Critical, 4 High, and local issues #11–#30 — have since been filed to GitHub, see below) and a follow-up pass extending coverage to cross-contract architecture, testing gaps, CI/CD, deployment tooling, and documentation accuracy (issues #31–#80, still tracked in this document).

## Pushed to GitHub

30 issues have been filed on `github.com/fanilabs/fanilab-smartcontract` and removed from this document to avoid duplication: the original 10 highest-severity findings (6 Critical + 4 High), plus the full remaining High/Medium/Low-classified backlog from the initial review pass (local issues #11–#30). Track them there:

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

The remaining 50 issues below (#31–#80) are not yet filed.

---

## Index

| # | Title | Labels |
|---|---|---|
| 31 | `escrow_contract::get_status` is a dead stub that always returns `Pending` | bug |
| 32 | `create_escrow` accepts any token address, making `get_token()` misleading | bug, security |
| 33 | `register_fleet` permanently fails for any owner already registered as a driver | bug |
| 34 | `dispute_resolution_contract::remove_admin` can remove the last admin, bricking governance | bug, security |
| 35 | Two divergent `UserProfile` definitions with different field names | bug |
| 36 | `DeliveryDetails` and `PartyAddresses` are fully-defined dead types | refactor |
| 37 | `AuthorizedContract` allowlist is built but never consulted | security, bug |
| 38 | Driver tier system is never wired into `assign_driver` despite being documented | feature |
| 39 | `DeliveryMetadata.delivery_id` is never validated against the real `DeliveryId` | bug |
| 40 | `FaniLabError::DeliveryNotFound` and `EscrowError::DeliveryNotFound` carry different discriminants | refactor |
| 41 | Typed event payload structs and topic constants in `shared_types::events` are unused | refactor |
| 42 | No reputation decay for inactive drivers despite being a named roadmap item | feature |
| 43 | `add_evidence_hash` allows unbounded growth of a single storage entry | security, performance |
| 44 | No automated dispute evidence verification system despite being a named roadmap item | feature |
| 45 | Dispute resolution's reputation-penalty cross-call is never exercised by any test | test |
| 46 | No `proptest` dependency anywhere despite extensive property-testing documentation | test |
| 47 | No fuzz targets despite `SECURITY_AUDIT.md` prescribing `cargo fuzz` commands | test |
| 48 | Two-step admin transfer (`propose_admin`/`accept_admin`) has zero test coverage | test |
| 49 | `settlement_contract` test suite only exercises `init` | test |
| 50 | `deploy-all-contracts.sh` still builds with the pre-migration `wasm32-unknown-unknown` target | bug |
| 51 | `Makefile` targets still use `wasm32-unknown-unknown` and cover only 3 of 6 contracts | bug |
| 52 | `initialize-all-contracts.sh` only initializes 2 of the 6 deployed contracts | bug |
| 53 | Deploy script's error handling after `cargo build` is unreachable dead code | bug |
| 54 | `.env.example` doesn't match the variables `DEPLOYMENT.md` and the scripts actually need | bug |
| 55 | Release workflow's "Optimize WASM" step performs no optimization | performance |
| 56 | CI reinstalls `cargo-audit`/`cargo-outdated`/`cargo-tarpaulin`/`cargo-deny` from source on every run | performance |
| 57 | No CI job enforces `--locked`, despite repeated manual `Cargo.lock` pinning fire-drills | enhancement |
| 58 | `docs/API.md`'s table of contents promises 4 contracts it never documents | documentation |
| 59 | `docs/API.md` footer claims Soroban SDK 22.0.1, three versions behind actual | documentation |
| 60 | Three architecture/design docs are committed as completely empty files | documentation |
| 61 | Docs disagree with themselves on whether the protocol has 6 or 7 contracts | documentation |
| 62 | `escrow_contract` and `delivery_contract` each hand-roll an identical private `is_admin` helper | refactor |
| 63 | No admin override/recovery path in `fleet_management_contract` for a compromised owner key | enhancement, security |
| 64 | `update_fleet_treasury` has no timelock, cooldown, or driver notice | enhancement, security |
| 65 | No multi-signature support for fleet management despite being a named roadmap item | feature |
| 66 | No dynamic, volume-based fee adjustment despite being a named roadmap item | feature |
| 67 | No recovery mechanism for tokens sent directly to `escrow_contract` outside `create_escrow` | feature, security |
| 68 | No on-chain aggregate TVL view despite `MONITORING.md` naming it a key metric | feature |
| 69 | `register_user`/`UserProfile` are fully implemented but never consumed anywhere | enhancement |
| 70 | No way to enumerate current admins of `dispute_resolution_contract` | feature |
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

## Additional Findings — Architecture, Testing, Documentation & Tooling (Issues #31–#80)

A follow-up review pass covering cross-contract architecture, admin/governance models, test-coverage gaps, CI/CD and deployment tooling, and documentation accuracy — building on the initial pass now filed as GitHub #7–#16 and #17–#36, without duplicating any of those findings.

---

### 31. `escrow_contract::get_status` is a dead stub that always returns `Pending`

**Summary:** `EscrowContract::get_status` ignores its `Env` argument entirely and unconditionally returns `DeliveryStatus::Pending`, making it either dead code or a landmine for any caller who expects it to reflect real state.

**Background:** `contracts/escrow_contract/lib.rs:214-216` defines:
```rust
pub fn get_status(_env: Env) -> DeliveryStatus {
    DeliveryStatus::Pending
}
```
It takes no `delivery_id`, reads no storage, and returns a `DeliveryStatus` value — a type that conceptually belongs to `delivery_contract`, not `escrow_contract` (which owns `EscrowStatus`/`EscrowState`). No test file references it, and no other contract calls it.

**Problem Statement:** Any off-chain integration or future contract code that calls `escrow_contract.get_status()` expecting a meaningful answer will silently always get `Pending`, regardless of the escrow's actual `Locked`/`Released`/`Refunded`/`Paused` state. Because it compiles and is part of the public contract interface, it looks like a supported query.

**Proposed Solution:** Remove the function if it is confirmed dead, or, if it was meant to expose escrow state, replace it with a parameterized function that returns the actual `EscrowStatus` for a given `delivery_id` (duplicating what `get_escrow(delivery_id).status` already provides, so removal is the more likely correct fix).

**Acceptance Criteria:**
- [ ] `get_status` is either removed from the public contract interface or reimplemented to take a `delivery_id` and return the real, current `EscrowStatus`.
- [ ] No caller in the codebase silently relies on the current hardcoded behavior.

**Technical Notes:** Removing a public contract function changes the contract's ABI; if any deployed client already calls it, coordinate the removal with a deprecation note in `CHANGELOG.md`.

**Relevant Files:** `contracts/escrow_contract/lib.rs:214-216`

**Testing Requirements:** If kept, add a test that creates an escrow, transitions it through at least two states, and asserts `get_status`/its replacement reflects each transition. If removed, confirm `cargo build` and `cargo test` still pass with no dangling references.

**Definition of Done:** The function either accurately reflects escrow state or no longer exists; `cargo clippy` and the full test suite pass.

**Suggested Labels:** `bug`

---

### 32. `create_escrow` accepts any token address, making `get_token()` misleading

**Summary:** `create_escrow` takes a caller-supplied `token: Address` with no check against the protocol's configured `ProtocolConfig.token`, so escrows can be funded in a completely different asset than the one `get_token()` reports.

**Background:** `contracts/escrow_contract/lib.rs:293-330` accepts `token: Address` as a parameter and stores it directly on the `EscrowRecord`, independent of `ProtocolConfig.token` set at `init` (`lib.rs:159-182`). `get_token()` (`lib.rs:225-227`) only ever returns the `ProtocolConfig` value, not the per-escrow token.

**Problem Statement:** A sender can call `create_escrow` with an arbitrary token contract (including a worthless or malicious one) regardless of what the protocol claims to support. Meanwhile, any off-chain system, dashboard, or `docs/MONITORING.md`-style TVL calculator that trusts `get_token()` as "the asset used by this protocol" will misreport or mis-aggregate value for escrows funded in other tokens. There is also no admin allowlist of acceptable tokens.

**Proposed Solution:** Either (a) enforce `token == load_protocol_config(&env).token` in `create_escrow` and drop the redundant parameter, or (b) if multi-token support is intentional, add an admin-managed token allowlist and change `get_token()`/monitoring guidance to be explicit that it returns only the *default* token, not an exhaustive list of tokens in use.

**Acceptance Criteria:**
- [ ] `create_escrow`'s accepted tokens are either restricted to the configured token or explicitly governed by an allowlist.
- [ ] `get_token()`'s documented meaning matches its actual guarantee.
- [ ] A test asserts that an unsupported/unlisted token is rejected (if allowlist approach) or that the `token` argument is validated against config (if single-token approach).

**Technical Notes:** If an allowlist is chosen, consider a `DataKey::AllowedToken(Address) -> bool` pattern consistent with `identity_reputation_contract`'s existing `AuthorizedContract(Address)` pattern (see issue #37).

**Relevant Files:** `contracts/escrow_contract/lib.rs:159-182, 225-227, 293-330`

**Testing Requirements:** Test that `create_escrow` with a token other than the configured/allowed one is rejected with a typed error (or, if intentionally multi-token, that `get_token()`'s docs and behavior are reconciled and a new `get_escrow(id).token` is the documented way to discover per-escrow assets).

**Definition of Done:** Token acceptance policy is explicit, enforced, and documented; `get_token()` cannot mislead a caller about what asset a given escrow actually holds.

**Suggested Labels:** `bug`, `security`

---

### 33. `register_fleet` permanently fails for any owner already registered as a driver

**Summary:** `register_fleet`'s cross-call into `identity_reputation_contract::register_driver` is unconditional; if the fleet owner already has a driver profile, `register_driver` panics with `AlreadyInitialized`, which unwinds and rolls back the *entire* `register_fleet` call — including the fleet counter increment and profile write.

**Background:** `contracts/fleet_management_contract/lib.rs:135-145` calls `register_driver` on the configured identity contract whenever one is set, with no existence check first:
```rust
if let Some(identity_addr) = env.storage().instance().get::<DataKey, Address>(&DataKey::IdentityContract) {
    let _: () = env.invoke_contract(&identity_addr, &Symbol::new(&env, "register_driver"), ...);
}
```
`identity_reputation_contract::register_driver` (`lib.rs:108-113`) panics with `FaniLabError::AlreadyInitialized` if a `DriverProfile` already exists for that address. Soroban propagates a panic in a cross-contract call as a failure of the entire top-level transaction.

**Problem Statement:** Once `set_identity_contract` is configured (a normal, expected admin action per the contract's own doc comments), any address that is already a registered driver — or that registers a *second* fleet after its first successful `register_fleet` call already auto-registered it as a driver — can never successfully call `register_fleet` again. There is no way to work around this without unsetting the identity contract entirely, which defeats the feature for everyone.

**Proposed Solution:** Before calling `register_driver`, check whether the owner already has a driver profile (e.g., expose a `has_driver_profile`/`try`-style read on `identity_reputation_contract`, or catch the specific error) and skip the cross-call if one exists, rather than letting the panic propagate.

**Acceptance Criteria:**
- [ ] Registering a second fleet under the same owner succeeds when an identity contract is configured.
- [ ] Registering a fleet for an owner who is already a registered driver succeeds.
- [ ] A regression test covers both scenarios end-to-end (identity contract wired, owner registers fleet twice / owner pre-registered as driver).

**Technical Notes:** Since Soroban contracts can't easily catch cross-contract panics today, the cleanest fix is a query-first pattern: read-only "does this driver profile exist" check before the mutating call, keeping `identity_reputation_contract`'s existing invariant (`register_driver` still panics for direct callers) intact.

**Relevant Files:** `contracts/fleet_management_contract/lib.rs:105-154`, `contracts/identity_reputation_contract/lib.rs:108-128`

**Testing Requirements:** Add a `fleet_management_contract` test (with `identity_reputation_contract` as a dev-dependency, see also issue #78) that: sets an identity contract, registers fleet A for owner X, then registers fleet B for the same owner X, and asserts both succeed.

**Definition of Done:** `register_fleet` is idempotent with respect to driver-profile auto-registration; multi-fleet owners are not blocked.

**Suggested Labels:** `bug`

---

### 34. `dispute_resolution_contract::remove_admin` can remove the last admin, bricking governance

**Summary:** `remove_admin` lets any current admin remove any other admin — including themself if they are the sole remaining admin — with no floor on admin count, permanently locking every admin-gated function in the contract.

**Background:** `contracts/dispute_resolution_contract/lib.rs:79-85`:
```rust
pub fn remove_admin(env: Env, caller: Address, old_admin: Address) {
    caller.require_auth();
    if !Self::is_admin(env.clone(), caller.clone()) { panic_with_error!(&env, FaniLabError::Unauthorized); }
    env.storage().instance().remove(&DataKey::Admin(old_admin));
}
```
Unlike `escrow_contract`'s single-`Admin`-address model (which always has exactly one admin who can propose a successor), this contract models admins as an open-ended `DataKey::Admin(Address) -> bool` set with no minimum-count invariant, and `add_admin` (`lib.rs:69-77`) itself requires being an existing admin to add a new one.

**Problem Statement:** If `old_admin == caller` and `caller` is the only address with `is_admin() == true`, this call succeeds and leaves the contract with zero admins. Every admin-gated function afterward (`add_admin`, `resolve_dispute_refund_sender`, `resolve_dispute_split_funds`, `resolve_dispute_pay_driver`, `set_identity_reputation_contract`) becomes permanently uncallable — there is no owner-of-last-resort, no timelock, and no redeploy-free recovery path. This can happen accidentally (an admin cleaning up their own access) or as a griefing vector if a single admin key is ever briefly compromised and the attacker's only goal is disruption rather than theft.

**Proposed Solution:** Track an admin count (or iterate is unnecessary — a simple counter suffices) and reject `remove_admin` when it would bring the count to zero. Alternatively, require at least one other function (e.g., a separate "retire contract" ADR-level action) to explicitly acknowledge zero-admin state before allowing it.

**Acceptance Criteria:**
- [ ] `remove_admin` rejects any call that would leave the contract with zero admins, with a dedicated typed error.
- [ ] A test asserts that removing the sole admin fails, and that removing one of two-or-more admins still succeeds.

**Technical Notes:** A `DataKey::AdminCount(u32)` instance value, incremented in `add_admin` / decremented in `remove_admin`, is the simplest implementation and also directly enables issue #70 (admin enumeration).

**Relevant Files:** `contracts/dispute_resolution_contract/lib.rs:69-92`

**Testing Requirements:** Unit test: single admin calls `remove_admin(self, self)` → must fail. Unit test: two admins, one removes the other → must succeed and leave exactly one admin.

**Definition of Done:** It is structurally impossible to leave `dispute_resolution_contract` with zero admins via any public entry point.

**Suggested Labels:** `bug`, `security`

---

### 35. Two divergent `UserProfile` definitions with different field names

**Summary:** `shared_types::UserProfile` and `identity_reputation_contract::UserProfile` are two independently-declared, structurally-different types with the same name, mirroring the already-tracked `DriverProfile` duplication ([GitHub #9](https://github.com/fanilabs/fanilab-smartcontract/issues/9)-adjacent context) but for a different struct entirely.

**Background:** `contracts/shared_types/lib.rs:551-555`:
```rust
pub struct UserProfile {
    pub address: Address,
    pub registered_at: u64,
}
```
`contracts/identity_reputation_contract/lib.rs:9-12`:
```rust
pub struct UserProfile {
    pub address: Address,
    pub join_date: u64,
}
```
The two structs are not interchangeable (different field name, `registered_at` vs `join_date`), and `identity_reputation_contract::register_user`/`get_user_profile` (`lib.rs:130-162`) exclusively use the *local* definition, never `shared_types::UserProfile`.

**Problem Statement:** Off-chain SDKs or other contracts that build against `shared_types::UserProfile` (the type one would expect to be canonical, given `shared_types`' stated purpose per ADR-003) will get field-name mismatches against what `identity_reputation_contract` actually returns. This is the same class of "no single source of truth" problem already flagged for `DriverProfile` in GitHub #24, but affects an entirely separate type that issue does not mention.

**Proposed Solution:** Delete one of the two `UserProfile` definitions and have `identity_reputation_contract` use `shared_types::UserProfile` exclusively (consistent with the direction already recommended for `DriverProfile`).

**Acceptance Criteria:**
- [ ] Only one `UserProfile` type exists in the workspace.
- [ ] `identity_reputation_contract::register_user`/`get_user_profile` use the shared type.
- [ ] Existing tests referencing `join_date` are updated to the shared field name.

**Technical Notes:** This is a breaking storage-schema change if any `UserProfile` records already exist on a deployed network; treat it like the `DriverProfile` consolidation recommended in GitHub #24 and land both in the same migration window.

**Relevant Files:** `contracts/shared_types/lib.rs:551-555`, `contracts/identity_reputation_contract/lib.rs:9-12, 130-162`

**Testing Requirements:** Update `contracts/identity_reputation_contract/test.rs` (currently has zero tests for `register_user`/`get_user_profile` at all — see also issue #69) to use the consolidated type and verify field values round-trip correctly.

**Definition of Done:** A single canonical `UserProfile` type is used protocol-wide; `cargo test` passes.

**Suggested Labels:** `bug`

---

### 36. `DeliveryDetails` and `PartyAddresses` are fully-defined dead types

**Summary:** `shared_types` declares two fully-documented public structs, `DeliveryDetails` and `PartyAddresses`, that are never constructed or consumed by any contract — only by `shared_types`' own unit tests.

**Background:** `contracts/shared_types/lib.rs:230-235` (`PartyAddresses { sender, driver, recipient }`) and `lib.rs:282-288` (`DeliveryDetails { id, driver: String, status: DeliveryStatus }`) are both public, `#[contracttype]`-annotated structs. A workspace-wide search shows their only usages are in `shared_types/lib.rs`'s own `#[cfg(test)]` module (`party_addresses_preserve_fields` test); no contract in `contracts/escrow_contract`, `delivery_contract`, `dispute_resolution_contract`, `fleet_management_contract`, `identity_reputation_contract`, or `settlement_contract` references either type.

**Problem Statement:** `DeliveryDetails` even encodes `driver` as a `String` rather than an `Address`, which is inconsistent with every other place a driver is represented in the codebase (`Address` everywhere else) — a strong signal this is either an abandoned early draft of `DeliveryRecord` or a placeholder that was never wired up. Dead public types in a shared library inflate the WASM-adjacent surface area contracts must be audited against and confuse anyone trying to understand the "real" cross-contract data model.

**Proposed Solution:** Remove both types if confirmed unused, or, if `DeliveryDetails` was meant as a lightweight summary view (id + driver + status) for a future pagination/enumeration API (see GitHub #27), fix its `driver` field to `Address` and actually wire it into a query function.

**Acceptance Criteria:**
- [ ] `DeliveryDetails` and `PartyAddresses` are either removed or given at least one real caller.
- [ ] If removed, their unit tests in `shared_types/lib.rs`'s test module are removed too.

**Technical Notes:** Grep confirms zero non-test references: `grep -rn "DeliveryDetails\|PartyAddresses" contracts/ | grep -v shared_types/lib.rs` returns nothing.

**Relevant Files:** `contracts/shared_types/lib.rs:230-235, 282-288, 323-338`

**Testing Requirements:** After removal, `cargo build --workspace` and `cargo test --workspace` must still pass with no dangling references.

**Definition of Done:** No public type in `shared_types` is unreferenced outside its own test module.

**Suggested Labels:** `refactor`

---

### 37. `AuthorizedContract` allowlist is built but never consulted

**Summary:** `identity_reputation_contract` implements a complete admin-gated allowlist (`set_authorized_contract`/`is_authorized_contract`) for which contracts may call it, but not a single mutating function in the contract actually checks it.

**Background:** `contracts/identity_reputation_contract/lib.rs:84-106` implements:
```rust
pub fn set_authorized_contract(env: Env, admin: Address, contract_addr: Address, authorized: bool) { ... }
pub fn is_authorized_contract(env: Env, contract_addr: Address) -> bool { ... }
```
Instead, `register_driver`, `increase_reputation`, `decrease_reputation`, and `update_driver_kyc_status` each independently check `caller == delivery_contract || caller == dispute_contract` against the two fixed addresses set once in `initialize` (`lib.rs:224-227, 257-270`). The `AuthorizedContract` storage key and its setter/getter are entirely orthogonal to that check.

**Problem Statement:** This is a genuine access-control primitive that looks load-bearing (it's admin-gated, has a dedicated storage key, and a public getter) but provides zero actual protection — an admin can call `set_authorized_contract(addr, true)` believing they've granted `addr` write access to reputation data, and nothing changes, because no code path reads `is_authorized_contract`. Conversely, the two hardcoded addresses from `initialize` can never be revoked or rotated without a full contract migration, since nothing consults the allowlist that exists specifically to make that flexible.

**Proposed Solution:** Either (a) delete the unused allowlist functions and storage key as dead code, or (b) replace the hardcoded `delivery_contract`/`dispute_contract` equality checks in `increase_reputation`/`decrease_reputation`/`register_driver`/`update_driver_kyc_status` with `is_authorized_contract(caller)`, and have `initialize` call `set_authorized_contract` for the initial two contracts — making the allowlist the actual, sole source of truth and enabling future contracts to be authorized without a redeploy.

**Acceptance Criteria:**
- [ ] Either the allowlist is removed, or all caller-authorization checks in the contract route through `is_authorized_contract`.
- [ ] If wired up, a test confirms a newly-authorized contract can call `increase_reputation`/`decrease_reputation` and a de-authorized one cannot.

**Technical Notes:** Option (b) is strictly more valuable since it also fixes the "hardcoded, unrotatable authorized callers" limitation implicit in the current design, and costs little given the storage plumbing already exists.

**Relevant Files:** `contracts/identity_reputation_contract/lib.rs:84-106, 108-128, 174-203, 205-254, 256-289`

**Testing Requirements:** Add tests for: an authorized third contract successfully calling `increase_reputation`; a de-authorized former caller being rejected; `set_authorized_contract`/`is_authorized_contract` themselves (currently have zero test coverage — see issue #69 for the broader identity-contract coverage gap).

**Definition of Done:** The `AuthorizedContract` mechanism is either gone or is the single enforced authorization path for reputation-mutating calls.

**Suggested Labels:** `security`, `bug`

---

### 38. Driver tier system is never wired into `assign_driver` despite being documented

**Summary:** `identity_reputation_contract::get_driver_tier`/`is_eligible_for_enterprise` exist, are tested in isolation, and are described in `docs/architecture/smart-contract-architecture.md` as governing job eligibility — but `delivery_contract::assign_driver` never calls either, so tier has zero effect on which deliveries a driver can take.

**Background:** `docs/architecture/smart-contract-architecture.md:16` states `delivery_contract` "**Interacts with**: `identity_reputation_contract` (to verify driver tier)". In reality, `contracts/delivery_contract/lib.rs:166-196` (`assign_driver`) only checks `is_admin` or self-assignment (`caller == driver`) — it never invokes `identity_reputation_contract` at all. `get_driver_tier` and `is_eligible_for_enterprise` (`identity_reputation_contract/lib.rs:291-306`) are exercised only by that contract's own unit tests (`contracts/identity_reputation_contract/test.rs:143-191`); a workspace-wide search confirms no other contract calls them.

**Problem Statement:** The entire tiering/enterprise-eligibility feature — the mechanism the architecture doc positions as the reason `identity_reputation_contract` and `delivery_contract` are coupled — has no effect on the actual delivery-assignment flow. Any Bronze-tier driver can be assigned to (and self-assign for) any delivery, including whatever "high-paying enterprise jobs" `PLAN.md`'s reputation description alludes to. This is a documented, load-bearing cross-contract relationship that doesn't exist in code.

**Proposed Solution:** Add an optional tier/eligibility gate to `assign_driver` (e.g., a `min_tier: Option<DriverTier>` on delivery metadata, or a simpler enterprise-flag check via `is_eligible_for_enterprise`) that cross-calls `identity_reputation_contract` before allowing assignment, or — if tiering isn't ready for this release — update the architecture doc to describe it as planned rather than implemented.

**Acceptance Criteria:**
- [ ] Either `assign_driver` enforces a tier/eligibility check via a real cross-contract call, or the architecture documentation is corrected to not claim this integration exists today.
- [ ] If implemented, a test demonstrates a Bronze-tier driver being rejected from an enterprise-flagged delivery and a Gold-tier driver being accepted.

**Technical Notes:** Wiring this requires `delivery_contract` to know the address of `identity_reputation_contract`, which it currently does not store anywhere (only `EscrowContract` address is stored in `DataKey::EscrowContract`) — a new instance storage key and admin setter (mirroring `fleet_management_contract::set_identity_contract`) would be needed.

**Relevant Files:** `contracts/delivery_contract/lib.rs:166-196`, `contracts/identity_reputation_contract/lib.rs:291-306`, `docs/architecture/smart-contract-architecture.md:13-16`

**Testing Requirements:** New test(s) in `delivery_contract/test.rs` covering tier-gated and non-gated assignment paths, plus a doc-accuracy check if the "won't implement now" path is chosen.

**Definition of Done:** Documentation and code agree on whether driver tier affects job assignment; if it does, the enforcement is tested.

**Suggested Labels:** `feature`

---

### 39. `DeliveryMetadata.delivery_id` is never validated against the real `DeliveryId`

**Summary:** The caller-supplied `DeliveryMetadata.delivery_id: u64` field is stored verbatim and never checked against the actual `DeliveryId` the contract assigns via its own counter, so a delivery's storage key and its embedded metadata can permanently disagree.

**Background:** `contracts/delivery_contract/lib.rs:78-120` (`create_delivery`) generates the real `delivery_id` from an internal counter (`DataKey::DeliveryCounter`), but the `metadata: DeliveryMetadata` parameter (defined in `contracts/shared_types/lib.rs:577-584`) carries its *own*, independent `delivery_id: u64` field supplied entirely by the caller. Nothing in `create_delivery` cross-checks `metadata.delivery_id` against the freshly-minted `delivery_id`.

**Problem Statement:** A sender can call `create_delivery` with `metadata.delivery_id` set to any arbitrary value — 0, a different delivery's ID, or a value that collides with an unrelated record — and the contract will happily store it under the *real* `DeliveryId` key while the metadata blob inside claims to belong to a different delivery entirely. Any off-chain consumer that trusts `metadata.delivery_id` as authoritative (rather than the outer `DeliveryRecord.delivery_id`) will misattribute cargo details to the wrong delivery.

**Proposed Solution:** Either derive `metadata.delivery_id` from the real counter inside `create_delivery` (overwriting whatever the caller passed) rather than trusting caller input, or drop the redundant field from `DeliveryMetadata` entirely since `DeliveryRecord.delivery_id` already serves that purpose.

**Acceptance Criteria:**
- [ ] `metadata.delivery_id` (if kept) always equals `DeliveryRecord.delivery_id` for every stored record, enforced by the contract rather than trusted from the caller.
- [ ] A test creates a delivery with a mismatched `metadata.delivery_id` and asserts the contract either rejects it or overwrites it to the correct value.

**Technical Notes:** Overwriting is the lower-risk fix since it requires no signature changes to `create_delivery`, whereas removing the field is a breaking change to `DeliveryMetadata`'s shape used across `shared_types`.

**Relevant Files:** `contracts/delivery_contract/lib.rs:78-120`, `contracts/shared_types/lib.rs:577-584`

**Testing Requirements:** Add a test asserting `get_delivery(id).metadata.delivery_id == id` always holds, regardless of what the caller passed in.

**Definition of Done:** It is impossible to create a delivery whose stored metadata disagrees with its own storage key about which delivery it describes.

**Suggested Labels:** `bug`

---

### 40. `FaniLabError::DeliveryNotFound` and `EscrowError::DeliveryNotFound` carry different discriminants

**Summary:** The protocol has two separately-defined "delivery not found" error variants with the same name but different numeric codes (`FaniLabError::DeliveryNotFound = 4` vs `EscrowError::DeliveryNotFound = 2`), undermining any attempt at a single canonical error-code table for off-chain clients.

**Background:** `contracts/shared_types/lib.rs:8-29` defines the shared `FaniLabError` enum used by `delivery_contract`, `dispute_resolution_contract`, `fleet_management_contract` (partially), and `identity_reputation_contract`. `contracts/escrow_contract/lib.rs:127-136` separately defines its own `EscrowError` enum, whose `DeliveryNotFound` variant is numbered `2`, not `4`. `docs/API.md:502-518` documents `FaniLabError` as *the* protocol error table with `DeliveryNotFound = 4`, but escrow errors returned to a client for the exact same conceptual failure ("no record for this ID") will show up as code `2`.

**Problem Statement:** An off-chain SDK or dashboard that builds a single "error code → human message" lookup table (exactly what `docs/API.md:539-544`'s "Error Handling Best Practices" recommends: "Parse error discriminant... Match against error enum values") cannot do so correctly across contracts, because the same semantic error means different numbers depending which contract raised it. This is a distinct problem from GitHub #24 (which is about the `DriverProfile` *data* type being duplicated, not error *codes*).

**Proposed Solution:** Either have `escrow_contract` reuse `shared_types::FaniLabError::DeliveryNotFound` directly instead of defining its own overlapping variant, or clearly document that each contract's error codes are contract-scoped (not globally unique) and off-chain clients must key error tables by `(contract, code)` rather than `code` alone.

**Acceptance Criteria:**
- [ ] Either `EscrowError` is reconciled with `FaniLabError` for overlapping concepts, or `docs/API.md` explicitly documents that error codes are per-contract, not global.
- [ ] The chosen approach is applied consistently to `FleetError`/`DeliveryError` too, which have the same overlap problem (e.g., `FleetError::Unauthorized = 3` vs `FaniLabError::Unauthorized = 1`).

**Technical Notes:** Full consolidation onto one enum may not be desirable (each contract's error space benefits from independent evolution), so the documentation-first fix is the pragmatic default; full consolidation is the higher-effort, more correct fix.

**Relevant Files:** `contracts/shared_types/lib.rs:8-29`, `contracts/escrow_contract/lib.rs:127-136`, `contracts/fleet_management_contract/lib.rs:13-24`, `docs/API.md:502-518`

**Testing Requirements:** If enums are consolidated, update all `try_*` test assertions across every `test.rs` that match on the affected variants.

**Definition of Done:** Off-chain consumers have an unambiguous, documented way to interpret error codes across every contract.

**Suggested Labels:** `refactor`

---

### 41. Typed event payload structs and topic constants in `shared_types::events` are unused

**Summary:** `shared_types` defines a complete typed-event system (`events::escrow_funded()`-style topic helpers plus `DeliveryCreatedEvent`, `EscrowFundedEvent`, `DriverAssignedEvent`, etc. payload structs) that ADR-007 presents as the protocol's event architecture, but almost every contract bypasses it and publishes raw tuples with inline, ad hoc `Symbol::new(&env, "...")` topics instead.

**Background:** `contracts/shared_types/lib.rs:32-153` defines seven topic-helper functions (`delivery_created`, `escrow_funded`, etc.) and seven matching payload structs. `contracts/escrow_contract/lib.rs` does use the topic helpers (`events::escrow_funded(&env)` etc., e.g. `lib.rs:326-329`) but publishes plain tuples `(sender, recipient, amount)` rather than the matching `EscrowFundedEvent` struct. `contracts/delivery_contract/lib.rs`, `contracts/fleet_management_contract/lib.rs`, and `contracts/dispute_resolution_contract/lib.rs` don't use the shared topic helpers *or* the payload structs at all — they inline their own `Symbol::new(&env, "fleet_registered")`, `Symbol::new(&env, "dispute_raised")`, `Symbol::new(&env, "driver_invited")`, etc. directly at each call site.

**Problem Statement:** ADR-007 ("Event-Driven Architecture") frames this as a deliberate, centralized design for off-chain indexing consistency, but in practice topic strings for most events exist only as scattered string literals with no central registry, and none of the seven documented payload struct shapes are ever actually put on the wire. An indexer built against the `shared_types::events` API (as `docs/API.md`'s "Events" section implies is possible) would get nothing usable for most event types, and a typo in one of the many inline `Symbol::new` calls (e.g., renaming `"fleet_registered"` in one place but not another) would silently break event consumers with no compiler check.

**Proposed Solution:** Either (a) fully commit to the shared event system — move every inline topic string into `shared_types::events` and have every `publish()` call use the matching typed struct — or (b) remove the unused payload structs and topic helpers that aren't being adopted, and document that each contract owns its own event shapes.

**Acceptance Criteria:**
- [ ] Every contract's `events().publish()` calls consistently either use `shared_types::events`/payload structs, or the shared module is trimmed to only what's actually used, with no silent partial adoption.
- [ ] `docs/architecture/event-system.md` (currently empty — see issue #60) documents whichever approach is chosen.

**Technical Notes:** Given `#[allow(deprecated)]` is already present everywhere for `events().publish()` (per `SOROBAN_SDK_27_MIGRATION.md`), this is a good opportunity to land the fix alongside a future migration to the SDK's `#[contractevent]` macro once it's stable.

**Relevant Files:** `contracts/shared_types/lib.rs:32-153`, `contracts/escrow_contract/lib.rs` (multiple `events().publish()` sites), `contracts/delivery_contract/lib.rs`, `contracts/fleet_management_contract/lib.rs`, `contracts/dispute_resolution_contract/lib.rs`

**Testing Requirements:** For any event call site changed to use a typed struct, update the corresponding test assertions (several tests already assert on emitted event tuples/topics and would need adjusting).

**Definition of Done:** The protocol has one, consistently-applied event convention, either fully centralized in `shared_types` or explicitly documented as per-contract.

**Suggested Labels:** `refactor`

---

### 42. No reputation decay for inactive drivers despite being a named roadmap item

**Summary:** `PLAN.md` explicitly lists "Create reputation decay mechanism for inactive drivers" as a Medium-High priority feature; no such mechanism exists anywhere in `identity_reputation_contract` or any other contract.

**Background:** `PLAN.md:19` lists the decay feature under "New Features." `contracts/identity_reputation_contract/lib.rs` has no time-based decay logic anywhere — `reputation_score` only ever changes via `increase_reputation`/`decrease_reputation`, both invoked exclusively as a direct result of a delivery event, never on a schedule or based on elapsed time since a driver's last activity.

**Problem Statement:** A driver who earns a high reputation score and then goes permanently inactive retains Gold-tier status (`get_driver_tier`, `≥75`) and enterprise eligibility (`is_eligible_for_enterprise`) indefinitely, with no mechanism to reflect that their reliability signal is stale. This undermines the reputation system's usefulness for its stated purpose (gating access to "high-paying enterprise jobs" per the architecture doc) since an inactive account looks identical to an actively-good one.

**Proposed Solution:** Add a `last_active_at: u64` field to `DriverProfile`, updated whenever `increase_reputation`/`decrease_reputation` fires, and a permissionless or admin-triggered `apply_decay(driver)` function that reduces `reputation_score` based on elapsed time since `last_active_at` (e.g., a fixed point decay per 30-day period beyond some grace window).

**Acceptance Criteria:**
- [ ] `DriverProfile` tracks last-activity time.
- [ ] A decay function exists, is callable (permissionless is simplest, since it can't be abused to *harm* the driver beyond a fair, time-based formula), and reduces `reputation_score` for drivers inactive beyond a defined threshold.
- [ ] Decay never reduces `reputation_score` below 0 (`saturating_sub`, consistent with `decrease_reputation`'s existing pattern).

**Technical Notes:** Because Soroban has no native cron/scheduled execution, decay must be applied lazily — e.g., computed on read in `get_driver_profile`/`get_driver_tier`, or via an explicit external trigger — rather than assumed to run automatically.

**Relevant Files:** `contracts/identity_reputation_contract/lib.rs:14-22, 205-289, 291-306`, `PLAN.md:19`

**Testing Requirements:** Tests for: no decay within the grace window; decay applied correctly after N periods of inactivity; decay resets/pauses correctly once a driver becomes active again; decay never underflows.

**Definition of Done:** Reputation scores measurably decline for provably inactive drivers, closing the gap between `PLAN.md`'s stated roadmap and the shipped contract.

**Suggested Labels:** `feature`

---

### 43. `add_evidence_hash` allows unbounded growth of a single storage entry

**Summary:** `dispute_resolution_contract::add_evidence_hash` lets either party to a dispute push an unlimited number of `BytesN<32>` hashes onto a single `Vec` stored under one persistent key, with no cap — a straightforward storage-growth griefing vector against a specific dispute record.

**Background:** `contracts/dispute_resolution_contract/lib.rs:205-245`:
```rust
dispute.evidence_hashes.push_back(evidence_hash.clone());
env.storage().persistent().set(&dispute_key, &dispute);
```
There is no limit on how many times `add_evidence_hash` can be called for a given `delivery_id` while `dispute.status == DisputeStatus::Open`, and no cap on `DisputeCase.evidence_hashes`'s length.

**Problem Statement:** `docs/API.md:550-555` documents a 64 KB max storage entry size for Soroban. Either the sender or recipient on a disputed delivery (both are authorized callers per the function's own check) can call `add_evidence_hash` in a loop until the `DisputeCase` entry approaches or exceeds that limit, at which point further writes to it fail and the dispute becomes unresolvable — exactly the "no unbounded data structures" / "no griefing attacks possible" failure mode that `docs/SECURITY_AUDIT.md`'s own Denial-of-Service checklist (section 10) calls out as something to verify. This is a self-inflicted DoS: a party unhappy with how a dispute is going could deliberately corrupt their own dispute record to stall resolution.

**Proposed Solution:** Add a maximum evidence-hash count (a small, sane constant, e.g. 20) enforced in `add_evidence_hash`, returning a typed error once exceeded.

**Acceptance Criteria:**
- [ ] `add_evidence_hash` rejects submissions once a defined cap is reached, with a dedicated error variant.
- [ ] A test adds evidence up to the cap successfully and confirms the next call is rejected.

**Technical Notes:** Consider whether large volumes of evidence belong on-chain at all versus storing only a Merkle root or IPFS/off-chain pointer hash per submission — the cap doesn't need to be generous since the hash itself is just a pointer to off-chain content.

**Relevant Files:** `contracts/dispute_resolution_contract/lib.rs:205-245`, `docs/SECURITY_AUDIT.md:108-113`

**Testing Requirements:** Boundary test at cap, cap+1, and confirm existing `add_evidence_hash` happy-path tests still pass unmodified below the cap.

**Definition of Done:** No dispute's evidence list can grow without bound; the cap is documented and tested.

**Suggested Labels:** `security`, `performance`

---

### 44. No automated dispute evidence verification system despite being a named roadmap item

**Summary:** `PLAN.md` lists "Build automated dispute evidence verification system" as a Medium-High priority feature; today, `add_evidence_hash` stores raw 32-byte hashes with zero verification logic of any kind.

**Background:** `PLAN.md:18`. `contracts/dispute_resolution_contract/lib.rs:205-245` stores whatever `BytesN<32>` value the caller supplies — there is no hash-format validation, no linkage to an expected commitment scheme, no oracle callback, and no distinction between "hash of a real photo of the damaged package" and "32 random bytes."

**Problem Statement:** The evidence mechanism currently provides no more integrity guarantee than an unstructured free-text field would — anyone can submit an arbitrary hash claiming it "is" evidence, and admins resolving disputes (`resolve_dispute_pay_driver`, `resolve_dispute_refund_sender`, `resolve_dispute_split_funds`) have no on-chain signal about evidence validity, only an off-chain promise that the hash corresponds to something real. This gap is explicitly acknowledged as unsolved by the project's own roadmap.

**Proposed Solution:** At minimum, require evidence submissions to reference a pre-registered commitment scheme (e.g., a hash-of-hash reveal pattern, or an admin/oracle-signed attestation that a specific hash was reviewed and found valid before dispute resolution can proceed), building toward the "automated" verification PLAN.md describes as a longer-term goal.

**Acceptance Criteria:**
- [ ] A defined verification step (even a minimal admin-attestation one) exists between "evidence submitted" and "dispute resolved," rather than resolution being entirely independent of evidence content.
- [ ] The chosen design is documented in `docs/ARCHITECTURE_DECISION_RECORDS.md`.

**Technical Notes:** This is intentionally scoped as "at least a first verifiable step," since a fully automated, trustless evidence-verification oracle is a substantial, multi-contract undertaking (likely its own ADR and possibly its own contract) — this issue tracks closing the gap between the roadmap claim and the current zero-verification baseline, not delivering the entire long-term vision in one PR.

**Relevant Files:** `contracts/dispute_resolution_contract/lib.rs:205-245`, `PLAN.md:18`

**Testing Requirements:** Tests covering the new verification gate: unverified evidence blocks resolution (if that's the chosen design), verified evidence unblocks it.

**Definition of Done:** Dispute resolution is provably influenced by evidence verification status, not purely by admin discretion over opaque hashes.

**Suggested Labels:** `feature`

---

### 45. Dispute resolution's reputation-penalty cross-call is never exercised by any test

**Summary:** `resolve_dispute_refund_sender`'s call into `identity_reputation_contract::decrease_reputation` has zero test coverage, because `identity_reputation_contract` isn't even a dev-dependency of `dispute_resolution_contract`.

**Background:** `contracts/dispute_resolution_contract/lib.rs:280-295` conditionally cross-calls `decrease_reputation` on a configured `IdentityReputationContract` address. `contracts/dispute_resolution_contract/Cargo.toml`'s `[dev-dependencies]` section lists only `delivery_contract` and `escrow_contract` — not `identity_reputation_contract`. A search of `contracts/dispute_resolution_contract/test.rs` confirms `set_identity_reputation_contract` is never called in any test, and the only occurrence of the word "reputation" in the file is a comment (`test.rs:365`) acknowledging the penalty exists without actually testing it.

**Problem Statement:** Every existing test for `resolve_dispute_refund_sender` exercises only the `None`-branch of `if let Some(reputation_addr) = ...` (`lib.rs:280`), meaning the code path that actually penalizes a driver's reputation after a lost dispute — arguably the single most consequential side effect of a refund resolution — has never been run in CI. A regression here (e.g., wrong argument order, wrong penalty amount, or a broken cross-call) would ship undetected.

**Proposed Solution:** Add `identity_reputation_contract` as a dev-dependency of `dispute_resolution_contract`, and add integration tests that wire a real `identity_reputation_contract` instance, register a driver, raise and resolve a dispute against them via `resolve_dispute_refund_sender`, and assert the driver's `reputation_score` actually decreased by `DISPUTE_REPUTATION_PENALTY` (10).

**Acceptance Criteria:**
- [ ] `identity_reputation_contract` is a dev-dependency of `dispute_resolution_contract`.
- [ ] A new test wires `set_identity_reputation_contract` and asserts the reputation penalty is applied end-to-end.

**Technical Notes:** This mirrors the pattern `dispute_resolution_contract/test.rs` already uses for its `escrow_contract`/`delivery_contract` integration tests — no new test infrastructure needs inventing, just an additional dev-dependency and setup step.

**Relevant Files:** `contracts/dispute_resolution_contract/Cargo.toml`, `contracts/dispute_resolution_contract/lib.rs:280-295`, `contracts/dispute_resolution_contract/test.rs`

**Testing Requirements:** As described in Proposed Solution; also add a test confirming the resolution still succeeds gracefully when no identity contract is configured (the current, only-tested branch), so both paths have coverage going forward.

**Definition of Done:** The reputation-penalty side effect of dispute resolution is verified by an automated test, not just a code comment.

**Suggested Labels:** `test`

---

### 46. No `proptest` dependency anywhere despite extensive property-testing documentation

**Summary:** `docs/TESTING.md`, `docs/SECURITY_AUDIT.md`, and `PLAN.md` each describe property-based testing (via `proptest`) as part of the project's testing practice, complete with example code — but `proptest` is not a dependency of any crate in the workspace.

**Background:** `docs/TESTING.md:178-200` shows a full `proptest!` example and instructs `cargo add proptest --dev`. `docs/SECURITY_AUDIT.md:183-187` lists `cargo test proptest` as part of the security test suite. `PLAN.md:32` lists "Write property-based tests for fee calculations" as a Critical-priority testing task. A check of every contract's `Cargo.toml` `[dev-dependencies]` section confirms none declare `proptest`.

**Problem Statement:** Anyone following `docs/TESTING.md`'s instructions to run `cargo test proptest` will simply find nothing, since no property tests exist. `PRODUCTION_READINESS.md`'s Testing section explicitly claims "[x] Property-based testing framework" as already implemented — a claim this finding directly contradicts, independent of the broader documentation-accuracy concerns already raised in GitHub #34.

**Proposed Solution:** Add `proptest` as a dev-dependency to at least `escrow_contract` (the contract with the most fee-calculation arithmetic) and implement the exact property the docs already describe as an example: fee is always `< amount` and `>= 0` across the full valid input space.

**Acceptance Criteria:**
- [ ] `proptest` is a declared dev-dependency of at least `escrow_contract`.
- [ ] At least one property-based test exists and passes, covering `calculate_fee`'s invariants across a wide range of `amount`/`platform_fee_bps` values.

**Technical Notes:** `calculate_fee` (`escrow_contract/lib.rs:52-54`) is a pure function, making it the lowest-friction starting point for property testing without needing `Env`/storage setup.

**Relevant Files:** `contracts/escrow_contract/Cargo.toml`, `contracts/escrow_contract/lib.rs:52-54`, `docs/TESTING.md:178-200`, `docs/SECURITY_AUDIT.md:183-187`, `PLAN.md:32`

**Testing Requirements:** The proptest itself is the deliverable; also verify `cargo test` runtime doesn't regress unacceptably (property tests can be slow — tune case counts if needed).

**Definition of Done:** Running the exact commands `docs/TESTING.md` and `docs/SECURITY_AUDIT.md` already document actually exercises real property-based tests.

**Suggested Labels:** `test`

---

### 47. No fuzz targets despite `SECURITY_AUDIT.md` prescribing `cargo fuzz` commands

**Summary:** `docs/SECURITY_AUDIT.md` instructs running `cargo fuzz run escrow_operations`, `cargo fuzz run state_transitions`, and `cargo fuzz run fee_calculations` as part of the security testing process — none of these fuzz targets, nor a `fuzz/` directory, nor a `cargo-fuzz` setup of any kind exists in the repository.

**Background:** `docs/SECURITY_AUDIT.md:158-167`. A workspace-wide search for `fuzz` finds no `fuzz/` directory, no `#![no_main]` libfuzzer harness, and no fuzz-related dev-dependency anywhere. `PLAN.md:33` also lists "Add fuzzing tests for state machine transitions" as a Critical-priority open task, consistent with this being unimplemented rather than removed.

**Problem Statement:** State-machine transition logic (`delivery_contract::validate_transition`, the escrow status guards) and fee-calculation arithmetic are exactly the kind of logic fuzzing is best at catching edge cases in, and the project's own security documentation asserts this practice is in place. It is not; anyone attempting to follow the documented security-testing procedure verbatim will hit a wall immediately.

**Proposed Solution:** Add a `fuzz/` directory with `cargo-fuzz`, and implement at least the `state_transitions` target `SECURITY_AUDIT.md` already names, fuzzing `delivery_contract::validate_transition` across the full `DeliveryStatus` state space to confirm it never allows an undocumented transition and never panics.

**Acceptance Criteria:**
- [ ] A `fuzz/` directory exists with at least one working fuzz target matching one of the three named in `docs/SECURITY_AUDIT.md`.
- [ ] The fuzz target runs cleanly for a reasonable duration (e.g., 60 seconds) in CI or as a documented manual step without crashing.

**Technical Notes:** `validate_transition` (`delivery_contract/lib.rs:35-53`) is a pure, `no_std`-compatible function taking two `DeliveryStatus` enum values — an ideal, low-effort first fuzz target requiring no `Env`/storage scaffolding.

**Relevant Files:** `contracts/delivery_contract/lib.rs:35-53`, `docs/SECURITY_AUDIT.md:158-167`, `PLAN.md:33`

**Testing Requirements:** N/A beyond the fuzz target itself; consider wiring a time-boxed fuzz run into CI as a non-blocking job.

**Definition of Done:** At least one of the three fuzz targets `docs/SECURITY_AUDIT.md` documents actually exists and runs.

**Suggested Labels:** `test`

---

### 48. Two-step admin transfer (`propose_admin`/`accept_admin`) has zero test coverage

**Summary:** `escrow_contract`'s entire two-step admin-transfer mechanism — explicitly called out as a security feature in ADR-005, `docs/GOVERNANCE.md`, and `PRODUCTION_READINESS.md` — has no test in `escrow_contract/test.rs` at all.

**Background:** `contracts/escrow_contract/lib.rs:245-289` implements `propose_admin` and `accept_admin`. A search of `contracts/escrow_contract/test.rs` for `propose_admin`/`accept_admin` returns zero matches. `docs/ARCHITECTURE_DECISION_RECORDS.md`'s ADR-005 specifically frames this as a deliberate security design ("Prevents accidental transfers... New admin must prove access"), and `PRODUCTION_READINESS.md`'s Security section lists "[x] Two-step admin transfer mechanism" as already-verified.

**Problem Statement:** This is the sole mechanism for ever changing who controls `update_platform_fee`, `set_settlement_contract`, `resolve_dispute`, and `resolve_dispute_split` on a live deployment — an untested regression here (e.g., someone accidentally allowing a third party other than the pending admin to call `accept_admin`, or breaking the `propose_admin` authorization check) could lock out or hand control of the entire escrow contract to the wrong party, and nothing in CI would catch it. It is also the only place in this contract using raw `panic!("...")` instead of the contract's own typed `EscrowError`/`FaniLabError` (`lib.rs:253, 272`), compounding the risk since a test relying on a specific panic string is more brittle than one matching a typed error.

**Proposed Solution:** Add tests covering: successful propose → accept flow updates `get_admin()`; a non-admin calling `propose_admin` is rejected; a non-pending address calling `accept_admin` is rejected; `accept_admin` with no pending proposal is rejected; the old admin loses privileges immediately after a successful transfer.

**Acceptance Criteria:**
- [ ] All five scenarios above have passing tests.
- [ ] (Optional but recommended alongside this fix) the two raw `panic!` calls at `lib.rs:253, 272` are converted to typed errors so the new tests can assert on structured error values rather than panic message strings.

**Technical Notes:** This pairs naturally with a broader "typed errors everywhere" pass, but the test-coverage gap is real and valuable independent of that refactor.

**Relevant Files:** `contracts/escrow_contract/lib.rs:245-289`, `contracts/escrow_contract/test.rs`

**Testing Requirements:** As listed in Proposed Solution.

**Definition of Done:** The two-step admin transfer path, the sole recovery mechanism for a compromised or rotating admin key, is fully covered by automated tests.

**Suggested Labels:** `test`

---

### 49. `settlement_contract` test suite only exercises `init`

**Summary:** `settlement_contract`'s entire test module consists of a single test (`test_init`) that calls `init` and asserts nothing about its return value or any subsequent state — `get_driver_preference` and `execute_settlement_swap` have no tests whatsoever, including no test of the one piece of real logic the contract currently has: `execute_settlement_swap`'s `caller.require_auth()` check.

**Background:** `contracts/settlement_contract/src/lib.rs:43-59` is the entire test module:
```rust
#[test]
fn test_init() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(SettlementContract, ());
    let client = SettlementContractClient::new(&env, &contract_id);
    client.init(&admin);
}
```
No test calls `get_driver_preference` or `execute_settlement_swap` at all.

**Problem Statement:** Even accounting for the fact that `settlement_contract`'s real swap logic is intentionally deferred to "Phase 3" (per its own header comment, and per GitHub #30), the contract *today* still has real, testable behavior: `get_driver_preference` always returning `None`, and `execute_settlement_swap` enforcing `caller.require_auth()` before doing nothing else. Neither is verified. If a future change to `execute_settlement_swap` accidentally weakens or removes the auth check while adding Phase 3 logic, there is no existing test that would catch the regression, because there's no baseline test establishing the auth check's current behavior.

**Proposed Solution:** Add tests for `get_driver_preference` (asserts `None` today, giving a clear "this changed" signal once Phase 3 lands) and for `execute_settlement_swap`'s auth requirement (asserts an unauthorized caller is rejected).

**Acceptance Criteria:**
- [ ] A test asserts `get_driver_preference` returns `None` for an arbitrary address.
- [ ] A test asserts `execute_settlement_swap` requires the caller's authorization (fails without it, per Soroban's `require_auth` test patterns).

**Technical Notes:** This is a small, low-risk addition that also gives the "settlement is a live no-op stub already wired into the payout path" tracking issue (GitHub #30) a concrete regression net for the one piece of behavior that does exist today.

**Relevant Files:** `contracts/settlement_contract/src/lib.rs:14-59`

**Testing Requirements:** As described above.

**Definition of Done:** Both public functions of `settlement_contract` beyond `init` have at least one test each.

**Suggested Labels:** `test`

---

### 50. `deploy-all-contracts.sh` still builds with the pre-migration `wasm32-unknown-unknown` target

**Summary:** The deployment script builds contracts with `cargo build --target wasm32-unknown-unknown --release`, the exact target `SOROBAN_SDK_27_MIGRATION.md` documents as replaced by `wasm32v1-none` — every CI workflow was updated for the migration, but this script was not.

**Background:** `scripts/deploy-all-contracts.sh:46` (`cargo build --target wasm32-unknown-unknown --release`) and `:59` (reads WASM files from `target/wasm32-unknown-unknown/release/`). Compare with `.github/workflows/ci.yml:24,34,37`, `release.yml:23,26`, and `deploy-testnet.yml:36,46`, all of which use `wasm32v1-none`, and `SOROBAN_SDK_27_MIGRATION.md:13-16`, which states plainly: "**Reason**: Soroban SDK 27.0.0 requires the new `wasm32v1-none` target."

**Problem Statement:** Since the workspace's `Cargo.toml` pins `soroban-sdk = "27.0.0"`, running this script as-is will either fail to build against the old target, or (worse, if it happens to build) produce a `.wasm` artifact using an unsupported/incorrect target ABI for the SDK version actually in use — meaning the one script `docs/DEPLOYMENT.md` and `PRODUCTION_READINESS.md` point to as evidence of "automated deployment scripts" is presently broken for the exact SDK version the codebase runs on.

**Proposed Solution:** Update both the build command and the WASM path glob in `deploy-all-contracts.sh` to `wasm32v1-none`, matching every CI workflow.

**Acceptance Criteria:**
- [ ] `deploy-all-contracts.sh` builds and locates artifacts using `wasm32v1-none`, consistent with CI.
- [ ] A local dry run (or a CI smoke-test job) confirms the script finds and would deploy the correct WASM files.

**Technical Notes:** This is a pure find/replace of `wasm32-unknown-unknown` → `wasm32v1-none` in two spots; see also issue #51 for the same bug in `Makefile`.

**Relevant Files:** `scripts/deploy-all-contracts.sh:46, 59`

**Testing Requirements:** Add a lightweight CI job or pre-commit check that greps deployment scripts for `wasm32-unknown-unknown` to prevent this drift from recurring after future target changes.

**Definition of Done:** `deploy-all-contracts.sh` builds against the same target every CI workflow already uses.

**Suggested Labels:** `bug`

---

### 51. `Makefile` targets still use `wasm32-unknown-unknown` and cover only 3 of 6 contracts

**Summary:** Every build target in the root `Makefile` — `build`, `build-escrow`, `build-delivery`, `build-dispute` — compiles against the stale `wasm32-unknown-unknown` target, and there are no equivalent per-contract targets for `fleet_management_contract`, `identity_reputation_contract`, or `settlement_contract`.

**Background:** `Makefile:9-19` shows all four build targets using `--target wasm32-unknown-unknown`, the same stale target flagged in issue #50 but in a separate file with its own independent drift. `PRODUCTION_READINESS.md:221` cites the Makefile as evidence of "Developer Experience 10/10" ("Windows-friendly Makefile"), yet running `make build` today would target an ABI the current `soroban-sdk = "27.0.0"` dependency was migrated away from.

**Problem Statement:** Contributors following the most obvious entry point (`make build` or `make test`, both documented at the top of the file) get a build that doesn't match what CI actually verifies (`wasm32v1-none`), risking "works on my machine, fails in CI" confusion, or a WASM artifact that silently doesn't match the SDK version in use. The missing `build-fleet`/`build-identity`/`build-settlement` targets are a smaller but related gap — half the contracts have no convenient single-contract build shortcut at all.

**Proposed Solution:** Update all four existing targets to `wasm32v1-none`, and add the three missing per-contract targets for parity.

**Acceptance Criteria:**
- [ ] `make build`, `make test`, and every `build-*` target use `wasm32v1-none`.
- [ ] `build-fleet`, `build-identity`, and `build-settlement` targets exist, matching the pattern of the existing three.

**Technical Notes:** Check `Makefile.windows` for the same drift while making this change, since it likely mirrors the same stale target.

**Relevant Files:** `Makefile:1-30`, `Makefile.windows`

**Testing Requirements:** Run `make build` locally post-fix and confirm the resulting WASM artifacts land under `target/wasm32v1-none/release/`.

**Definition of Done:** The Makefile builds against the same target as CI, with a build target available for every one of the six contracts.

**Suggested Labels:** `bug`

---

### 52. `initialize-all-contracts.sh` only initializes 2 of the 6 deployed contracts

**Summary:** `deploy-all-contracts.sh` deploys all six contracts, but the companion `initialize-all-contracts.sh` only calls `init` on `escrow_contract` and `delivery_contract` — `dispute_resolution_contract`, `fleet_management_contract`, `identity_reputation_contract`, and `settlement_contract` are deployed but left completely uninitialized and unwired by the documented deployment flow.

**Background:** `scripts/deploy-all-contracts.sh:91` lists `CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")`. `scripts/initialize-all-contracts.sh:40-65` only contains an "Initialize Escrow Contract" step and an "Initialize Delivery Contract" step; there is no corresponding step for the other four, and no step that performs any of the required cross-contract wiring calls (`escrow_contract::set_settlement_contract`, `dispute_resolution_contract::set_identity_reputation_contract`, `fleet_management_contract::set_identity_contract`).

**Problem Statement:** Following `docs/DEPLOYMENT.md`'s exact documented flow ("Use the initialization script... `./scripts/initialize-all-contracts.sh $STELLAR_NETWORK`") leaves four of six contracts uninitialized on a fresh deployment. Any call into `dispute_resolution_contract`, `fleet_management_contract`, `identity_reputation_contract`, or `settlement_contract` after following this script exactly as documented will panic with `NotInitialized`/`AlreadyInitialized`-adjacent errors, since their `init`/`initialize` functions were simply never invoked.

**Proposed Solution:** Extend `initialize-all-contracts.sh` to initialize all six contracts in the dependency order `docs/DEPLOYMENT.md:178-187` already documents, and add the cross-contract wiring calls needed for the protocol to actually function end-to-end (settlement contract address on escrow, identity contract address on dispute resolution and fleet management).

**Acceptance Criteria:**
- [ ] All six contracts are initialized by the script.
- [ ] All documented cross-contract address-wiring admin calls are included.
- [ ] The JSON parsing in the script (currently `grep`-based, see Technical Notes) is extended to read all six contract IDs, not just two.

**Technical Notes:** The script's `grep -o '"escrow_contract": "[^"]*'`-style JSON parsing (`initialize-all-contracts.sh:33-34`) is fragile but functional for the two keys it currently reads; extending it to six keys is mechanical, though switching to `jq` would be more robust (see also issue #54's related script-robustness observations).

**Relevant Files:** `scripts/initialize-all-contracts.sh:1-69`, `scripts/deploy-all-contracts.sh:91`, `docs/DEPLOYMENT.md:256-260`

**Testing Requirements:** Manual dry run against a local/testnet deployment confirming all six contracts report initialized state (e.g., via each contract's `get_admin`-equivalent query) after running the script once.

**Definition of Done:** Running `deploy-all-contracts.sh` followed by `initialize-all-contracts.sh` leaves every deployed contract initialized and cross-wired, with no manual follow-up steps required beyond what the scripts already promise.

**Suggested Labels:** `bug`

---

### 53. Deploy script's error handling after `cargo build` is unreachable dead code

**Summary:** `deploy-all-contracts.sh` runs under `set -e`, which means the `if [ $? -ne 0 ]` check immediately following `cargo build` (and the analogous pattern inside the `deploy_contract` loop) can never actually execute — the script already exits at the failing command before reaching its own error-handling branch.

**Background:** `scripts/deploy-all-contracts.sh:6` sets `set -e`. `:46-51`:
```bash
cargo build --target wasm32-unknown-unknown --release

if [ $? -ne 0 ]; then
    echo "${RED}❌ Build failed${NC}"
    exit 1
fi
```
Under `set -e`, if `cargo build` returns non-zero, the shell terminates the script immediately at that line — it never reaches the `if` statement, whose custom red-text failure message is therefore always dead code. The same shape recurs for the per-contract deploy loop at `:96-101`, where `contract_id=$(deploy_contract "$contract")`'s failure (a non-zero exit from a command substitution assignment) similarly triggers `set -e` before the subsequent `if [ $? -ne 0 ]` check can run.

**Problem Statement:** The script's authors clearly intended for build/deploy failures to print a specific, actionable red-colored error message before exiting — but because of the `set -e`/`$?`-check interaction, users instead get bash's generic, uncustomized "command failed" behavior (just the raw `cargo`/`stellar` CLI output, without the intended wrapper messaging). This is a real, if minor, gap between intended and actual script behavior that could confuse operators debugging a failed deployment.

**Proposed Solution:** Either remove `set -e` and handle every failure explicitly via `$?` checks (more verbose, but makes the existing error messages actually reachable), or keep `set -e` and replace the dead `$?` checks with `||` error traps (e.g., `cargo build ... || { echo "..."; exit 1; }`) that fire correctly under `set -e`.

**Acceptance Criteria:**
- [ ] The custom failure messages in `deploy-all-contracts.sh` actually execute when the corresponding command fails (verify by intentionally breaking a build/deploy step locally and observing the custom message, not just bash's default error).

**Technical Notes:** The `||`-trap approach is the more idiomatic fix and requires no behavioral change beyond making the existing messages reachable.

**Relevant Files:** `scripts/deploy-all-contracts.sh:6, 44-54, 90-101`

**Testing Requirements:** Manual verification: temporarily point the build at a nonexistent target or break a contract's build, run the script, and confirm the custom red error message (not just raw `cargo`/`stellar` output) is what's printed before exit.

**Definition of Done:** The script's intended custom error messaging actually fires on failure paths.

**Suggested Labels:** `bug`

---

### 54. `.env.example` doesn't match the variables `DEPLOYMENT.md` and the scripts actually need

**Summary:** The committed `.env.example` has only 4 variables, one of which is named differently from what the documentation and scripts expect, and it's missing every variable `initialize-all-contracts.sh` actually sources from `.env` at runtime.

**Background:** `.env.example` contains only:
```
STELLAR_NETWORK=testnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
CONTRACT_DEPLOYER_KEY=S...
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```
`docs/DEPLOYMENT.md:41-63` documents a much larger set of "Required Environment Variables," including `CONTRACT_DEPLOYER_SECRET` (not `CONTRACT_DEPLOYER_KEY`), `ADMIN_ADDRESS`, six `*_CONTRACT_ID` variables, `TOKEN_ADDRESS`, and `PLATFORM_FEE_BPS`. `scripts/initialize-all-contracts.sh:26-49` sources `.env` and directly references `$ADMIN_ADDRESS` and `$TOKEN_ADDRESS` in its `stellar contract invoke` calls — neither of which appears anywhere in the shipped `.env.example`.

**Problem Statement:** Anyone following `docs/DEPLOYMENT.md`'s own instructions ("Copy environment template... Edit .env with your configuration") and using `.env.example` as their starting point will produce a `.env` file missing `ADMIN_ADDRESS` and `TOKEN_ADDRESS`, causing `initialize-all-contracts.sh` to invoke contracts with empty/unset values for those parameters — a runtime failure that only becomes visible when the initialization script is actually run, not when `.env` is created. The `CONTRACT_DEPLOYER_KEY` vs `CONTRACT_DEPLOYER_SECRET` naming mismatch compounds the confusion for anyone cross-referencing the file against the docs.

**Proposed Solution:** Regenerate `.env.example` to include every variable `docs/DEPLOYMENT.md` documents as required, using matching names, so the template and the docs/scripts agree.

**Acceptance Criteria:**
- [ ] `.env.example` contains every variable referenced by `docs/DEPLOYMENT.md`'s "Required Environment Variables" section and by both scripts in `scripts/`.
- [ ] Variable names are identical across `.env.example`, `docs/DEPLOYMENT.md`, and the scripts (resolving the `CONTRACT_DEPLOYER_KEY`/`CONTRACT_DEPLOYER_SECRET` mismatch in one direction or the other).

**Technical Notes:** This pairs well with issue #52 (initialization script only covering 2 of 6 contracts) — fixing both together gives a coherent, complete deployment onboarding path.

**Relevant Files:** `.env.example`, `docs/DEPLOYMENT.md:41-63`, `scripts/initialize-all-contracts.sh:26-49`

**Testing Requirements:** A dry run of `cp .env.example .env` followed by filling in placeholder values and running `initialize-all-contracts.sh` should not hit any unset-variable surprises beyond what's obviously a placeholder.

**Definition of Done:** `.env.example`, the deployment docs, and the deployment scripts are all in agreement about what environment variables exist and what they're called.

**Suggested Labels:** `bug`

---

### 55. Release workflow's "Optimize WASM" step performs no optimization

**Summary:** The `release.yml` workflow has a step literally named "Optimize WASM" that only copies `.wasm` files from the build output directory to the release artifacts directory — it never invokes `wasm-opt` or any other size-reduction tool.

**Background:** `.github/workflows/release.yml:28-34`:
```yaml
- name: Optimize WASM
  run: |
    mkdir -p release_artifacts
    for contract in target/wasm32v1-none/release/*.wasm; do
      filename=$(basename "$contract")
      cp "$contract" "release_artifacts/$filename"
    done
```
This is a plain copy loop with no call to `wasm-opt`. `docs/DEPLOYMENT.md:105-115` and `docs/PERFORMANCE.md:139-147` both separately document `wasm-opt -Oz` as the recommended production-artifact optimization step, and `SOROBAN_SDK_27_MIGRATION.md:56-59` notes wasm-opt was intentionally *removed* from `deploy-testnet.yml` because it's "no longer needed with new target" — but that same rationale was never reconciled with `release.yml`'s step still being labeled as if optimization happens there.

**Problem Statement:** Whether or not `wasm-opt` is still needed under `wasm32v1-none` (per the migration doc's claim), the step name actively misleads anyone reading the release workflow into believing published release artifacts are size-optimized when they are, in fact, exactly the raw `cargo build --release` output. If `wasm-opt` genuinely isn't needed anymore, the step should be renamed to reflect that it's just artifact staging; if it *is* still valuable for mainnet-bound release binaries (`docs/PERFORMANCE.md`'s guidance doesn't caveat it as target-specific), the actual optimization call is missing.

**Proposed Solution:** Either rename the step to something accurate like "Stage Release Artifacts," or restore an actual `wasm-opt -Oz` invocation if optimized release binaries are still desired — resolving the discrepancy between `SOROBAN_SDK_27_MIGRATION.md`'s claim and `docs/PERFORMANCE.md`/`docs/DEPLOYMENT.md`'s continued recommendation of the tool.

**Acceptance Criteria:**
- [ ] The workflow step's name accurately reflects what it does.
- [ ] If `wasm-opt` is determined to still add value for `wasm32v1-none` release binaries, it's actually invoked; if not, `docs/PERFORMANCE.md`/`docs/DEPLOYMENT.md` are updated to stop recommending it.

**Technical Notes:** Confirm empirically whether `wasm-opt -Oz` changes artifact size at all under `wasm32v1-none` before deciding which direction to take — this determines whether the fix is "rename the step" or "actually optimize."

**Relevant Files:** `.github/workflows/release.yml:28-34`, `docs/DEPLOYMENT.md:105-115`, `docs/PERFORMANCE.md:139-147`, `SOROBAN_SDK_27_MIGRATION.md:56-59`

**Testing Requirements:** If `wasm-opt` is restored, compare `.wasm` file sizes before/after in a CI log to confirm it has a measurable effect; if not, no functional test is needed beyond the rename.

**Definition of Done:** The release workflow's step name and actual behavior agree, and the project's documentation no longer contradicts itself about whether `wasm-opt` is part of the release pipeline.

**Suggested Labels:** `performance`

---

### 56. CI reinstalls `cargo-audit`/`cargo-outdated`/`cargo-tarpaulin`/`cargo-deny` from source on every run

**Summary:** `ci.yml` and `security-audit.yml` each `cargo install` several auxiliary tools from source on every single workflow run, with no caching of the compiled binaries — adding several minutes of redundant compile time to every CI run and every scheduled daily security audit.

**Background:** `.github/workflows/ci.yml:43-45` (`cargo install cargo-tarpaulin`), `:54-56` (`cargo install cargo-audit`), `:59-61` (`cargo install cargo-outdated`). `.github/workflows/security-audit.yml:31-32` (`cargo install cargo-audit`), `:37-38` (`cargo install cargo-deny`). The `Swatinem/rust-cache@v2` step present in both workflows (`ci.yml:27-28`, `security-audit.yml:28-29`) caches the workspace's own dependency build artifacts, but by default does not cache globally-installed `cargo install` binaries under `~/.cargo/bin`, so each of these tools is rebuilt from source on every run, including the `security-audit.yml` workflow's **daily scheduled** invocation (`cron: '0 0 * * *'`, `security-audit.yml:4-6`).

**Problem Statement:** This is pure wasted CI time and compute, recurring on every push, every PR, and every day at midnight regardless of whether anything changed — four separate tool compilations (tarpaulin, audit, outdated, deny) that are functionally identical between runs unless the tool's own version bumps. At scale this meaningfully slows down PR feedback loops and inflates CI minutes consumed.

**Proposed Solution:** Cache `~/.cargo/bin` (or the specific tool binaries) keyed on tool version, or switch to prebuilt-binary installation methods (e.g., `taiki-e/install-action`, or GitHub Releases binary downloads) for tools that publish them, avoiding from-source compilation entirely in the common case.

**Acceptance Criteria:**
- [ ] Repeated CI runs (with no `Cargo.lock` or tool-version changes) reuse cached/prebuilt tool binaries instead of recompiling from source.
- [ ] Measured CI run time for the affected jobs decreases.

**Technical Notes:** `taiki-e/install-action` supports `cargo-audit`, `cargo-outdated`, `cargo-tarpaulin`, and `cargo-deny` via prebuilt binaries and is a drop-in replacement for the `cargo install` lines.

**Relevant Files:** `.github/workflows/ci.yml:42-61`, `.github/workflows/security-audit.yml:31-41`

**Testing Requirements:** Compare CI job duration before/after across a few runs to confirm the improvement; ensure tool versions used remain pinned/reproducible after the switch.

**Definition of Done:** No CI job spends time compiling `cargo-audit`/`cargo-outdated`/`cargo-tarpaulin`/`cargo-deny` from source when a cached or prebuilt alternative is available.

**Suggested Labels:** `performance`

---

### 57. No CI job enforces `--locked`, despite repeated manual `Cargo.lock` pinning fire-drills

**Summary:** None of `cargo build`, `cargo test`, or `cargo clippy` in `ci.yml` pass `--locked`, so CI can silently resolve dependency versions differently from what's committed in `Cargo.lock` — a gap the repository's own recent commit history shows has already caused real pain.

**Background:** `.github/workflows/ci.yml:34, 37, 40` run `cargo clippy`, `cargo build`, and `cargo test` with no `--locked` flag. The repository's recent commit log includes `d12c9b4 fix: downgrade soroban-sdk to 21.7.3 for CI compatibility`, `b847fbd fix: force update ethnum to 1.5.3 in Cargo.lock`, and `19418f5 fix: complete Cargo.lock update with ethnum 1.5.3` — three separate, sequential commits fighting `Cargo.lock`/dependency-resolution drift in CI within recent history.

**Problem Statement:** Without `--locked`, a `Cargo.lock` that's out of sync with `Cargo.toml` (or a registry index change that affects resolution) doesn't fail loudly and immediately — instead, Cargo silently re-resolves and potentially updates the lockfile in memory for that run, masking exactly the kind of drift that produced the three fix commits above. Enforcing `--locked` would have surfaced that drift as an explicit, actionable CI failure ("Cargo.lock is out of date") the first time it happened, rather than requiring iterative manual debugging across multiple commits.

**Proposed Solution:** Add `--locked` to the `cargo build`, `cargo test`, and `cargo clippy` invocations in `ci.yml`.

**Acceptance Criteria:**
- [ ] All three commands in `ci.yml` use `--locked`.
- [ ] CI fails clearly (with Cargo's own "the lock file needs to be updated" message) if `Cargo.lock` and `Cargo.toml` ever drift again, rather than silently re-resolving.

**Technical Notes:** This should be introduced carefully in a follow-up PR after confirming the current `Cargo.lock` is fully consistent post-SDK-27-migration, to avoid immediately breaking CI on landing.

**Relevant Files:** `.github/workflows/ci.yml:33-40`, recent commits `d12c9b4`, `b847fbd`, `19418f5`

**Testing Requirements:** Verify CI passes with `--locked` added on the current `main` branch state before merging; intentionally desync `Cargo.lock` locally afterward to confirm the flag produces the expected hard failure.

**Definition of Done:** `Cargo.lock`/`Cargo.toml` drift fails CI immediately and explicitly, rather than being silently absorbed by Cargo's default resolution behavior.

**Suggested Labels:** `enhancement`

---

### 58. `docs/API.md`'s table of contents promises 4 contracts it never documents

**Summary:** `docs/API.md`'s table of contents lists sections for Dispute Resolution, Fleet Management, Identity Reputation, and Settlement contracts — none of the four actually appear anywhere in the document body, which only covers Escrow, Delivery, and Shared Types.

**Background:** `docs/API.md:6-12`:
```markdown
- [Escrow Contract](#escrow-contract)
- [Delivery Contract](#delivery-contract)
- [Dispute Resolution Contract](#dispute-resolution-contract)
- [Fleet Management Contract](#fleet-management-contract)
- [Identity Reputation Contract](#identity-reputation-contract)
- [Settlement Contract](#settlement-contract)
- [Shared Types](#shared-types)
```
Scanning the full document (`docs/API.md`, 588 lines) confirms `## Escrow Contract` (`:16`), `## Delivery Contract` (`:234`), and `## Shared Types` (`:394`) are the only top-level sections that actually exist — there is no `## Dispute Resolution Contract`, `## Fleet Management Contract`, `## Identity Reputation Contract`, or `## Settlement Contract` heading anywhere in the file.

**Problem Statement:** `docs/API.md` bills itself as "Complete API documentation for all FaniLab smart contracts," but over half the deployed contracts have zero function-level documentation — no parameters, no authorization requirements, no error tables, nothing — despite the ToC implying otherwise. Anyone clicking those ToC links lands nowhere (broken in-page anchors), and anyone integrating against `fleet_management_contract` or `identity_reputation_contract` has no API reference at all, unlike `escrow_contract`/`delivery_contract` integrators.

**Proposed Solution:** Add the four missing sections, documenting each public function's parameters, authorization requirements, errors, events, and state changes, following the exact format already established for the Escrow and Delivery sections.

**Acceptance Criteria:**
- [ ] `docs/API.md` contains a real section for every contract listed in its own table of contents.
- [ ] Each new section follows the existing format (Parameters/Authorization/Errors/Events/State Changes) used for Escrow and Delivery.

**Technical Notes:** This is a large but mechanical documentation task — the actual function signatures, authorization checks, and error variants are all directly readable from each contract's `lib.rs`.

**Relevant Files:** `docs/API.md:1-13, 234-392` (existing pattern to follow)

**Testing Requirements:** N/A (documentation-only); consider a lightweight CI check (e.g., a script asserting every ToC anchor has a matching heading) to prevent this from recurring.

**Definition of Done:** `docs/API.md`'s table of contents and body are fully consistent — every linked section exists and documents its contract's real public interface.

**Suggested Labels:** `documentation`

---

### 59. `docs/API.md` footer claims Soroban SDK 22.0.1, three versions behind actual

**Summary:** `docs/API.md`'s final line states "**Soroban SDK**: 22.0.1" — the workspace has since migrated fully to SDK 27.0.0, per `Cargo.toml` and `SOROBAN_SDK_27_MIGRATION.md`.

**Background:** `docs/API.md:588` reads `**Soroban SDK**: 22.0.1`. The workspace root `Cargo.toml` pins `soroban-sdk = "27.0.0"`, and `SOROBAN_SDK_27_MIGRATION.md` documents this migration as complete as of 2026-07-14.

**Problem Statement:** A reference document's stated SDK version is exactly the kind of detail an integrator or new contributor would check first to determine client-library compatibility (per the document's own "SDKs and Client Libraries" section immediately above it), and it's simply wrong. Combined with issue #80 (below), this suggests the SDK-27 migration's documentation follow-through was incomplete outside of the dedicated migration doc itself.

**Proposed Solution:** Update the footer to reflect SDK 27.0.0 (and consider deriving this line from `Cargo.toml` in a future doc-generation pass rather than hand-maintaining it, to prevent recurrence).

**Acceptance Criteria:**
- [ ] `docs/API.md`'s footer states the correct, current Soroban SDK version.

**Technical Notes:** Grep the rest of `docs/` for other stray `22.0.1`/pre-migration version references that may have been missed in the same sweep.

**Relevant Files:** `docs/API.md:585-588`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** Every version marker in `docs/API.md` matches the workspace's actual, current `soroban-sdk` dependency version.

**Suggested Labels:** `documentation`

---

### 60. Three architecture/design docs are committed as completely empty files

**Summary:** `docs/architecture/event-system.md`, `docs/contract-design/escrow-design.md`, and `docs/protocol/delivery-protocol.md` all exist in the repository but contain zero bytes of content.

**Background:** All three files are present in the `docs/` tree structure (implying they were intentionally scaffolded) but reading any of them returns an empty file. `docs/architecture/smart-contract-architecture.md` (a sibling file in the same `docs/architecture/` directory) is fully written, showing the directory structure is real and actively maintained — these three are the exception, not evidence the whole area is abandoned.

**Problem Statement:** `event-system.md` is exactly the document that should resolve the ambiguity flagged in issue #41 (whether `shared_types::events` or ad hoc inline topics is the intended pattern) but currently offers nothing. `escrow-design.md` and `delivery-protocol.md` are presumably meant to be the deep-dive design rationale for the two most financially/logically critical contracts in the protocol, and both are blank. A new contributor or auditor navigating `docs/` by directory structure would reasonably expect these files to contain the project's most important design context, only to find nothing.

**Proposed Solution:** Populate each file with real content, or remove them if they're confirmed placeholders with no near-term content plan (an empty file that looks intentional is worse than no file, since it implies completeness that doesn't exist).

**Acceptance Criteria:**
- [ ] Each of the three files either contains substantive content or is removed from the repository.
- [ ] If populated, `event-system.md` explicitly resolves the shared-event-system-vs-inline-topics question raised in issue #41.

**Technical Notes:** `docs/contract-design/escrow-design.md` and `docs/protocol/delivery-protocol.md` could reasonably be derived from the already-thorough inline doc comments in `escrow_contract/lib.rs` and `delivery_contract/lib.rs` (e.g., the state-machine transition table already documented in `delivery_contract/lib.rs:27-34`).

**Relevant Files:** `docs/architecture/event-system.md`, `docs/contract-design/escrow-design.md`, `docs/protocol/delivery-protocol.md`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** No doc file in the repository is committed empty; every file under `docs/` contains the content its filename and location promise.

**Suggested Labels:** `documentation`

---

### 61. Docs disagree with themselves on whether the protocol has 6 or 7 contracts

**Summary:** `docs/architecture/smart-contract-architecture.md` and `docs/ARCHITECTURE_DECISION_RECORDS.md`'s ADR-002 both describe the system as "7 core smart contracts," but the workspace ships exactly 6 deployable contract crates plus 1 non-deployed shared library — and the architecture doc's own enumeration (1 through 7) actually lists the library as item #1, contradicting its own "7 contracts" framing two lines above.

**Background:** `docs/architecture/smart-contract-architecture.md:5`: "the system is broken down into **7 core smart contracts (and libraries)**." Its own numbered list then runs `1. shared_types (Library)` through `7. settlement_contract` — i.e., 6 actual contracts plus 1 library, deliberately parenthesized as "(Library)" to distinguish it from the six real contracts, yet the header sentence counts it as one of the "7... contracts" anyway. `docs/ARCHITECTURE_DECISION_RECORDS.md`'s ADR-002 (`:38-47`) independently states "Use 7 specialized contracts with shared types library" — again conflating the library with the contract count. `Cargo.toml`'s workspace `members = ["contracts/*"]` includes `shared_types`, but its own `Cargo.toml` declares only `crate-type = ["rlib"]` (no `cdylib`), confirming it is not, and cannot be, deployed as a Soroban contract.

**Problem Statement:** This is a small but real internal inconsistency across two separate architecture documents that both got the same detail wrong the same way — suggesting the "7 contracts" framing may have been copy-pasted between them rather than independently verified against the actual crate layout, and neither has been corrected despite the codebase settling at 6 deployable contracts.

**Proposed Solution:** Update both documents to consistently describe 6 deployable contracts plus 1 shared (non-deployed) library, matching the actual crate structure.

**Acceptance Criteria:**
- [ ] `docs/architecture/smart-contract-architecture.md` and `docs/ARCHITECTURE_DECISION_RECORDS.md` both accurately describe the contract/library count and don't contradict their own itemized lists.

**Technical Notes:** None beyond careful wording — this is a pure documentation-accuracy fix with no code implications.

**Relevant Files:** `docs/architecture/smart-contract-architecture.md:1-6`, `docs/ARCHITECTURE_DECISION_RECORDS.md:38-47`, `contracts/shared_types/Cargo.toml`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** Both documents describe the same, accurate contract/library count with no internal self-contradiction.

**Suggested Labels:** `documentation`

---

### 62. `escrow_contract` and `delivery_contract` each hand-roll an identical private `is_admin` helper

**Summary:** Both contracts independently implement a private `is_admin(env, caller) -> bool` function with effectively identical logic (compare `caller` against the `Admin` value in instance storage), rather than sharing one implementation from `shared_types`, which already owns `StorageKey::Admin`.

**Background:** `contracts/escrow_contract/lib.rs:30-37`:
```rust
fn is_admin(env: &Env, caller: &Address) -> bool {
    let stored_admin: Address = env.storage().instance().get(&StorageKey::Admin).expect("Not initialized");
    *caller == stored_admin
}
```
`contracts/delivery_contract/lib.rs:354-364`:
```rust
fn is_admin(env: &Env, caller: &Address) -> bool {
    if let Some(admin) = env.storage().instance().get::<_, Address>(&StorageKey::Admin) {
        admin == *caller
    } else {
        false
    }
}
```
Both read the same `shared_types::StorageKey::Admin` key; the only functional difference is that `escrow_contract`'s version panics via `.expect()` if uninitialized while `delivery_contract`'s returns `false` — an inconsistency that is itself a small correctness smell (the two contracts disagree on what "is the caller admin" should do before `init` has ever run).

**Problem Statement:** This is the exact kind of small, easy-to-miss divergence that produces subtly different behavior across contracts for what should be identical logic — if one copy is fixed or hardened in the future (e.g., issue #34's admin-model work) and the other isn't, the two contracts will silently drift further apart. `shared_types` (per ADR-003's own stated rationale, "Single source of truth... Easier maintenance") is the obvious place to centralize this.

**Proposed Solution:** Add a single `pub fn is_admin(env: &Env, caller: &Address) -> bool` helper to `shared_types`, decide on one consistent pre-init behavior (returning `false` is safer and matches `delivery_contract`'s current choice), and have both contracts call the shared version.

**Acceptance Criteria:**
- [ ] Exactly one `is_admin` implementation exists in the workspace, in `shared_types`.
- [ ] `escrow_contract` and `delivery_contract` both use it, with consistent pre-init behavior.

**Technical Notes:** `fleet_management_contract` also has similar (though not identical, since it inlines the check rather than extracting a helper) admin-comparison logic at several call sites — worth reviewing for the same consolidation once the shared helper exists.

**Relevant Files:** `contracts/escrow_contract/lib.rs:30-37`, `contracts/delivery_contract/lib.rs:354-364`, `contracts/shared_types/lib.rs`

**Testing Requirements:** Existing admin-authorization tests in both contracts should continue to pass unmodified after the consolidation; add one test confirming pre-init `is_admin` behavior is now consistent between the two contracts.

**Definition of Done:** One shared `is_admin` implementation is used everywhere it's needed, with no duplicated logic and no cross-contract behavioral inconsistency.

**Suggested Labels:** `refactor`

---

### 63. No admin override/recovery path in `fleet_management_contract` for a compromised owner key

**Summary:** `fleet_management_contract` stores an `Admin` address at `init` but the only thing that admin can ever do is call `set_identity_contract` — there is no admin-level ability to reassign a fleet's ownership, force-update a treasury, or otherwise intervene if a fleet owner's key is lost or compromised.

**Background:** `contracts/fleet_management_contract/lib.rs:70-96` shows `init` storing `DataKey::Admin`, used exclusively by `set_identity_contract` (`:83-96`). Every other mutating function — `register_fleet`, `update_fleet_treasury`, `add_driver_to_fleet`, `accept_fleet_invite`, `remove_driver_from_fleet` — is gated solely on `owner`/`driver` identity, with the contract-level `Admin` playing no role whatsoever.

**Problem Statement:** If a fleet owner's private key is lost or compromised, there is currently no recovery path: the admin cannot reassign the fleet to a new owner address, cannot force-update a compromised treasury address to stop further payouts from routing there (once GH #12's treasury-routing wiring lands), and cannot remove drivers on the owner's behalf. This is a meaningful operational gap for a contract explicitly designed to serve "Enterprise Logistics SMEs" (per `docs/architecture/smart-contract-architecture.md:29`) — organizations that will reasonably expect some administrative recourse for a lost enterprise credential, similar to what `escrow_contract`'s admin already provides for the escrow lifecycle (`release_escrow`/`refund_escrow`/`resolve_dispute` are all admin-callable).

**Proposed Solution:** Add admin-gated override functions — e.g., `admin_reassign_fleet_owner(admin, fleet_id, new_owner)` and `admin_force_update_treasury(admin, fleet_id, new_treasury)` — mirroring the recovery-oriented admin powers already present in `escrow_contract`, each emitting a distinct event for auditability.

**Acceptance Criteria:**
- [ ] An admin can reassign a fleet's `owner` and/or force-update its `treasury` without cooperation from the current owner.
- [ ] Both actions emit dedicated events distinguishable from owner-initiated changes.
- [ ] Tests cover both the happy path and a non-admin caller being rejected.

**Technical Notes:** Given issue #64's related concern about `update_fleet_treasury` having no timelock, consider whether the *admin* override path should have a timelock too, or whether its emergency nature justifies immediacy — this is a product decision worth capturing in `docs/GOVERNANCE.md` alongside the implementation.

**Relevant Files:** `contracts/fleet_management_contract/lib.rs:70-191`

**Testing Requirements:** As described in Acceptance Criteria.

**Definition of Done:** A lost or compromised fleet-owner key no longer permanently strands a fleet with no recourse.

**Suggested Labels:** `enhancement`, `security`

---

### 64. `update_fleet_treasury` has no timelock, cooldown, or driver notice

**Summary:** A fleet owner can change the treasury address that (once GH #12's routing fix lands) all of that fleet's active drivers' escrow payouts will flow to, instantly and with zero notice period, cooldown, or driver acknowledgment.

**Background:** `contracts/fleet_management_contract/lib.rs:166-191` (`update_fleet_treasury`) requires only the owner's signature and takes effect immediately — the very next `get_payout_address` call for any active driver in that fleet reflects the new treasury. There is no minimum time between treasury changes, no pending-change/confirmation window (unlike `escrow_contract`'s own two-step admin transfer pattern, ADR-005), and no event or mechanism that gives drivers advance warning before their future earnings are redirected.

**Problem Statement:** GH issue #12 already tracks the fact that fleet treasury routing isn't wired into the live payout path *yet* — but this issue is about the control surface that will govern that routing once it is wired, which is a distinct, forward-looking gap. Once fixed, a fleet owner (or an attacker who has compromised the owner's key) could redirect all future driver payouts for that fleet to an arbitrary address with a single signed transaction and no warning, and any driver actively working under that fleet has no on-chain way to know their next payout destination changed until after the fact.

**Proposed Solution:** Add a delay between proposing and taking effect for treasury changes (a "propose then activate after N ledgers" pattern, mirroring `escrow_contract`'s two-step admin transfer), and/or an event drivers' off-chain clients can watch specifically for treasury changes on fleets they belong to.

**Acceptance Criteria:**
- [ ] Treasury changes require either a timelock or an explicit two-step confirmation before taking effect on live payout routing.
- [ ] A dedicated event is emitted immediately on proposal (not just on activation), giving drivers the earliest possible signal.

**Technical Notes:** This should land no later than whenever GH #12 (fleet treasury routing wiring) is implemented, ideally in the same change, since shipping the routing fix without this control would introduce the exact risk this issue describes.

**Relevant Files:** `contracts/fleet_management_contract/lib.rs:166-191`

**Testing Requirements:** Tests for: a proposed treasury change not taking effect immediately; the change taking effect only after the timelock/confirmation step; the "old" treasury still receiving payouts during the delay window.

**Definition of Done:** Fleet treasury changes cannot silently and instantly redirect driver earnings the moment GH #12's routing wiring is live.

**Suggested Labels:** `enhancement`, `security`

---

### 65. No multi-signature support for fleet management despite being a named roadmap item

**Summary:** `PLAN.md` lists "Add multi-signature support for fleet management" as a Medium-High priority feature; `FleetProfile.owner` is a single `Address` with no concept of multiple co-owners or threshold approval for any fleet action.

**Background:** `PLAN.md:17`. `contracts/fleet_management_contract/lib.rs:36-42` defines `FleetProfile { fleet_id, owner: Address, treasury, total_active_drivers }` — a single owner, full stop. Every owner-gated function (`update_fleet_treasury`, `add_driver_to_fleet`, and `remove_driver_from_fleet`'s owner branch) checks equality against that one address.

**Problem Statement:** For the "Enterprise Logistics SMEs" this contract is explicitly designed to serve, requiring a single private key to authorize treasury changes and driver-roster changes is a substantial operational risk (single point of failure) and a real gap relative to what the roadmap already commits to delivering.

**Proposed Solution:** Introduce an M-of-N signer model for fleet-level actions — e.g., a `Vec<Address>` of authorized signers per fleet plus a threshold, with owner-gated functions instead requiring a quorum of `require_auth()` calls or a propose/approve/execute pattern.

**Acceptance Criteria:**
- [ ] A fleet can be configured with more than one authorized signer and a threshold.
- [ ] Owner-gated actions require the configured threshold of signatures rather than a single address.
- [ ] Existing single-owner fleets continue to work unchanged (threshold of 1, one signer) for backward compatibility.

**Technical Notes:** This is a substantial design change; consider scoping the first iteration to just `update_fleet_treasury` and `remove_driver_from_fleet` (the two most consequential actions) rather than every fleet function, to keep the initial implementation reviewable.

**Relevant Files:** `contracts/fleet_management_contract/lib.rs:36-42, 166-338`, `PLAN.md:17`

**Testing Requirements:** Tests for: single-signer fleets behaving exactly as today; multi-signer fleets requiring quorum before an action executes; a sub-threshold set of approvals being insufficient.

**Definition of Done:** Enterprise fleet owners can configure multi-party authorization for consequential fleet actions, closing the gap with `PLAN.md`'s stated roadmap.

**Suggested Labels:** `feature`

---

### 66. No dynamic, volume-based fee adjustment despite being a named roadmap item

**Summary:** `PLAN.md` lists "Implement dynamic fee adjustment based on delivery volume" as a Medium-High priority feature; `platform_fee_bps` today is a single, flat, manually-set value with no concept of volume tiers.

**Background:** `PLAN.md:21`. `contracts/escrow_contract/lib.rs:184-208` (`update_platform_fee`) sets one global `platform_fee_bps` value applied identically to every escrow regardless of sender, recipient, or historical delivery volume. There is no per-user or per-tier fee schedule anywhere in the contract.

**Problem Statement:** High-volume senders (e.g., the enterprise customers `fleet_management_contract` is built for) and one-off individual senders currently pay an identical fee rate, with no mechanism to reward volume the way the roadmap envisions. This is purely a missing feature relative to stated plans, not a bug in the current flat-fee behavior.

**Proposed Solution:** Add a volume-tracking mechanism (e.g., leveraging the `deliveries_completed` counter concept already present for drivers, extended to senders) and a tiered fee schedule that `calculate_fee` consults based on the sender's historical volume, with admin-configurable tier thresholds.

**Acceptance Criteria:**
- [ ] Fee calculation can vary based on a sender's tracked delivery volume.
- [ ] Tier thresholds and rates are admin-configurable, not hardcoded.
- [ ] Existing flat-fee behavior remains the default/fallback for senders below the lowest volume tier, preserving backward compatibility.

**Technical Notes:** This will require tracking per-sender volume somewhere — likely a new `DataKey::SenderVolume(Address) -> u32` in `escrow_contract`, incremented on each successful `release_escrow`.

**Relevant Files:** `contracts/escrow_contract/lib.rs:52-54, 184-208`, `PLAN.md:21`

**Testing Requirements:** Tests for: fee rate changing correctly as a sender crosses volume thresholds; admin updates to tier configuration taking effect; existing flat-fee tests continuing to pass for senders below any tier.

**Definition of Done:** Platform fees can vary by sender volume, per an admin-configurable schedule, closing the gap with `PLAN.md`'s stated roadmap.

**Suggested Labels:** `feature`

---

### 67. No recovery mechanism for tokens sent directly to `escrow_contract` outside `create_escrow`

**Summary:** If a user mistakenly sends tokens directly to `escrow_contract`'s address via a plain token transfer (bypassing `create_escrow` entirely), those funds become permanently unrecoverable — no function in the contract can sweep balance that isn't tied to a tracked `EscrowRecord`.

**Background:** Every fund-moving function in `contracts/escrow_contract/lib.rs` (`release_escrow`, `refund_escrow`, `resolve_dispute`, `resolve_dispute_split`) operates strictly in terms of a specific `delivery_id`'s `EscrowRecord.amount` — none of them, nor any other function in the contract, references the contract's *total* token balance independent of tracked records. A user who sends tokens directly to the contract address (a common real-world mistake, especially for exchange-style wallets that let users paste any contract address as a "recipient") funds a balance the contract has no code path to ever release.

**Problem Statement:** This is a permanent, admin-unrecoverable loss of user funds for a plausible, foreseeable user error — not a hypothetical edge case, but the same class of "stray transfer" mistake that essentially every token-holding contract on every chain has to guard against. `docs/SECURITY_AUDIT.md`'s own Denial-of-Service section (10) asks "No stuck funds scenarios" under Financial Operations (section 3) — this is precisely such a scenario, and it is currently unmitigated.

**Proposed Solution:** Add an admin-gated `sweep_untracked_balance(admin, token, recipient)` function that computes `token_balance - sum(all tracked EscrowRecord.amount for that token)` and allows the admin to recover only the untracked excess, never touching funds backing a live escrow.

**Acceptance Criteria:**
- [ ] An admin can recover token balance sent directly to the contract outside `create_escrow`.
- [ ] The sweep function cannot be used to withdraw any balance backing an active, tracked `EscrowRecord`.
- [ ] A test funds the contract via a direct transfer (not `create_escrow`) and confirms the admin can recover exactly that amount, no more.

**Technical Notes:** Computing "untracked balance" requires either maintaining a running total of tracked escrow amounts per token (simplest: a `DataKey::TotalLocked(Address) -> i128` counter updated alongside every escrow state change) or iterating all records (impractical at scale) — the counter approach also directly enables issue #68 (on-chain TVL view).

**Relevant Files:** `contracts/escrow_contract/lib.rs` (all fund-moving functions), `docs/SECURITY_AUDIT.md:50-56`

**Testing Requirements:** As described in Acceptance Criteria; also test that the sweep amount is exactly zero when the contract's balance is fully accounted for by tracked escrows.

**Definition of Done:** Tokens mistakenly sent directly to `escrow_contract` are recoverable by the admin without ever putting a live escrow's committed funds at risk.

**Suggested Labels:** `feature`, `security`

---

### 68. No on-chain aggregate TVL view despite `MONITORING.md` naming it a key metric

**Summary:** `docs/MONITORING.md` lists "Total Value Locked (TVL): Sum of all escrows" as a Financial Metric operators are expected to track, but no contract exposes any aggregate value — every TVL calculation must be done entirely off-chain by replaying every `escrow_funded`/`escrow_released`/`escrow_refunded` event from genesis.

**Background:** `docs/MONITORING.md:18-19`. `contracts/escrow_contract/lib.rs` has no `DataKey` or query function that tracks a running total across all escrows — `get_escrow(delivery_id)` only ever returns a single record.

**Problem Statement:** This forces every consumer of the documented TVL metric — dashboards, alerting on the "TVL drops > 20% in 1 hour" critical alert `docs/MONITORING.md:66` itself defines — to build and maintain their own off-chain event-replay indexer just to answer a question the contract could answer directly and authoritatively with a single stored counter. This also compounds with issue #67's need for a per-token tracked-total counter, since the same underlying data structure serves both purposes.

**Proposed Solution:** Add a `DataKey::TotalLocked(Address /* token */) -> i128` counter, incremented on `create_escrow` and decremented on `release_escrow`/`refund_escrow`/dispute resolutions, with a public `get_total_locked(token) -> i128` query.

**Acceptance Criteria:**
- [ ] `escrow_contract` exposes a query returning the current aggregate locked value for a given token.
- [ ] The counter stays accurate across the full escrow lifecycle (create, release, refund, dispute resolution paths including split).

**Technical Notes:** Share the implementation with issue #67's tracked-total requirement rather than building two separate counters.

**Relevant Files:** `contracts/escrow_contract/lib.rs`, `docs/MONITORING.md:18-19, 61-67`

**Testing Requirements:** Tests confirming the aggregate value increases on `create_escrow`, decreases correctly on every release/refund/dispute-resolution path (including partial amounts from `resolve_dispute_split`), and never goes negative.

**Definition of Done:** TVL is queryable directly from `escrow_contract` without requiring off-chain event replay.

**Suggested Labels:** `feature`

---

### 69. `register_user`/`UserProfile` are fully implemented but never consumed anywhere

**Summary:** `identity_reputation_contract::register_user` and `get_user_profile` are complete, working functions with their own storage key and event — but nothing in the protocol ever calls them, links a `UserProfile` to a delivery's `sender`/`recipient`, or reads one back for any purpose.

**Background:** `contracts/identity_reputation_contract/lib.rs:130-162` implements `register_user`/`get_user_profile` fully, including duplicate-registration protection and an event emission. No other contract in the workspace calls `register_user`, and `contracts/identity_reputation_contract/test.rs` has zero tests exercising either function (confirmed by search — the only test coverage in that file is for driver registration, KYC, and reputation scoring).

**Problem Statement:** This is a fully-built, seemingly-intentional feature (it has its own event, its own duplicate-guard, and a query function — not a stub) with no integration anywhere and no test coverage, making its actual purpose in the protocol unclear. Either it's meant to track senders/recipients (in which case `create_delivery` should be registering/checking users) and that wiring was never completed, or it's vestigial from an earlier design and should be removed.

**Proposed Solution:** Determine intent: if senders/recipients are meant to have on-chain profiles, wire `create_delivery` (or an onboarding flow) to call `register_user`, and use `UserProfile` data for something concrete (e.g., a "new user" flag for UX purposes, or a KYC-adjacent check mirroring `DriverProfile.kyc_verified`). If there's no near-term use, remove the function and its storage key as dead code, consistent with the resolution direction for issue #36.

**Acceptance Criteria:**
- [ ] `register_user`/`UserProfile` either gain at least one real caller/consumer in the protocol, or are removed.
- [ ] Whichever direction is chosen, `contracts/identity_reputation_contract/test.rs` has test coverage matching the final decision (either testing real integration, or confirming clean removal via `cargo build`/`cargo test`).

**Technical Notes:** This should be resolved together with issue #35 (the duplicate `UserProfile` type), since fixing one without considering the other risks solving a type-consistency problem for a feature that gets deleted immediately after.

**Relevant Files:** `contracts/identity_reputation_contract/lib.rs:9-12, 130-162`, `contracts/identity_reputation_contract/test.rs`

**Testing Requirements:** As described in Acceptance Criteria.

**Definition of Done:** `register_user`/`UserProfile` have a clear, tested purpose in the protocol, or no longer exist.

**Suggested Labels:** `enhancement`

---

### 70. No way to enumerate current admins of `dispute_resolution_contract`

**Summary:** `dispute_resolution_contract`'s multi-admin model (`DataKey::Admin(Address) -> bool`) supports an open-ended set of admins, but there is no function to list them — `is_admin(candidate)` requires already knowing a specific address to check.

**Background:** `contracts/dispute_resolution_contract/lib.rs:69-92` implements `add_admin`/`remove_admin`/`is_admin`, all keyed per-address with no accompanying roster/count. Unlike `escrow_contract`'s single-admin model (where `get_admin()` always answers "who is in charge" directly), this contract's governance surface is opaque from the outside — the only way to know "who currently has admin rights" is to have independently tracked every `add_admin`/`remove_admin` call from contract genesis.

**Problem Statement:** For a contract whose admins can unilaterally resolve disputes and move funds (`resolve_dispute_refund_sender`, `resolve_dispute_pay_driver`, `resolve_dispute_split_funds`), the inability to cheaply audit "who currently holds that power" on-chain is a real governance-transparency gap — exactly the kind of accountability `docs/GOVERNANCE.md`'s "Action Logs"/"Audit Trail" sections describe as important, but which currently requires off-chain event replay to reconstruct rather than a direct on-chain read.

**Proposed Solution:** Maintain a `Vec<Address>` admin roster alongside the existing per-address boolean map (updated in lockstep by `add_admin`/`remove_admin`), and expose a `list_admins() -> Vec<Address>` query.

**Acceptance Criteria:**
- [ ] A query function returns the complete current admin roster.
- [ ] The roster stays accurate through add/remove cycles, including the last-admin protection from issue #34.

**Technical Notes:** Since issue #34 already proposes tracking an admin *count* to prevent removing the last admin, this issue's roster can piggyback on the same data-structure change rather than requiring a separate implementation effort.

**Relevant Files:** `contracts/dispute_resolution_contract/lib.rs:69-92`

**Testing Requirements:** Tests confirming the roster reflects additions and removals accurately, including after several add/remove cycles.

**Definition of Done:** Anyone can directly query the complete, current admin set of `dispute_resolution_contract` on-chain.

**Suggested Labels:** `feature`

---

### 71. Admin/governance model is reinvented three different ways across the six contracts

**Summary:** `escrow_contract`/`delivery_contract`/`fleet_management_contract`/`identity_reputation_contract` each use a single `Admin: Address` instance key with no rotation-in-place multi-admin support, while `dispute_resolution_contract` alone uses an open-ended `Admin(Address) -> bool` multi-admin map — two structurally incompatible governance models coexist in one protocol with no shared abstraction.

**Background:** Confirmed by direct inspection: `escrow_contract/lib.rs:163` (`StorageKey::Admin` single address, with `propose_admin`/`accept_admin` for rotation), `delivery_contract/lib.rs:64` (same single-address pattern, no rotation mechanism at all), `fleet_management_contract/lib.rs:74` (single address, no rotation), `identity_reputation_contract/lib.rs:55,68` (single address, no rotation, and two different init entry points besides — tracked separately as GH #10). `dispute_resolution_contract/lib.rs:33-40,66` uses `DataKey::Admin(Address) -> bool`, an entirely different shape supporting an arbitrary number of admins with no owner-of-record at all.

**Problem Statement:** This isn't just cosmetic inconsistency — it means the *security properties* of "who controls this contract" differ meaningfully depending which of the six contracts you're looking at: four contracts have exactly one admin with a secure two-step-or-nothing rotation story (and no rotation at all for three of the four), while the fifth has an unbounded admin set with the single-point-of-failure risk from issue #34 (last-admin removal) that the single-admin contracts structurally cannot have (since none of them support *removing* the sole admin without a replacement already being designated via `propose_admin`/`accept_admin` — except `delivery_contract` and `fleet_management_contract`, which have no rotation mechanism at all and would require a full migration to ever change admins). A protocol-wide governance model, even a simple one, would let every contract share the same well-understood security properties instead of six-going-on-two independently-reasoned-about designs.

**Proposed Solution:** Design one shared governance primitive in `shared_types` (e.g., a multi-admin set with a minimum-count floor and a consistent propose/accept rotation pattern) and migrate all six contracts onto it, documented as a new ADR.

**Acceptance Criteria:**
- [ ] A single, shared admin/governance abstraction exists in `shared_types`.
- [ ] All six contracts use it, with consistent security properties (rotation mechanism, minimum-admin-count floor) across the board.
- [ ] The design is documented as a new entry in `docs/ARCHITECTURE_DECISION_RECORDS.md`.

**Technical Notes:** This is a large, cross-cutting refactor best sequenced after issue #34 (last-admin protection) and issue #62 (duplicated `is_admin` helper) land individually, since both are natural building blocks toward this consolidation rather than competing with it.

**Relevant Files:** `contracts/escrow_contract/lib.rs:159-289`, `contracts/delivery_contract/lib.rs:60-76`, `contracts/fleet_management_contract/lib.rs:70-96`, `contracts/identity_reputation_contract/lib.rs:51-75`, `contracts/dispute_resolution_contract/lib.rs:33-92`

**Testing Requirements:** Full regression test suite across all six contracts after migration, plus new tests for the shared abstraction itself in `shared_types`.

**Definition of Done:** Every contract in the workspace uses the same governance primitive with the same, well-understood security guarantees.

**Suggested Labels:** `refactor`

---

### 72. `docs/API.md` documents 30+ functions but shows a usage example for exactly one

**Summary:** Of every function documented across `docs/API.md`'s Escrow and Delivery sections, only `init` (`:32-39`) has an actual code example — every other function (roughly 20 across the two documented contracts, before even counting the four undocumented ones from issue #58) has a parameter/error list but no example call.

**Background:** `docs/API.md:16-392` documents `init`, `update_platform_fee`, `propose_admin`, `accept_admin`, `set_settlement_contract`, `create_escrow`, `release_escrow`, `refund_escrow`, `raise_dispute`, `resolve_dispute`, `resolve_dispute_split`, six query functions, `create_delivery`, `assign_driver`, `mark_in_transit`, `confirm_delivery`, `cancel_delivery`, `raise_dispute` (delivery), and two more query functions. Exactly one of these (`init`) includes a fenced code block showing how to actually call it.

**Problem Statement:** `PLAN.md:27` explicitly lists "Build interactive API examples using Stellar SDK" as a High-priority documentation task — this gap is a direct, measurable instance of that unaddressed roadmap item. A reference doc with parameter lists but almost no call examples is significantly harder to integrate against than one with worked examples for every state-changing function, especially for functions with several parameters and multiple error cases (e.g., `resolve_dispute_split`'s three parameters and three documented error cases have no example showing correct argument order or types).

**Proposed Solution:** Add a minimal Rust (or TypeScript, matching the doc's existing SDK section) usage example for every state-changing function currently missing one, following the exact style already used for `init`.

**Acceptance Criteria:**
- [ ] Every state-changing function documented in `docs/API.md` has at least one accompanying usage example.

**Technical Notes:** This can be done incrementally per-contract; combining with the section rewrite from issue #58 (documenting the four missing contracts) is a natural place to establish "every function gets an example" as the doc's baseline standard going forward.

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

**Problem Statement:** Every integrator today must hand-roll their own typed bindings against the raw Stellar SDK, re-deriving each contract's parameter order, types, and error codes from `docs/API.md` (itself incomplete per issue #58) rather than importing a maintained package — exactly the friction a dedicated SDK wrapper is meant to remove, and exactly what the roadmap already commits to delivering.

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

**Problem Statement:** The upgrade guide presents a specific, concrete recommendation as though it's an established pattern in this codebase, but it is purely illustrative/aspirational — there is no working example anywhere to base a real migration on, and no tooling exists to help generate or validate one when the time comes. Given several issues in this backlog (e.g., #25's `UserProfile` consolidation, #61's governance-model unification) would themselves require exactly this kind of state migration to land safely on a live deployment, the absence of any real, tested migration pattern is a concrete blocker for shipping those fixes without a full redeploy-and-data-loss cycle.

**Proposed Solution:** Implement one real, tested migration function for an actual pending change already identified in this backlog (e.g., the `UserProfile` field rename from issue #35) as a template others can follow, and document any lessons learned back into `docs/UPGRADE_GUIDE.md`.

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

**Proposed Solution:** Add `escrow_contract` (and `identity_reputation_contract`, needed for issue #33's fix) as dev-dependencies of `fleet_management_contract`, and write at least one true end-to-end test: register a fleet, add and activate a driver, create and release an escrow for that driver, and assert the funds actually land in the fleet's treasury (once GitHub #12 is fixed) rather than the driver's own address.

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

**Definition of Done:** Every non-trivial public struct in `shared_types` that isn't already flagged as dead code (issue #36) has direct field-level test coverage at the `shared_types` level.

**Suggested Labels:** `test`

---

### 80. `CHANGELOG.md`'s `[Unreleased]` section is stale relative to the completed SDK 27 migration

**Summary:** `CHANGELOG.md`'s `[Unreleased] > Changed` section reads "Updated Soroban SDK to 22.0.1" — but the workspace has since fully migrated to SDK 27.0.0 (a separate, dedicated migration document exists for it), and the `[Unreleased]` section was never updated or cut into a new version to reflect that, the CI toolchain pin, or the `wasm32v1-none` target change.

**Background:** `CHANGELOG.md:8-38` is the `[Unreleased]` section, whose "Changed" subsection (`:28-32`) still lists "Updated Soroban SDK to 22.0.1" as the current changelog entry for the SDK version. Meanwhile, `SOROBAN_SDK_27_MIGRATION.md` documents a completed migration to SDK 27.0.0 dated 2026-07-14, and recent commit history (`6944bd4 Migrate to Soroban SDK 27.0.0 with wasm32v1-none target`, `38f7c38 ci: update Rust toolchain to stable`, `1e113ea fix: pin Rust to 1.81.0 in CI workflows for compatibility`) shows substantial toolchain work has landed since that changelog entry was written, none of which is reflected anywhere in `CHANGELOG.md`.

**Problem Statement:** `CHANGELOG.md`'s own documented "Release Process" (`:63-69`) describes `[Unreleased]` as the staging area that should be updated with changes and then cut into a versioned release — but real, substantial changes (an entire major SDK migration, a WASM target change, multiple CI toolchain pins) have landed without ever touching this file, leaving it actively wrong about the current SDK version rather than merely incomplete. This is the same underlying fact as issue #59 (`docs/API.md`'s stale SDK version), but a distinct process failure: a changelog is supposed to be updated as part of every notable change's own workflow, not corrected after the fact alongside unrelated documentation.

**Proposed Solution:** Update `CHANGELOG.md`'s `[Unreleased]` section to accurately reflect the SDK 27.0.0 migration, the `wasm32v1-none` target change, and the Rust toolchain pinning work, and consider whether this backlog of accumulated `[Unreleased]` changes should finally be cut into a real version per the document's own documented release process.

**Acceptance Criteria:**
- [ ] `CHANGELOG.md` accurately reflects the current SDK version and the toolchain/target changes that have landed since the `[Unreleased]` section was last touched.

**Technical Notes:** Fix this alongside issue #59 so both stale "22.0.1" references are corrected in the same pass, but keep them as separately tracked issues since they represent different underlying failures (a stray reference doc line vs. a changelog process gap).

**Relevant Files:** `CHANGELOG.md:8-38`, `SOROBAN_SDK_27_MIGRATION.md`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** `CHANGELOG.md`'s `[Unreleased]` section accurately reflects every notable change that has landed since it was last updated.

**Suggested Labels:** `documentation`

---

## Summary by Label

| Label | Count | Issues |
|---|---|---|
| `bug` | 12 | 31, 32, 33, 34, 35, 37, 39, 50, 51, 52, 53, 54 |
| `security` | 7 | 32, 34, 37, 43, 63, 64, 67 |
| `feature` | 10 | 38, 42, 44, 65, 66, 67, 68, 70, 74, 75 |
| `enhancement` | 5 | 57, 63, 64, 69, 77 |
| `refactor` | 6 | 36, 40, 41, 62, 71, 76 |
| `test` | 7 | 45, 46, 47, 48, 49, 78, 79 |
| `documentation` | 7 | 58, 59, 60, 61, 72, 73, 80 |
| `performance` | 3 | 43, 55, 56 |

(Several issues carry two labels — e.g. `bug`+`security` or `feature`+`security` — and are counted in both rows above. Issues #11–#30 were filed to GitHub in a later pass — see `Pushed to GitHub` above — and no longer appear here.)

## Summary by Contract

| Contract / Area | Issues in this doc | Filed on GitHub |
|---|---|---|
| `escrow_contract` | 31, 32, 48, 49, 55, 62, 67, 68, 76 | #7, #11, #12, #13, #14, #15, #16, #17, #18, #25, #26, #31 |
| `delivery_contract` | 38, 39, 62 | #19, #20, #23, #24, #27, #33 |
| `dispute_resolution_contract` | 34, 43, 44, 45, 70, 71 | #8, #21, #22, #32 |
| `identity_reputation_contract` | 35, 37, 38, 42, 69 | #9, #10, #24 |
| `fleet_management_contract` | 33, 63, 64, 65, 77, 78 | #12, #26, #27, #28 |
| `settlement_contract` | 49 | #15, #30, #35 |
| `shared_types` | 35, 36, 40, 41, 79 | #24, #26, #33 |
| Docs (`docs/`, root `*.md`) | 55, 58, 59, 60, 61, 72, 73, 75, 80 | — |
| CI/CD (`.github/workflows/`) | 50, 51, 56, 57 | — |
| Scripts/tooling (`scripts/`, `Makefile`, `.env.example`) | 50, 51, 52, 53, 54 | — |
| Cross-cutting / process | 40, 41, 46, 47, 71, 74 | #7, #8, #19, #27, #31, #34, #36 |

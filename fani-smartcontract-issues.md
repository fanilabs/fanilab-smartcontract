# FaniLab Smart Contracts — Substantive Issues

Derived from a direct read of every contract in `contracts/` (escrow_contract,
delivery_contract, dispute_resolution_contract, fleet_management_contract,
identity_reputation_contract, settlement_contract, shared_types) plus the
project's own `PLAN.md`, `PRODUCTION_READINESS.md`, `Cargo.toml`, and CI
workflow. Every issue below references the specific function and file it was
found in — none are generic placeholders.

This document consolidates three review passes: an initial pass (all 30 of whose issues — 6 Critical, 4 High, and local issues #11–#30 — have since been filed to GitHub, see below), a follow-up pass extending coverage to cross-contract architecture, testing gaps, CI/CD, deployment tooling, and documentation accuracy (issues #31–#80, all of which have since been filed to GitHub, see below), and a third pass covering reentrancy/authorization-bypass risk, further cross-contract consistency gaps, and repository-hygiene/documentation drift (issues #81–#130, of which #81–#110 have since also been filed to GitHub, see below; the remaining 20 issues are still tracked in this document, renumbered #91–#110).

## Pushed to GitHub

110 issues have been filed on `github.com/fanilabs/fanilab-smartcontract` and removed from this document to avoid duplication: the original 10 highest-severity findings (6 Critical + 4 High), the full remaining High/Medium/Low-classified backlog from the initial review pass (local issues #11–#30), the follow-up pass's architecture/testing/tooling findings (local issues #31–#80), and the third pass's findings covering reentrancy/authorization-bypass risk, cross-contract consistency, and further correctness/testing/documentation gaps (local issues #81–#110). Track them there:

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
| [#77](https://github.com/fanilabs/fanilab-smartcontract/issues/77) | Admin/governance model is reinvented three different ways across the six contracts |
| [#78](https://github.com/fanilabs/fanilab-smartcontract/issues/78) | `docs/API.md` documents 30+ functions but shows a usage example for exactly one |
| [#79](https://github.com/fanilabs/fanilab-smartcontract/issues/79) | No translated documentation despite being an explicit, prioritized roadmap item |
| [#80](https://github.com/fanilabs/fanilab-smartcontract/issues/80) | No SDK wrapper package despite being a named roadmap item and already-documented usage snippets |
| [#81](https://github.com/fanilabs/fanilab-smartcontract/issues/81) | No contract migration tooling despite `UPGRADE_GUIDE.md` documenting a `migrate_to_vN` pattern |
| [#82](https://github.com/fanilabs/fanilab-smartcontract/issues/82) | Fee-calculation-and-payout logic is triplicated across three escrow functions |
| [#83](https://github.com/fanilabs/fanilab-smartcontract/issues/83) | `DriverFleetStatus` has no terminal "removed" state, destroying fleet membership history |
| [#84](https://github.com/fanilabs/fanilab-smartcontract/issues/84) | No integration test scaffolding between `fleet_management_contract` and `escrow_contract` |
| [#85](https://github.com/fanilabs/fanilab-smartcontract/issues/85) | Core wire-format types in `shared_types` have no dedicated equality/round-trip tests |
| [#86](https://github.com/fanilabs/fanilab-smartcontract/issues/86) | `CHANGELOG.md`'s `[Unreleased]` section is stale relative to the completed SDK 27 migration |
| [#87](https://github.com/fanilabs/fanilab-smartcontract/issues/87) | `escrow_contract`'s fund-moving functions update state after transfers, contradicting the documented checks-effects-interactions pattern |
| [#88](https://github.com/fanilabs/fanilab-smartcontract/issues/88) | `resolve_dispute`/`resolve_dispute_split` emit a useless duplicated-caller event payload and never emit `escrow_released`/`escrow_refunded` |
| [#89](https://github.com/fanilabs/fanilab-smartcontract/issues/89) | `escrow_contract::propose_admin`/`accept_admin` use raw `panic!` instead of the contract's own typed errors |
| [#90](https://github.com/fanilabs/fanilab-smartcontract/issues/90) | `escrow_contract` has no way to unset a previously configured `settlement_contract` |
| [#91](https://github.com/fanilabs/fanilab-smartcontract/issues/91) | `escrow_contract::set_settlement_contract`/`get_settlement_contract` have zero test coverage |
| [#92](https://github.com/fanilabs/fanilab-smartcontract/issues/92) | `escrow_contract/test.rs` has no direct test for `resolve_dispute` or `resolve_dispute_split` |
| [#93](https://github.com/fanilabs/fanilab-smartcontract/issues/93) | A sender can bypass admin-mediated dispute resolution entirely by cancelling a `Disputed` delivery, forcing a full self-refund before an admin ever rules on the case |
| [#94](https://github.com/fanilabs/fanilab-smartcontract/issues/94) | Raising a dispute against a `Delivered` delivery never transitions `delivery_contract`'s status to `Disputed` |
| [#95](https://github.com/fanilabs/fanilab-smartcontract/issues/95) | `delivery_contract/test.rs` never verifies state rollback when a cross-contract escrow call fails |
| [#96](https://github.com/fanilabs/fanilab-smartcontract/issues/96) | `create_delivery` accepts empty origin/destination strings and zero-weight cargo with no minimum-content validation |
| [#97](https://github.com/fanilabs/fanilab-smartcontract/issues/97) | No way to amend delivery metadata after creation |
| [#98](https://github.com/fanilabs/fanilab-smartcontract/issues/98) | `DeliveryMetadata.estimated_delivery` is stored but never read, validated, or enforced anywhere |
| [#99](https://github.com/fanilabs/fanilab-smartcontract/issues/99) | `resolve_dispute_split_funds` and `resolve_dispute_pay_driver` never adjust driver reputation, unlike `resolve_dispute_refund_sender` |
| [#100](https://github.com/fanilabs/fanilab-smartcontract/issues/100) | Drivers are structurally excluded from the entire dispute process |
| [#101](https://github.com/fanilabs/fanilab-smartcontract/issues/101) | `DisputeCase` has no `resolved_at`/`resolved_by` fields, leaving dispute resolution unauditable on-chain |
| [#102](https://github.com/fanilabs/fanilab-smartcontract/issues/102) | No forced-resolution deadline or escalation path for an `Open` dispute |
| [#103](https://github.com/fanilabs/fanilab-smartcontract/issues/103) | `dispute_resolution_contract::resolve_dispute_split_funds` has no unauthorized-caller test |
| [#104](https://github.com/fanilabs/fanilab-smartcontract/issues/104) | `identity_reputation_contract` has no admin setter to repoint `delivery_contract`/`dispute_contract` after `initialize()` |
| [#105](https://github.com/fanilabs/fanilab-smartcontract/issues/105) | Reputation-scoring point values are hardcoded magic numbers with no admin configurability |
| [#106](https://github.com/fanilabs/fanilab-smartcontract/issues/106) | `DriverTier`'s Gold threshold and `ENTERPRISE_THRESHOLD` are independently hardcoded as the same magic number in two different places |
| [#107](https://github.com/fanilabs/fanilab-smartcontract/issues/107) | `identity_reputation_contract`: `is_eligible_for_enterprise`, `set_authorized_contract`, `is_authorized_contract`, and `get_admin` are all untested |
| [#108](https://github.com/fanilabs/fanilab-smartcontract/issues/108) | No way to deactivate or close a registered fleet |
| [#109](https://github.com/fanilabs/fanilab-smartcontract/issues/109) | No way for a fleet owner to rescind a pending driver invite before acceptance |
| [#110](https://github.com/fanilabs/fanilab-smartcontract/issues/110) | `fleet_management_contract::set_identity_contract` has zero test coverage |
| [#111](https://github.com/fanilabs/fanilab-smartcontract/issues/111) | No unified error-code reference table across six independently-numbered `contracterror` enums |
| [#112](https://github.com/fanilabs/fanilab-smartcontract/issues/112) | `FaniLabError::EscrowLocked` and `InvalidAddress` are dead error variants never returned by any contract |
| [#113](https://github.com/fanilabs/fanilab-smartcontract/issues/113) | `shared_types::DriverProfile` and `UserProfile` are declared after the `#[cfg(test)]` module and have zero dedicated unit tests |
| [#114](https://github.com/fanilabs/fanilab-smartcontract/issues/114) | All six contract crates blanket-suppress `#![allow(deprecated)]` for `events().publish()` instead of tracking migration to its replacement |
| [#115](https://github.com/fanilabs/fanilab-smartcontract/issues/115) | TTL pair `518400, 518400` is duplicated as inline magic numbers at ~25 call sites across four contracts with no shared constant |
| [#116](https://github.com/fanilabs/fanilab-smartcontract/issues/116) | `settlement_contract` is the only one of six contracts using Rust's standard `src/lib.rs` layout; the other five override to a flat `lib.rs` |

The remaining 20 issues below (#91–#110) are not yet filed.

---

## Index

| # | Title | Labels |
|---|---|---|
| 91 | `Cargo.toml`'s `release-with-logs` build profile is entirely unused | refactor |
| 92 | `README.md`/`CONTRIBUTING.md` describe a fictional 3-crate repository layout | documentation |
| 93 | `README.md`'s CI/coverage badges and org link point to a nonexistent GitHub org/repo | documentation |
| 94 | `README.md`/`SECURITY.md` claim version 0.2.x while every `Cargo.toml` still says 0.1.0 | documentation |
| 95 | `docs/DEPLOYMENT.md` documents a phantom `update_escrow_contract` function and phantom test infrastructure | documentation |
| 96 | Contributor docs and `dependabot.yml` reference GitHub labels that don't exist or don't match | documentation |
| 97 | `smart-contract-architecture.md` documents a phantom `RoleType` enum and a phantom `PickedUp` status | documentation |
| 98 | `docs/SECURITY_AUDIT.md` prescribes a test-naming convention zero tests actually use | test |
| 99 | `scripts/deploy-contract.sh`/`initialize-contract.sh` are committed empty, breaking documented flows | bug |
| 100 | Leftover `SwiftChainError` test comments and phantom `.gitignore` script paths | refactor |
| 101 | Repository root contains leftover developer debris (`test_script.py`, `tests_passing.png`) | refactor |
| 102 | `.vscode/settings.json` pins the stale `wasm32-unknown-unknown` target | enhancement |
| 103 | All CI workflows pin `dtolnay/rust-toolchain@stable`, a mutable branch reference | security |
| 104 | Several workflows pin deprecated GitHub Actions versions (`upload-artifact@v3`, `codecov-action@v3`) | performance |
| 105 | `deploy-testnet.yml`'s artifact-upload patterns never match the real output filename | bug |
| 106 | `deploy-all-contracts.sh`'s `deploy_contract()` captures echo output into `$contract_id`, corrupting JSON | bug |
| 107 | `ci.yml`'s outdated-dependency check has `continue-on-error: true`, defeating its own `--exit-code 1` | performance |
| 108 | `security-audit.yml` only runs `cargo deny check advisories`, never enforcing license/ban rules | security |
| 109 | `release.yml` builds and publishes a GitHub Release without ever running the test suite | bug |
| 110 | No CI step measures or enforces the 64 KB WASM contract-size limit | performance |

---

## Additional Findings — Cross-Contract Security, Correctness & Repository Hygiene (Issues #91–#110)

A third review pass covering reentrancy and authorization-bypass risks in the fund-moving paths, cross-contract state-consistency gaps, dead/unused configuration, and documentation that has drifted from the actual codebase — building on the initial pass (GitHub #7–#36), the follow-up pass (GitHub #37–#76), and this third pass's own earlier batches (GitHub #77–#116, formerly local issues #81–#110), without duplicating any of those findings.

---

### 91. `Cargo.toml`'s `release-with-logs` build profile is entirely unused, dead configuration

**Summary:** The workspace `Cargo.toml` defines a `[profile.release-with-logs]` profile (inheriting `release` with `debug-assertions = true`), but no script, Makefile target, or CI workflow anywhere in the repository ever builds with `--profile release-with-logs`.

**Background:** `Cargo.toml:20-22`. A repository-wide search for `release-with-logs` across every `.sh`, `.yml`, and `Makefile*` file returns zero matches outside the profile's own definition.

**Problem Statement:** This profile was evidently added for some debugging workflow (its name and `debug-assertions = true` setting both suggest a "build the release profile but keep assertions for troubleshooting" use case) but nothing in the committed tooling ever invokes it — a contributor discovering it has no documented way to know what it's for or how it was meant to be used.

**Why it Matters:** Dead configuration like this accumulates exactly the kind of "why does this exist" uncertainty that makes a codebase harder to trust as fully intentional — a smaller-scale instance of the same class of gap already flagged for empty doc files (GitHub #60) and phantom directories (see the README/CONTRIBUTING findings in this batch).

**Proposed Solution:** Either wire the profile into an actual documented use case (e.g., a `make build-debug` target, or a troubleshooting section in `docs/DEPLOYMENT.md`), or remove it if it no longer serves any purpose.

**Acceptance Criteria:**
- [ ] `release-with-logs` is either referenced by at least one script/Makefile target/workflow with accompanying documentation, or removed from `Cargo.toml`.

**Technical Notes:** This is a small, low-risk cleanup with no impact on the actual `release` profile the rest of the tooling uses.

**Relevant Files:** `Cargo.toml:20-22`

**Testing Requirements:** N/A (build configuration only); confirm `cargo build --release` is unaffected if the profile is removed.

**Definition of Done:** `Cargo.toml` contains no build profile that nothing in the repository ever uses.

**Suggested Labels:** `refactor`

---
### 92. `README.md`'s Repository Structure diagram and `CONTRIBUTING.md`'s crate overview both describe a fictional 3-crate layout

**Summary:** `README.md`'s "Repository Structure" tree shows only `escrow_contract/`, `delivery_contract/`, and `shared_types/` under `contracts/`, plus entirely nonexistent `src/{events,errors,storage,interfaces}/`, `tests/{integration_tests,contract_tests}/`, and `deploy/{testnet,mainnet}/` directories. `CONTRIBUTING.md` independently describes "three main crates" (the same three) as if `dispute_resolution_contract`, `fleet_management_contract`, `identity_reputation_contract`, and `settlement_contract` don't exist.

**Background:** `README.md:165-212` shows the fabricated tree in full, including `.github/workflows/ci.yml` as the *only* workflow listed (the repository has four: `ci.yml`, `release.yml`, `security-audit.yml`, `deploy-testnet.yml`). `CONTRIBUTING.md:20-26`: "The workspace consists of three main crates located in the `contracts/` directory: `shared_types/`... `escrow_contract/`... `delivery_contract/`." Neither document has been updated since the workspace grew to its current six-contract-plus-library shape.

**Problem Statement:** These are the two most likely entry points for a brand-new contributor, and both describe a workspace roughly half the size and complexity of the real one — anyone following README's directory tree to find, say, `fleet_management_contract` would conclude it doesn't exist; anyone following CONTRIBUTING's "three main crates" framing would have no idea `dispute_resolution_contract`'s cross-contract test dependencies (already a pattern GitHub #84 wants extended) are a thing they should be aware of.

**Why it Matters:** Unlike smaller, more contained doc-accuracy gaps elsewhere in this backlog, this is the first-impression documentation for the entire project — its inaccuracy compounds every other onboarding step that follows it.

**Proposed Solution:** Regenerate `README.md`'s Repository Structure section to match the actual `contracts/`, `docs/`, `scripts/`, and `.github/workflows/` layout, and update `CONTRIBUTING.md`'s crate overview to describe all six contracts plus `shared_types`.

**Acceptance Criteria:**
- [ ] `README.md`'s Repository Structure section lists the real six contracts, the real `docs/`/`scripts/`/`.github/workflows/` contents, and no directories that don't exist (`src/`, `tests/`, `deploy/`).
- [ ] `CONTRIBUTING.md` describes all six contracts, not three.

**Technical Notes:** Cross-check against GitHub #61/#67 (the "6 vs 7 contracts" count confusion in the architecture docs) while making this pass, since all three documents need to agree on the same real inventory.

**Relevant Files:** `README.md:165-212`, `CONTRIBUTING.md:20-26`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** `README.md` and `CONTRIBUTING.md` both accurately describe the workspace's real six-contract-plus-library structure.

**Suggested Labels:** `documentation`

---

### 93. `README.md`'s CI/coverage badges and GitHub org link point to a nonexistent `github.com/fanilab/FaniLab-SmartContract`

**Summary:** Every badge image and the "GitHub Organization" contact link in `README.md` points at `github.com/fanilab/FaniLab-SmartContract` — a different organization name (`fanilab` vs. the real `fanilabs`) and a different repository name/casing (`FaniLab-SmartContract` vs. the real `fanilab-smartcontract`) than the actual repository this file lives in.

**Background:** `README.md:319-320` (`![CI Status](https://github.com/fanilab/FaniLab-SmartContract/workflows/Rust%20CI/badge.svg)`, `![Security Audit](https://github.com/fanilab/FaniLab-SmartContract/workflows/Security%20Audit/badge.svg)`) and `:364` (`[FaniLab Organization](https://github.com/fanilab)`). The repository's actual identity, confirmed via `gh repo view`, is `fanilabs/fanilab-smartcontract`.

**Problem Statement:** Both status badges reference a workflow-status endpoint for an org/repo pair that doesn't exist, so they render as broken/404 images rather than live CI status — and the "GitHub Organization" link sends anyone who clicks it to either a nonexistent or unrelated `fanilab` account rather than the real `fanilabs` organization this project lives under.

**Why it Matters:** Status badges are meant to be the at-a-glance, trustworthy signal of project health that `PRODUCTION_READINESS.md` itself leans on ("Full CI/CD with security audits") — badges that can never resolve undermine exactly that signal for anyone landing on the README.

**Proposed Solution:** Update both badge URLs and the organization link to reference `fanilabs/fanilab-smartcontract`.

**Acceptance Criteria:**
- [ ] Both CI/coverage badges resolve to the real repository's actual workflow status.
- [ ] The GitHub organization link points to the real `fanilabs` org.

**Technical Notes:** The Codecov badge (`README.md:321`) has the same org/repo mismatch and should be corrected in the same pass.

**Relevant Files:** `README.md:319-321, 364`

**Testing Requirements:** N/A (documentation-only); manually verify each badge URL resolves after the fix.

**Definition of Done:** Every badge and GitHub link in `README.md` points to the real `fanilabs/fanilab-smartcontract` repository.

**Suggested Labels:** `documentation`

---

### 94. `README.md` and `SECURITY.md` claim version 0.2.x while every contract's `Cargo.toml` still declares `0.1.0`

**Summary:** `README.md`'s "Project Status" section states "Current Version: 0.2.0," and `SECURITY.md`'s supported-versions table lists `0.2.x` as the currently supported release — but every one of the seven crates in the workspace (`shared_types` plus all six contracts) declares `version = "0.1.0"` in its own `Cargo.toml`.

**Background:** `README.md:324` ("Current Version: 0.2.0"). `SECURITY.md:5-8` (`| 0.2.x | ✅ | | 0.1.x | ❌ |`). `grep`-confirmed: all seven `Cargo.toml` files under `contracts/` declare `version = "0.1.0"`, with no `0.2.0` anywhere in the workspace.

**Problem Statement:** These two documents jointly assert that version `0.1.x` is unsupported and `0.2.x` is current — but the actual, buildable version of every contract in the repository is `0.1.0`, meaning `SECURITY.md`'s own supported-versions table technically marks the only version that exists as unsupported.

**Why it Matters:** A version-support table that contradicts the code it's describing is worse than no table at all for anyone trying to determine whether to expect a security fix for their deployed version.

**Proposed Solution:** Either bump every `Cargo.toml`'s `version` field to `0.2.0` to match the documented claim, or correct `README.md`/`SECURITY.md` to reference `0.1.x`, whichever reflects the intended release state.

**Acceptance Criteria:**
- [ ] `README.md`, `SECURITY.md`, and every `Cargo.toml`'s `version` field agree on the same current version number.

**Technical Notes:** `CHANGELOG.md`'s `[0.2.0]` heading (dated `2024-12-XX`, still a placeholder date) suggests a `0.2.0` release was intended but never had its Cargo.toml versions bumped to match — worth reconciling in the same pass as GitHub #86's `CHANGELOG.md` staleness fix.

**Relevant Files:** `README.md:324`, `SECURITY.md:5-8`, `contracts/*/Cargo.toml`

**Testing Requirements:** N/A (documentation/versioning only).

**Definition of Done:** The version number documented in `README.md` and `SECURITY.md` matches what every `Cargo.toml` in the workspace actually declares.

**Suggested Labels:** `documentation`

---

### 95. `docs/DEPLOYMENT.md` documents a phantom `update_escrow_contract` function and phantom integration-test infrastructure

**Summary:** `DEPLOYMENT.md`'s "Rollback Procedures" section shows invoking `update_escrow_contract` on `delivery_contract` — a function that does not exist anywhere in the codebase, since `delivery_contract` has no setter for its `EscrowContract` address at all once `init` runs. The same document's "Integration Testing" section references `./scripts/deploy-local-test.sh` and `cargo test --test integration_tests`, neither of which exist.

**Background:** `docs/DEPLOYMENT.md:344-349` shows `stellar contract invoke --id $DELIVERY_CONTRACT_ID ... -- update_escrow_contract --new_escrow_contract $OLD_ESCROW_CONTRACT_ID`. A full read of `contracts/delivery_contract/lib.rs` confirms `DataKey::EscrowContract` is written only once, inside `init` (`:60-76`) — no function named `update_escrow_contract`, or any equivalent setter, exists. Separately, `docs/DEPLOYMENT.md:141-154` references `scripts/deploy-local-test.sh` (not present in `scripts/`) and `cargo test --test integration_tests` (there is no `tests/integration_tests` directory or corresponding Cargo test target anywhere in the workspace).

**Problem Statement:** Anyone following this document's documented rollback procedure verbatim would have their `stellar contract invoke` command fail outright (unknown function), with no indication from the doc itself that the function was never implemented. The integration-testing section has the identical problem for a different, non-fund-moving reason: the referenced tooling simply isn't there.

**Why it Matters:** A rollback procedure is exactly the kind of document an operator reaches for *during an incident*, when discovering it references a nonexistent function is the worst possible moment to learn that.

**Proposed Solution:** Either implement a real `update_escrow_contract`-equivalent setter on `delivery_contract` and keep the doc accurate to it, or rewrite the rollback section to describe what recovery actually looks like given the current contract's lack of a setter (e.g., full redeploy). Remove or replace the `deploy-local-test.sh`/`integration_tests` references with what actually exists in `scripts/` and the test suite today.

**Acceptance Criteria:**
- [ ] `docs/DEPLOYMENT.md`'s rollback section either references a real, implemented function, or accurately describes that no such setter currently exists.
- [ ] `docs/DEPLOYMENT.md`'s integration-testing section references only scripts and test commands that actually exist in the repository.

**Technical Notes:** This is a good candidate to resolve together with GitHub #104 (`identity_reputation_contract`'s own missing cross-contract setters) if the decision is to *add* the missing setter rather than rewrite the doc around its absence, since both are instances of the same "some contracts lack post-init address setters" gap.

**Relevant Files:** `docs/DEPLOYMENT.md:141-154, 331-350`, `contracts/delivery_contract/lib.rs:60-76`

**Testing Requirements:** N/A (documentation-only), unless a new setter function is added, in which case standard admin-gated setter tests apply.

**Definition of Done:** `docs/DEPLOYMENT.md` references only functions, scripts, and test commands that actually exist in the repository.

**Suggested Labels:** `documentation`

---

### 96. Contributor-facing docs and `dependabot.yml` reference GitHub labels that don't exist or don't match real label names

**Summary:** `CONTRIBUTING.md` tells newcomers to look for issues labeled `shared-types` (no such label exists in the repository). `PLAN.md` references `good-first-issue`, `help-wanted`, and `high-priority` (hyphenated) when the repository's real labels are `good first issue` and `help wanted` (space-separated) — and `high-priority` doesn't exist at all. `.github/dependabot.yml` configures automated PRs to be labeled `dependencies`, `rust`, and `github-actions`, none of which exist either.

**Background:** `CONTRIBUTING.md:93` ("We highly recommend starting with issues labeled `shared-types`"). `PLAN.md:69-74` lists `good-first-issue`, `help-wanted`, `high-priority`, `documentation`, `bug`, `feature` as the project's issue labels. `.github/dependabot.yml:12-14, 25-27` specifies `labels: ["dependencies", "rust"]` and `labels: ["dependencies", "github-actions"]`. The repository's actual label set (confirmed via the GitHub API) is: `bug`, `documentation`, `duplicate`, `enhancement`, `good first issue`, `help wanted`, `invalid`, `question`, `wontfix`, `security`, `feature`, `refactor`, `test`, `performance` — none of `shared-types`, `high-priority`, `dependencies`, `rust`, or `github-actions` are present, and the two that are conceptually close (`good-first-issue`/`good first issue`, `help-wanted`/`help wanted`) don't match GitHub's actual formatting.

**Problem Statement:** A new contributor following `CONTRIBUTING.md`'s explicit advice to filter by `shared-types` finds nothing; `PLAN.md`'s guidance about beginner-friendly or urgent work points at labels that either don't exist or are silently ignored by GitHub's UI due to the formatting mismatch; and every Dependabot-created PR silently fails to apply any of its three configured labels.

**Why it Matters:** This is the same underlying failure mode in three different files serving three different audiences (human contributors twice, an automated tool once) — none of the label-based guidance in this repository currently works as written.

**Proposed Solution:** Either create the missing labels (`shared-types`, `high-priority`, `dependencies`, `rust`, `github-actions`) to match what the docs and config already promise, or update `CONTRIBUTING.md`, `PLAN.md`, and `dependabot.yml` to reference the labels that actually exist.

**Acceptance Criteria:**
- [ ] Every label referenced in `CONTRIBUTING.md` and `PLAN.md` either exists in the repository or the document is updated to match real label names.
- [ ] `dependabot.yml`'s configured labels either exist or are updated to reference real ones.

**Technical Notes:** Note that this task's own labeling constraint (only `bug`, `feature`, `enhancement`, `refactor`, `documentation`, `test`, `security`, `performance` may be used) means any *new* labels this fix decides to create would need separate, explicit approval beyond that set.

**Relevant Files:** `CONTRIBUTING.md:93`, `PLAN.md:68-74`, `.github/dependabot.yml:12-14, 25-27`

**Testing Requirements:** N/A (repository configuration/documentation only).

**Definition of Done:** Every GitHub label referenced across `CONTRIBUTING.md`, `PLAN.md`, and `dependabot.yml` actually exists and is spelled the way GitHub's UI expects.

**Suggested Labels:** `documentation`

---

### 97. `docs/architecture/smart-contract-architecture.md` documents a phantom `RoleType` enum and a phantom `PickedUp` `DeliveryStatus` variant

**Summary:** This document's description of `shared_types` lists `RoleType (Sender, Receiver, Driver, FleetOwner, Admin)` as one of the library's shared enums — no such type exists anywhere in the codebase. It also documents `DeliveryStatus` as `(Pending, PickedUp, InTransit, Delivered, Disputed)` — the real enum has no `PickedUp` variant, and omits the real `Active` and `Cancelled` variants entirely.

**Background:** `docs/architecture/smart-contract-architecture.md:7-11` lists both. `contracts/shared_types/lib.rs:193-202` defines the real `DeliveryStatus` as `Pending, Active, InTransit, Delivered, Disputed, Cancelled` — no `RoleType` type is declared, imported, or referenced anywhere in the workspace.

**Problem Statement:** This is the same file GitHub #61/#67 already flags for internally contradicting itself about the contract/library count — here it additionally invents an entire type that was never built (`RoleType`) and misdescribes a real one (`DeliveryStatus`), suggesting the document was written aspirationally ahead of implementation and never reconciled against the code that actually shipped.

**Why it Matters:** For a document whose entire purpose is being the canonical architecture reference (linked from `README.md`'s own documentation section), describing types that don't exist is more actively misleading than simply being incomplete.

**Proposed Solution:** Correct the `shared_types` section to list only real types (`DeliveryStatus`, `EscrowState`, `CargoCategory`, and the structs actually defined), with `DeliveryStatus`'s real variant list.

**Acceptance Criteria:**
- [ ] The document's `shared_types` description lists only types that actually exist in `contracts/shared_types/lib.rs`.
- [ ] `DeliveryStatus`'s documented variants match the real enum exactly.

**Technical Notes:** Resolve alongside GitHub #61/#67's contract-count fix for this same file, since both are accuracy corrections to the same short document.

**Relevant Files:** `docs/architecture/smart-contract-architecture.md:7-11`, `contracts/shared_types/lib.rs:193-202`

**Testing Requirements:** N/A (documentation-only).

**Definition of Done:** `docs/architecture/smart-contract-architecture.md` describes only types that actually exist in the codebase, accurately.

**Suggested Labels:** `documentation`

---

### 98. `docs/SECURITY_AUDIT.md` prescribes a `security_`/`access_control_`/`state_transition_` test-naming convention that zero tests in the codebase use

**Summary:** The document's "Testing for Security" section instructs auditors to run `cargo test security_`, `cargo test access_control_`, and `cargo test state_transition_` to isolate security-relevant tests by name prefix — no test function anywhere in the workspace is named with any of these three prefixes, so all three commands currently run zero tests.

**Background:** `docs/SECURITY_AUDIT.md:150-156`. `grep`-confirmed across every `contracts/*/test.rs` file: no function matches `fn security_`, `fn access_control_`, or `fn state_transition_`. Existing tests use descriptive names instead (e.g., `test_release_escrow_unauthorized_rejected`, `test_invalid_dispute_when_cancelled`) that are perfectly good tests, just not named to match the documented filter convention.

**Problem Statement:** An auditor or CI job following this document's own instructions to isolate security/access-control/state-transition tests for focused review would get an empty test run and, without independently checking, could reasonably conclude no such tests exist at all — when in fact there are many, just not discoverable by the documented naming filter.

**Why it Matters:** This directly undermines the "Testing for Security" section's stated purpose: making the codebase's existing (and reasonably thorough) authorization and state-machine test coverage easy to isolate and re-run specifically during a security review.

**Proposed Solution:** Either rename relevant existing tests to include the documented prefixes (a large, cross-cutting rename), or update the document to describe how to actually filter the existing, descriptively-named tests (e.g., `cargo test unauthorized`, `cargo test invalid_state`).

**Acceptance Criteria:**
- [ ] The commands `docs/SECURITY_AUDIT.md` documents for isolating security-relevant tests actually match a meaningful subset of the real test suite.

**Technical Notes:** Updating the documentation to match existing naming conventions is far lower-risk than a workspace-wide test rename, and is the recommended direction unless a broader test-naming standardization is independently planned.

**Relevant Files:** `docs/SECURITY_AUDIT.md:148-156`, `contracts/*/test.rs`

**Testing Requirements:** N/A (documentation/tooling-alignment only).

**Definition of Done:** `docs/SECURITY_AUDIT.md`'s documented test-filtering commands actually select a real, meaningful subset of the test suite.

**Suggested Labels:** `test`

---

### 99. `scripts/deploy-contract.sh` and `scripts/initialize-contract.sh` are committed empty, silently breaking `README.md`'s documented flow and `deploy-testnet.yml`

**Summary:** Both scripts are committed as 0-byte files — yet `README.md`'s "Contract Deployment Instructions" directly invokes `./scripts/deploy-contract.sh escrow_contract` and `./scripts/initialize-contract.sh <CONTRACT_ID>`, and `.github/workflows/deploy-testnet.yml`'s manual single-contract deploy path runs `bash scripts/deploy-contract.sh ${{ github.event.inputs.contract }}`.

**Background:** `ls -la scripts/` confirms `deploy-contract.sh` and `initialize-contract.sh` are both exactly 0 bytes, alongside the fully-implemented `deploy-all-contracts.sh` (3.6 KB) and `initialize-all-contracts.sh` (1.8 KB). `README.md:287-294` documents the single-contract flow using both empty scripts as the primary, first-mentioned deployment method (ahead of any "deploy all" alternative). `.github/workflows/deploy-testnet.yml:56-64` branches on the `workflow_dispatch` input: `all` runs `deploy-all-contracts.sh`; any specific contract name runs the empty `deploy-contract.sh`.

**Problem Statement:** Any contributor following `README.md`'s own first documented deployment path gets a script that does nothing — no error, no output, just silent non-execution of an empty file, immediately followed by an initialization step that also does nothing. Worse, triggering `deploy-testnet.yml`'s workflow with anything other than `all` in the `contract` dropdown silently "succeeds" while deploying nothing, since `bash` executing an empty file exits `0`.

**Why it Matters:** This is the single most severe deployment-tooling gap in the repository: it's not a documentation inaccuracy pointing at a real gap elsewhere — it's the *actual, currently invoked* CI automation for single-contract testnet deploys doing nothing, which would not be caught by a casual glance at a green workflow run.

**Proposed Solution:** Implement both scripts (a single-contract equivalent of `deploy-all-contracts.sh`'s build+deploy logic, parameterized by contract name, and a single-contract equivalent of `initialize-all-contracts.sh`'s init logic).

**Acceptance Criteria:**
- [ ] `scripts/deploy-contract.sh <contract_name>` builds and deploys exactly the named contract, writing its ID to output.
- [ ] `scripts/initialize-contract.sh <CONTRACT_ID>` initializes the specified contract.
- [ ] `deploy-testnet.yml`'s single-contract dispatch path actually deploys a contract when triggered.

**Technical Notes:** This should be fixed together with, or at minimum sequenced around, Issue #106 (`deploy-all-contracts.sh`'s output-capture corruption bug), since a single-contract script implemented by extracting logic from that file would otherwise inherit the same bug.

**Relevant Files:** `scripts/deploy-contract.sh`, `scripts/initialize-contract.sh`, `README.md:287-294`, `.github/workflows/deploy-testnet.yml:56-64`

**Testing Requirements:** Manual dry run against a testnet deployment confirming both scripts actually deploy and initialize a single named contract; trigger `deploy-testnet.yml` with a specific contract selected and confirm a real deployment occurs.

**Definition of Done:** Both scripts perform real work, and neither `README.md`'s documented flow nor `deploy-testnet.yml`'s single-contract path silently does nothing.

**Suggested Labels:** `bug`

---

### 100. Leftover `SwiftChainError` test comments and phantom `.gitignore` script paths from a pre-rename project name

**Summary:** Five `#[should_panic]` annotations in `dispute_resolution_contract/test.rs` are commented with a reference to `SwiftChainError` — a type that does not exist anywhere in the codebase, since the actual error type these tests exercise is `FaniLabError`. `.gitignore` separately references `scripts/initialize/create-labels.py` and `scripts/initialize/create-swift-smart-contract-issues.py`, neither of which — nor the `scripts/initialize/` directory itself — exists anywhere in the repository.

**Background:** `contracts/dispute_resolution_contract/test.rs:229, 325, 404, 467, 618` each annotate a panic expectation with `// SwiftChainError::Unauthorized` or `// SwiftChainError::InvalidState`, immediately above code that actually asserts against `FaniLabError::Unauthorized`/`FaniLabError::InvalidState`. `.gitignore:9-11` lists `scripts-issues`, `scripts/initialize/create-labels.py`, and `scripts/initialize/create-swift-smart-contract-issues.py` — the "swift" in the second filename, combined with the test comments, indicates the project was previously named "SwiftChain" before its rename to FaniLab, and not every reference to the old name or its associated (apparently abandoned) tooling was cleaned up.

**Problem Statement:** These are small but genuine artifacts of an incomplete rename: the test comments actively mislead anyone reading them about which type is actually being tested, and the `.gitignore` entries reference tooling that was seemingly planned (label-creation and issue-generation scripts) but never committed, leaving no way to tell from the repository alone whether that tooling still exists somewhere else, was abandoned, or was renamed without updating `.gitignore`.

**Why it Matters:** Comments that name the wrong type are exactly the kind of small inaccuracy that erodes trust in a file's other comments once a reader notices one is wrong — and stale `.gitignore` entries for a differently-named, nonexistent script are a minor but real signal of incomplete project hygiene during the rename.

**Proposed Solution:** Correct all five test comments to reference `FaniLabError` instead of `SwiftChainError`; remove the phantom `.gitignore` entries if the referenced tooling was genuinely abandoned (or restore/rename the tooling if it still exists elsewhere and was simply misplaced).

**Acceptance Criteria:**
- [ ] No test comment in the workspace references `SwiftChainError`.
- [ ] `.gitignore` contains no path referencing a script or directory that doesn't exist in the repository.

**Technical Notes:** This is a pure cleanup with no behavioral change — the tests themselves already correctly assert against `FaniLabError`; only their explanatory comments are wrong.

**Relevant Files:** `contracts/dispute_resolution_contract/test.rs:229, 325, 404, 467, 618`, `.gitignore:9-11`

**Testing Requirements:** N/A (comment/config cleanup only).

**Definition of Done:** No remaining reference to the project's prior name or its abandoned tooling paths anywhere in test comments or `.gitignore`.

**Suggested Labels:** `refactor`

---

### 101. Repository root contains leftover developer debris (`test_script.py`, `tests_passing.png`)

**Summary:** `test_script.py`, committed at the repository root, is a one-off script that — if ever executed — permanently rewrites `contracts/delivery_contract/test.rs` in place, inserting a debug `println!` into every test matching a specific pattern. `tests_passing.png`, also at the root, is a static screenshot with no documented purpose, superseded by the CI status badges already in `README.md`.

**Background:** `test_script.py`'s full contents read committed test source (`contracts/delivery_contract/test.rs`), perform a string replacement inserting `println!("EVENTS LEN: {}", events.len());` before every `let last_event = events.last().unwrap();` occurrence, and overwrite the file with the result — a mutation script, not a test, sitting alongside `README.md`/`Cargo.toml` at the project root as if it were part of the maintained toolset.

**Problem Statement:** Neither file is referenced by any Makefile target, CI workflow, or documentation anywhere in the repository — both appear to be one-off artifacts from local development that were accidentally committed. `test_script.py` in particular is actively dangerous if anyone unfamiliar with it runs it expecting a normal test utility: it silently mutates committed source rather than just reporting results.

**Why it Matters:** A committed script that rewrites test source files in place, with a name suggesting it's a normal project utility, is a real risk of accidental data loss or confusing diffs for the next person who runs anything matching `test_script.py` out of curiosity.

**Proposed Solution:** Remove both files from the repository (or, if `test_script.py`'s debug-instrumentation capability is still useful, move it to a clearly-labeled `scratch/`-style location excluded from the main tree with a comment explaining its one-off purpose).

**Acceptance Criteria:**
- [ ] Neither `test_script.py` nor `tests_passing.png` remains at the repository root without a documented purpose.

**Technical Notes:** Confirm neither file is referenced anywhere (CI, Makefile, docs) before removal — a quick repository-wide search confirms zero references to either filename outside their own content.

**Relevant Files:** `test_script.py`, `tests_passing.png`

**Testing Requirements:** N/A (repository hygiene only); confirm `cargo test` and all CI workflows are unaffected by removal.

**Definition of Done:** The repository root contains no undocumented, one-off developer artifacts that could be mistaken for maintained project tooling.

**Suggested Labels:** `refactor`

---

### 102. `.vscode/settings.json` pins the stale `wasm32-unknown-unknown` target; `launch.json` only has a debug configuration for `escrow_contract`

**Summary:** `.vscode/settings.json` sets `"rust-analyzer.cargo.target": "wasm32-unknown-unknown"`, causing rust-analyzer to check and report errors against the pre-migration target rather than the current `wasm32v1-none` the CI pipeline and Makefile.windows both build against. Separately, `.vscode/launch.json` provides a named debug configuration only for `escrow_contract`'s tests, with no equivalent for the other five contracts.

**Background:** `.vscode/settings.json:2` (`"rust-analyzer.cargo.target": "wasm32-unknown-unknown"`). `.vscode/launch.json:14-27` defines "Debug escrow_contract tests" (`cargo test --no-run -p escrow_contract`) alongside a generic "Debug unit tests" configuration, but no "Debug delivery_contract tests," "Debug dispute_resolution_contract tests," etc.

**Problem Statement:** Anyone using the recommended VS Code setup (per `.vscode/extensions.json`'s `rust-lang.rust-analyzer` recommendation) gets in-editor diagnostics computed against a target the project no longer builds for — potentially surfacing false errors or hiding real ones specific to `wasm32v1-none`. The single-contract debug configuration gap is a smaller but related DX inconsistency: five of six contracts have no one-click debug launch entry.

**Why it Matters:** This is the same `wasm32-unknown-unknown` staleness already tracked for the Makefile (GitHub #57), `deploy-all-contracts.sh` (GitHub #56), and — elsewhere in this batch — `README.md`/`CONTRIBUTING.md`/`docs/DEPLOYMENT.md`, but in the one place that actively shapes what a contributor sees while writing code in their editor, not just what they'd run from a terminal.

**Proposed Solution:** Update `rust-analyzer.cargo.target` to `wasm32v1-none`, and add a debug launch configuration for each of the remaining five contracts, mirroring the existing `escrow_contract` entry.

**Acceptance Criteria:**
- [ ] `.vscode/settings.json` targets `wasm32v1-none`.
- [ ] `.vscode/launch.json` has a debug configuration for all six contracts, not just `escrow_contract`.

**Technical Notes:** This can be fixed independently of the Makefile/script fixes tracked elsewhere, since it's a distinct file with its own distinct mechanism (editor tooling, not build/CI).

**Relevant Files:** `.vscode/settings.json:2`, `.vscode/launch.json:14-27`

**Testing Requirements:** N/A (editor configuration only); manually confirm rust-analyzer reports no spurious target-specific errors after the change.

**Definition of Done:** VS Code's configured Rust target matches the project's actual build target, and every contract has an equivalent debug launch entry.

**Suggested Labels:** `enhancement`

---
### 103. All four CI workflows pin `dtolnay/rust-toolchain@stable`, a mutable branch reference, instead of a fixed version

**Summary:** `ci.yml`, `release.yml`, `security-audit.yml`, and `deploy-testnet.yml` all reference the third-party action `dtolnay/rust-toolchain@stable` — a mutable ref that can change what it resolves to at any time, rather than a pinned tag or commit SHA.

**Background:** `grep`-confirmed identical usage in `.github/workflows/ci.yml:22`, `release.yml:21`, `security-audit.yml:26`, and `deploy-testnet.yml:34`. Unlike `actions/checkout@v4` (a pinned major version tag) used in the same workflows, `dtolnay/rust-toolchain@stable` resolves to whatever the `stable` branch of that external repository currently points to at the moment each workflow runs.

**Problem Statement:** Every one of this project's CI, release, security-audit, and deployment pipelines depends on the continued trustworthiness and availability of a branch reference this project has no control over — a compromise of the upstream `dtolnay/rust-toolchain` repository, or even an unintentional breaking change pushed to its `stable` branch, would silently alter what every workflow in this repository executes on its very next run, with no corresponding change to this repository's own history.

**Why it Matters:** This is a textbook software-supply-chain integrity gap: the project's own `deny.toml` and `security-audit.yml` are specifically designed to catch exactly this class of risk for Cargo dependencies (vulnerability/license/ban checks), yet the CI pipeline that runs those very checks is itself pinned to a mutable external reference with none of the same scrutiny applied.

**Proposed Solution:** Pin `dtolnay/rust-toolchain` to a specific release tag or commit SHA in all four workflows, updated deliberately (e.g., via Dependabot's `github-actions` ecosystem support, which `dependabot.yml` already configures) rather than automatically tracking `stable`.

**Acceptance Criteria:**
- [ ] All four workflows reference a fixed version or commit SHA of `dtolnay/rust-toolchain`, not the `stable` branch.

**Technical Notes:** `dependabot.yml`'s existing `github-actions` ecosystem block will automatically propose updates to a pinned SHA/tag going forward, once one is in place — this closes the loop between this fix and the automated-update tooling that already exists.

**Relevant Files:** `.github/workflows/ci.yml:22`, `.github/workflows/release.yml:21`, `.github/workflows/security-audit.yml:26`, `.github/workflows/deploy-testnet.yml:34`

**Testing Requirements:** Confirm all four workflows still succeed after pinning to a specific version.

**Definition of Done:** No CI workflow in the repository resolves its Rust toolchain from a mutable, unpinned reference.

**Suggested Labels:** `security`

---

### 104. Several workflows pin deprecated major versions of third-party GitHub Actions (`upload-artifact@v3`, `codecov-action@v3`)

**Summary:** `deploy-testnet.yml` uses `actions/upload-artifact@v3` and `ci.yml` uses `codecov/codecov-action@v3` — both major versions their respective vendors have deprecated in favor of v4 (and, for Codecov, v5), with GitHub having phased out backend support for `upload-artifact@v3` and `download-artifact@v3` altogether.

**Background:** `.github/workflows/deploy-testnet.yml:67` (`uses: actions/upload-artifact@v3`). `.github/workflows/ci.yml:48` (`uses: codecov/codecov-action@v3`). Both are the specific major versions GitHub/Codecov have publicly deprecated; workflows still referencing them are increasingly likely to fail outright, or already do, rather than merely emitting a deprecation warning.

**Problem Statement:** Since these steps sit at the very end of their respective jobs (artifact upload; coverage reporting), a failure here doesn't necessarily fail the whole workflow depending on how it's configured, meaning this class of breakage can go unnoticed for a long time — the job "passes" while its final reporting/artifact step silently stops working.

**Why it Matters:** This is a distinct problem from Issue #103 (a mutable, unpinned reference) — these two actions are pinned, but to specific versions that are now stale and unsupported, the opposite failure mode of the same underlying "keep CI action versions current" gap.

**Proposed Solution:** Bump `actions/upload-artifact` to `v4` and `codecov/codecov-action` to its current major version in the affected workflows.

**Acceptance Criteria:**
- [ ] `deploy-testnet.yml` and `ci.yml` both reference current, supported major versions of these third-party actions.

**Technical Notes:** `actions/upload-artifact@v4` has a materially different API for some options than v3 (e.g., default retention/compression behavior) — verify the workflow's `with:` block still behaves as intended after the bump, not just that the version number changes.

**Relevant Files:** `.github/workflows/deploy-testnet.yml:66-72`, `.github/workflows/ci.yml:47-51`

**Testing Requirements:** Trigger both workflows and confirm the artifact-upload and coverage-reporting steps both complete successfully post-upgrade.

**Definition of Done:** No workflow in the repository references a deprecated major version of a third-party GitHub Action.

**Suggested Labels:** `performance`

---

### 105. `deploy-testnet.yml`'s artifact-upload path patterns never match `deploy-all-contracts.sh`'s actual output filename

**Summary:** The workflow's final "Save Deployment Artifacts" step uploads files matching `deployment-*.json` and `contract-ids.txt` — but `deploy-all-contracts.sh` writes its output to `contract-ids-$NETWORK.json` (e.g., `contract-ids-testnet.json`), a filename neither pattern matches.

**Background:** `.github/workflows/deploy-testnet.yml:66-72`:
```yaml
- name: Save Deployment Artifacts
  uses: actions/upload-artifact@v3
  with:
    name: deployment-info
    path: |
      deployment-*.json
      contract-ids.txt
```
`scripts/deploy-all-contracts.sh:11` sets `OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"` — the file the deploy step actually produces is named `contract-ids-testnet.json` (or `contract-ids-mainnet.json`), matching neither `deployment-*.json` nor the exact literal `contract-ids.txt`.

**Problem Statement:** Every run of this workflow's artifact-upload step silently finds zero matching files and uploads nothing (or fails, depending on `upload-artifact`'s configured strictness) — the "deployment-info" artifact this workflow is specifically designed to preserve for every testnet deployment has, per this filename mismatch, likely never actually contained the deployed contract IDs.

**Why it Matters:** This defeats the entire purpose of the step: a record of which contract IDs were deployed on which run is exactly the kind of information an operator needs after the fact, and right now it's not being captured at all despite the workflow appearing to succeed.

**Proposed Solution:** Correct the `path:` patterns to match the real output filename (`contract-ids-*.json`), or standardize `deploy-all-contracts.sh` and this workflow on one agreed-upon filename.

**Acceptance Criteria:**
- [ ] The workflow's artifact-upload step actually captures `deploy-all-contracts.sh`'s real output file.
- [ ] A test run of the workflow confirms the uploaded artifact contains the deployed contract IDs.

**Technical Notes:** Fix this together with Issue #104 (the same step's `upload-artifact@v3` deprecation) since both touch the identical few lines of workflow YAML.

**Relevant Files:** `.github/workflows/deploy-testnet.yml:66-72`, `scripts/deploy-all-contracts.sh:11`

**Testing Requirements:** Trigger the workflow and confirm the resulting artifact download actually contains the deployment's `contract-ids-*.json` file.

**Definition of Done:** `deploy-testnet.yml`'s artifact-upload step captures the file `deploy-all-contracts.sh` actually produces.

**Suggested Labels:** `bug`

---

### 106. `deploy-all-contracts.sh`'s `deploy_contract()` captures decorative echo output into `$contract_id`, corrupting the JSON output file

**Summary:** `deploy_contract()` both prints human-readable status messages (`echo "${YELLOW}Deploying $contract_name...`, `echo "${GREEN}✓ ... deployed:`) and returns the actual contract ID, all via the same stdout stream. Because the caller captures the function's entire output with `contract_id=$(deploy_contract "$contract")`, every decorative message — not just the real ID — ends up inside `$contract_id`, which is then written directly into the deployment's JSON output file.

**Background:** `scripts/deploy-all-contracts.sh:57-82` (`deploy_contract`): the function `echo`s a "Deploying..." line, then (on success) a "✓ ... deployed: $contract_id" line, *then* `echo "$contract_id"` — all three lines go to the function's stdout. The caller, `scripts/deploy-all-contracts.sh:96` (`contract_id=$(deploy_contract "$contract")`), captures *all* of that output as a single multi-line string via command substitution. That multi-line, ANSI-color-coded value is then written verbatim into the JSON file at `:107-109` (`echo "    \"$contract\": \"$contract_id\","`).

**Problem Statement:** `contract-ids-$NETWORK.json` — the file `initialize-all-contracts.sh` depends on for parsing contract IDs (via `grep`, per GitHub #58) and the same file Issue #105 is about correctly uploading — is not valid JSON after any real deployment run: it contains embedded newlines, ANSI escape codes, and decorative emoji/text baked into what should be a plain contract ID string.

**Why it Matters:** This is a foundational bug in the deployment pipeline's data hand-off: every downstream consumer of `contract-ids-$NETWORK.json` (the initialization script, the CI artifact, any future tooling) inherits corrupted input the moment a real deployment runs, regardless of how correct those downstream consumers' own logic is.

**Proposed Solution:** Redirect `deploy_contract`'s decorative status messages to stderr (`>&2`) so only the final, real contract ID is ever captured by the caller's command substitution — a minimal, standard bash fix for exactly this class of function-return-via-stdout pitfall.

**Acceptance Criteria:**
- [ ] `deploy_contract`'s decorative echo statements no longer appear in the captured `$contract_id` value.
- [ ] `contract-ids-$NETWORK.json` is valid, parseable JSON after a real deployment run.

**Technical Notes:** This should be fixed before or alongside Issue #99 (implementing the currently-empty `deploy-contract.sh`), since any single-contract script built by extracting this function would otherwise inherit the identical bug.

**Relevant Files:** `scripts/deploy-all-contracts.sh:56-138`

**Testing Requirements:** Run the script against a mock/local deployment and verify the resulting `contract-ids-$NETWORK.json` parses successfully with a standard JSON parser (`jq` or `python -m json.tool`).

**Definition of Done:** `deploy-all-contracts.sh` produces valid JSON on every run, with no decorative output leaking into the captured contract ID.

**Suggested Labels:** `bug`

---

### 107. `ci.yml`'s outdated-dependency check is configured with `continue-on-error: true`, so it can never actually fail the build

**Summary:** The "Check for Outdated Dependencies" step runs `cargo outdated --exit-code 1` — designed to fail the job if any dependency is outdated — but the step itself is marked `continue-on-error: true`, meaning that non-zero exit code is unconditionally swallowed and the job proceeds as if it succeeded regardless of the result.

**Background:** `.github/workflows/ci.yml:58-62`:
```yaml
- name: Check for Outdated Dependencies
  run: |
    cargo install cargo-outdated
    cargo outdated --exit-code 1
  continue-on-error: true
```

**Problem Statement:** `--exit-code 1` and `continue-on-error: true` are directly self-defeating when combined — the former exists specifically to make CI fail on outdated dependencies, and the latter unconditionally erases that failure signal before it can affect the workflow's outcome. As currently configured, this step can never cause `ci.yml` to fail no matter how outdated the workspace's dependencies become.

**Why it Matters:** This is distinct from GitHub #36 (which is about *adding* missing checks like `cargo machete` and a coverage floor) — this issue is about an *existing* check that already runs on every PR and push, but has been configured to have zero actual enforcement power, giving a false impression that dependency freshness is being monitored.

**Proposed Solution:** Remove `continue-on-error: true` if outdated dependencies should genuinely block CI, or, if the intent is advisory-only, remove `--exit-code 1` (which serves no purpose once the exit code is ignored anyway) and clarify in the step name that it's informational.

**Acceptance Criteria:**
- [ ] The step's configuration is internally consistent: either it can fail the build on outdated dependencies (no `continue-on-error`), or it's clearly advisory-only (no `--exit-code 1`), not both mechanisms fighting each other.

**Technical Notes:** If enforcement is chosen, this should land carefully — check whether the workspace's dependencies are already outdated today, to avoid immediately breaking CI on landing.

**Relevant Files:** `.github/workflows/ci.yml:58-62`

**Testing Requirements:** Verify the step's new behavior matches its documented intent — either failing CI when an outdated dependency is intentionally introduced in a test branch, or clearly not affecting the job outcome.

**Definition of Done:** The outdated-dependency check's configuration matches its actual, intended enforcement level.

**Suggested Labels:** `performance`

---

### 108. `security-audit.yml` only runs `cargo deny check advisories`, never enforcing `deny.toml`'s license and dependency-ban rules

**Summary:** The workflow's "Run cargo-deny" step executes `cargo deny check advisories` specifically — but `deny.toml` also defines a `[licenses]` section (denying unlicensed and copyleft dependencies) and a `[bans]` section (flagging multiple-version duplicates) that are never checked by any command this workflow, or any other workflow in the repository, ever runs.

**Background:** `.github/workflows/security-audit.yml:40-41` (`run: cargo deny check advisories`). `deny.toml:12-28` defines `[licenses] unlicensed = "deny"`, `copyleft = "deny"`, an explicit `allow` list of permitted licenses, and `deny.toml:30-34`'s `[bans] multiple-versions = "warn"`. Running `cargo deny check` with no argument (or `cargo deny check licenses`/`check bans` explicitly) is required to actually evaluate those two sections — `check advisories` alone only evaluates the `[advisories]` section.

**Problem Statement:** Roughly two-thirds of `deny.toml`'s configured enforcement — the license-compliance rules and the dependency-ban rules — is dead configuration from CI's perspective: a dependency with a denied (e.g., copyleft) license, or an explicitly banned crate, could be added to any `Cargo.toml` in the workspace today and no workflow would ever flag it.

**Why it Matters:** `PRODUCTION_READINESS.md` cites `deny.toml` as evidence of both "Security" and "Documentation" readiness ("License and dependency checks... License compliance checking (cargo-deny)") — but the CI automation backing that claim only actually exercises the smallest of the file's three configured concerns.

**Proposed Solution:** Change the workflow step to `cargo deny check` (no argument, evaluating all configured categories: advisories, licenses, bans, and sources) or add explicit `cargo deny check licenses` and `cargo deny check bans` steps alongside the existing advisories check.

**Acceptance Criteria:**
- [ ] `security-audit.yml` actually evaluates `deny.toml`'s license and ban rules, not just its advisory rules.

**Technical Notes:** Run `cargo deny check` locally first (once a toolchain is available) to confirm the current dependency set already passes the license/bans rules before wiring this into CI as a blocking check, to avoid landing a change that immediately breaks the pipeline on an unrelated pre-existing violation.

**Relevant Files:** `.github/workflows/security-audit.yml:37-41`, `deny.toml:12-34`

**Testing Requirements:** Confirm the updated workflow step runs successfully against the current dependency tree, and deliberately introduce a disallowed license locally to confirm the check now catches it.

**Definition of Done:** `security-audit.yml` enforces every category `deny.toml` configures, not just advisories.

**Suggested Labels:** `security`

---

### 109. `release.yml` builds and publishes a GitHub Release without ever running the test suite

**Summary:** The release workflow, triggered on any `v*` tag push, builds all six contracts, packages the resulting WASM, generates checksums, and publishes a GitHub Release — at no point does it run `cargo test`.

**Background:** `.github/workflows/release.yml`'s `create_release` job runs: checkout, toolchain setup, `cargo build --target wasm32v1-none --release` (`:25-26`), WASM staging (`:28-34`), checksum generation (`:36-39`), release-notes generation (`:41-56`), and `softprops/action-gh-release@v1` (`:58-68`) — no `cargo test` step exists anywhere in this file, unlike `ci.yml`, which does run tests on every push/PR to `main`.

**Problem Statement:** Because this workflow triggers on a *tag* push rather than a push to `main`, it is entirely possible to tag and publish a release from a commit that either never went through `ci.yml` at all, or went through it but had test failures ignored — nothing in `release.yml` itself verifies the code being packaged and published actually passes its own test suite before the release goes out.

**Why it Matters:** A published GitHub Release is the artifact external integrators and downstream deployers actually consume — publishing one without a test gate means the project's own test suite, however thorough, provides no guarantee about what ends up in a tagged release specifically.

**Proposed Solution:** Add a `cargo test --verbose` step (or a dependency on `ci.yml`'s equivalent job having already passed for the tagged commit) before the build/package/publish steps.

**Acceptance Criteria:**
- [ ] `release.yml` runs the full test suite and fails the workflow (blocking the release) if any test fails, before building or publishing release artifacts.

**Technical Notes:** A `needs:`-based dependency on a reusable "test" job shared with `ci.yml` would avoid duplicating the test step's definition across both workflows.

**Relevant Files:** `.github/workflows/release.yml:11-69`

**Testing Requirements:** Trigger the workflow against a tag with a deliberately failing test and confirm the release is *not* published.

**Definition of Done:** No tagged release can be published without the full test suite passing first.

**Suggested Labels:** `bug`

---

### 110. No CI step measures or enforces the 64 KB WASM contract-size limit

**Summary:** `docs/API.md`'s "Rate Limits & Constraints" section and `docs/PERFORMANCE.md`'s own "Performance Checklist" both name a 64 KB maximum WASM contract size as a hard Soroban platform limit — no workflow in `.github/workflows/` ever measures the size of a built `.wasm` file, let alone fails the build if one exceeds it.

**Background:** `docs/API.md:551` ("Max contract size: 64 KB (WASM)"). `docs/PERFORMANCE.md:305` ("[ ] Contract size < 60 KB" — the checklist's own self-imposed margin below the hard limit). `ci.yml` and `release.yml` both build all six contracts to WASM but neither includes any `ls -la`/size-comparison/threshold-check step on the resulting artifacts.

**Problem Statement:** A contract that silently grows past 64 KB — plausible as more functions, error variants, and cross-contract wiring accumulate across the kind of feature work already tracked throughout this backlog — would only be discovered at actual deployment time, when `stellar contract deploy` rejects it, rather than being caught automatically in CI the moment the regression is introduced.

**Why it Matters:** This is a hard platform limit, not a soft guideline — unlike most of `docs/PERFORMANCE.md`'s advice (which is about efficiency), exceeding this one specific number makes a contract literally undeployable, making it the single highest-value, most mechanically simple check currently missing from CI.

**Proposed Solution:** Add a step to `ci.yml` (and/or `release.yml`) that measures each built `.wasm` file's size and fails the job if any exceeds a configured threshold (e.g., 60 KB, matching `docs/PERFORMANCE.md`'s own stated safety margin below the hard 64 KB limit).

**Acceptance Criteria:**
- [ ] CI fails if any contract's built WASM artifact exceeds the configured size threshold.
- [ ] The threshold and its rationale are documented alongside the check itself.

**Technical Notes:** This is a small, mechanical shell check (`wc -c` or `stat` on each `.wasm` file compared against a threshold) with no dependency on any tool beyond what CI already has installed.

**Relevant Files:** `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/API.md:551`, `docs/PERFORMANCE.md:303-312`

**Testing Requirements:** Confirm the check correctly passes against current contract sizes and correctly fails when a test artifact is deliberately padded past the threshold.

**Definition of Done:** CI automatically catches a contract growing past the Soroban WASM size limit before it ever reaches deployment.

**Suggested Labels:** `performance`

---

## Summary by Label

| Label | Count | Issues |
|---|---|---|
| `documentation` | 6 | 92, 93, 94, 95, 96, 97 |
| `bug` | 4 | 99, 105, 106, 109 |
| `refactor` | 3 | 91, 100, 101 |
| `performance` | 3 | 104, 107, 110 |
| `security` | 2 | 103, 108 |
| `test` | 1 | 98 |
| `enhancement` | 1 | 102 |

(Issues #11–#110 were filed to GitHub across three review passes — see `Pushed to GitHub` above — and no longer appear here.)

## Summary by Contract

| Contract / Area | Issues in this doc | Filed on GitHub |
|---|---|---|
| `escrow_contract` | — | #7, #11, #12, #13, #14, #15, #16, #17, #18, #25, #26, #31, #37, #38, #54, #55, #61, #68, #73, #74, #82, #87, #88, #89, #90, #91, #92, #93, #100 |
| `delivery_contract` | — | #19, #20, #23, #24, #27, #33, #44, #45, #68, #93, #94, #95, #96, #97, #98 |
| `dispute_resolution_contract` | — | #8, #21, #22, #32, #40, #49, #50, #51, #76, #77, #94, #99, #100, #101, #102, #103, #105 |
| `identity_reputation_contract` | — | #9, #10, #24, #41, #43, #44, #48, #75, #104, #105, #106, #107 |
| `fleet_management_contract` | — | #12, #26, #27, #28, #39, #69, #70, #71, #83, #84, #108, #109, #110 |
| `settlement_contract` | — | #15, #30, #35, #55, #116 |
| `shared_types` | — | #24, #26, #33, #41, #42, #46, #47, #85, #111, #112, #113 |
| Docs (`docs/`, root `*.md`) | 92, 93, 94, 95, 96, 97, 98 | #64, #65, #66, #67, #78, #79, #81, #86 |
| CI/CD (`.github/workflows/`) | 103, 104, 105, 107, 108, 109, 110 | #56, #57, #62, #63 |
| Scripts/tooling (`scripts/`, `Makefile`, `.env.example`) | 99, 106 | #56, #57, #58, #59, #60 |
| Cross-cutting / process | 91, 100, 101, 102 | #7, #8, #19, #27, #31, #34, #36, #46, #47, #52, #53, #77, #80, #114, #115 |

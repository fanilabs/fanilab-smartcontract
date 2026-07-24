# FaniLab Smart Contracts — Substantive Issues (Backlog Fully Published)

Derived from a direct read of every contract in `contracts/` (escrow_contract,
delivery_contract, dispute_resolution_contract, fleet_management_contract,
identity_reputation_contract, settlement_contract, shared_types) plus the
project's own `PLAN.md`, `PRODUCTION_READINESS.md`, `Cargo.toml`, and CI
workflow. Every issue below references the specific function and file it was
found in — none are generic placeholders.

This document consolidated three review passes: an initial pass (all 30 of whose issues — 6 Critical, 4 High, and local issues #11–#30 — have since been filed to GitHub, see below), a follow-up pass extending coverage to cross-contract architecture, testing gaps, CI/CD, deployment tooling, and documentation accuracy (issues #31–#80, all of which have since been filed to GitHub, see below), and a third pass covering reentrancy/authorization-bypass risk, further cross-contract consistency gaps, and repository-hygiene/documentation drift (issues #81–#130, all of which have since been filed to GitHub, see below). **The local backlog is now fully published: zero unpublished issues remain in this document.**

## Pushed to GitHub

All 130 issues identified across the three review passes have been filed on `github.com/fanilabs/fanilab-smartcontract` and removed from this document to avoid duplication: the original 10 highest-severity findings (6 Critical + 4 High), the full remaining High/Medium/Low-classified backlog from the initial review pass (local issues #11–#30), the follow-up pass's architecture/testing/tooling findings (local issues #31–#80), the third pass's earlier findings covering reentrancy/authorization-bypass risk and cross-contract consistency (local issues #81–#110, filed as GitHub #77–#116), and the third pass's final batch of repository-hygiene and CI/tooling findings (filed as GitHub #125–#144, completing the backlog). Track them there:

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
| [#125](https://github.com/fanilabs/fanilab-smartcontract/issues/125) | `Cargo.toml`'s `release-with-logs` build profile is entirely unused, dead configuration |
| [#126](https://github.com/fanilabs/fanilab-smartcontract/issues/126) | `README.md`'s Repository Structure diagram and `CONTRIBUTING.md`'s crate overview both describe a fictional 3-crate layout |
| [#127](https://github.com/fanilabs/fanilab-smartcontract/issues/127) | `README.md`'s CI/coverage badges and GitHub org link point to a nonexistent `github.com/fanilab/FaniLab-SmartContract` |
| [#128](https://github.com/fanilabs/fanilab-smartcontract/issues/128) | `README.md` and `SECURITY.md` claim version 0.2.x while every contract's `Cargo.toml` still declares `0.1.0` |
| [#129](https://github.com/fanilabs/fanilab-smartcontract/issues/129) | `docs/DEPLOYMENT.md` documents a phantom `update_escrow_contract` function and phantom integration-test infrastructure |
| [#130](https://github.com/fanilabs/fanilab-smartcontract/issues/130) | Contributor-facing docs and `dependabot.yml` reference GitHub labels that don't exist or don't match real label names |
| [#131](https://github.com/fanilabs/fanilab-smartcontract/issues/131) | `docs/architecture/smart-contract-architecture.md` documents a phantom `RoleType` enum and a phantom `PickedUp` `DeliveryStatus` variant |
| [#132](https://github.com/fanilabs/fanilab-smartcontract/issues/132) | `docs/SECURITY_AUDIT.md` prescribes a `security_`/`access_control_`/`state_transition_` test-naming convention that zero tests in the codebase use |
| [#133](https://github.com/fanilabs/fanilab-smartcontract/issues/133) | `scripts/deploy-contract.sh` and `scripts/initialize-contract.sh` are committed empty, silently breaking `README.md`'s documented flow and `deploy-testnet.yml` |
| [#134](https://github.com/fanilabs/fanilab-smartcontract/issues/134) | Leftover `SwiftChainError` test comments and phantom `.gitignore` script paths from a pre-rename project name |
| [#135](https://github.com/fanilabs/fanilab-smartcontract/issues/135) | Repository root contains leftover developer debris (`test_script.py`, `tests_passing.png`) |
| [#136](https://github.com/fanilabs/fanilab-smartcontract/issues/136) | `.vscode/settings.json` pins the stale `wasm32-unknown-unknown` target; `launch.json` only has a debug configuration for `escrow_contract` |
| [#137](https://github.com/fanilabs/fanilab-smartcontract/issues/137) | All four CI workflows pin `dtolnay/rust-toolchain@stable`, a mutable branch reference, instead of a fixed version |
| [#138](https://github.com/fanilabs/fanilab-smartcontract/issues/138) | Several workflows pin deprecated major versions of third-party GitHub Actions (`upload-artifact@v3`, `codecov-action@v3`) |
| [#139](https://github.com/fanilabs/fanilab-smartcontract/issues/139) | `deploy-testnet.yml`'s artifact-upload path patterns never match `deploy-all-contracts.sh`'s actual output filename |
| [#140](https://github.com/fanilabs/fanilab-smartcontract/issues/140) | `deploy-all-contracts.sh`'s `deploy_contract()` captures decorative echo output into `$contract_id`, corrupting the JSON output file |
| [#141](https://github.com/fanilabs/fanilab-smartcontract/issues/141) | `ci.yml`'s outdated-dependency check is configured with `continue-on-error: true`, so it can never actually fail the build |
| [#142](https://github.com/fanilabs/fanilab-smartcontract/issues/142) | `security-audit.yml` only runs `cargo deny check advisories`, never enforcing `deny.toml`'s license and dependency-ban rules |
| [#143](https://github.com/fanilabs/fanilab-smartcontract/issues/143) | `release.yml` builds and publishes a GitHub Release without ever running the test suite |
| [#144](https://github.com/fanilabs/fanilab-smartcontract/issues/144) | No CI step measures or enforces the 64 KB WASM contract-size limit |

**All issues have been filed. Zero unpublished issues remain in this document — the local backlog is empty.**

---

## Summary by Label

All findings from all three review passes have been filed to GitHub with real repository labels (`bug`, `feature`, `enhancement`, `refactor`, `documentation`, `test`, `security`, `performance`); none remain tracked by local label counts in this document. See the labels on each issue at `github.com/fanilabs/fanilab-smartcontract/issues`.

(Issues #7–#144 were filed to GitHub across three review passes — see `Pushed to GitHub` above — and no longer appear here.)

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
| Docs (`docs/`, root `*.md`) | — | #64, #65, #66, #67, #78, #79, #81, #86, #126, #127, #128, #129, #130, #131, #132 |
| CI/CD (`.github/workflows/`) | — | #56, #57, #62, #63, #137, #138, #139, #141, #142, #143, #144 |
| Scripts/tooling (`scripts/`, `Makefile`, `.env.example`) | — | #56, #57, #58, #59, #60, #133, #140 |
| Cross-cutting / process | — | #7, #8, #19, #27, #31, #34, #36, #46, #47, #52, #53, #77, #80, #114, #115, #125, #134, #135, #136 |

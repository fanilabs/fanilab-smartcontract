# Changelog

All notable changes to FaniLab Smart Contracts will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `escrow_contract::clear_fleet_management_contract(admin)` — admin-gated, mirrors `clear_settlement_contract`: unsets a configured fleet-management contract so `get_fleet_management_contract` returns `None` and payouts for fleet-linked escrows go directly to the driver; a no-op that still succeeds when nothing is configured (Issue #239). No `clear_dispute_resolution_contract` counterpart is provided — clearing it would permanently disable `freeze_funds`; the documented remedy is to repoint via `set_dispute_resolution_contract` (decision recorded in `docs/API.md`)
- `dispute_resolution_contract`: `add_admin` / `remove_admin` now emit `admin_added` / `admin_removed` events carrying `(caller, affected_admin)` so roster changes are observable on-chain (Issue #212)
- Full test module for `dispute_resolution_contract::force_resolve_dispute` — timing boundaries (before / exactly at / after the resolution window), authorization for each delivery party and a non-party, the open-dispute precondition, populated `resolved_at`/`resolved_by`, and a near-`u64::MAX` resolution limit that no longer overflows (Issue #213)
- Regression tests that `resolve_dispute_refund_sender` and `resolve_dispute_pay_driver` reject a non-`Paused` escrow before any state mutation or reputation adjustment, plus coverage that all three resolution entry points still succeed against a `Paused` escrow (Issue #211)
- Production-ready CI/CD pipeline with security audits
- CI: coverage upload now fails the build on error (`fail_ci_if_error: true`)
- CI: `cargo machete` step to detect unused dependencies automatically
- CI: `cargo outdated` step is now a hard gate (removed `continue-on-error`)
- Release: `release.yml` now validates that the pushed `vX.Y.Z` tag matches the version declared by every workspace crate (and that the crates agree with each other) before anything is built or published; a mismatch fails the workflow with a message naming both the tag and the manifest version (#246)
- Release: the "Contracts" list in the generated release notes is now derived from the built `*.wasm` artifacts — the same directory the checksum block is generated from — instead of a hand-maintained list, so it stays correct as workspace members are added or removed (#245)
- CI: all four workflows now declare explicit least-privilege `permissions` blocks — `contents: write` for the release job, `contents: read` everywhere else — so they behave correctly under either a read-only or read-write default token and bound the blast radius of a compromised action (#244)
- `identity_reputation_contract::has_driver_profile` query function for driver existence checks
- `shared_types::ttl` constants (`LEDGER_TTL_THRESHOLD`, `LEDGER_TTL_EXTEND_TO`) now used by `delivery_contract`, `dispute_resolution_contract`, `identity_reputation_contract`, and `fleet_management_contract` instead of hand-typed `518400, 518400` literals at every `extend_ttl` call site
- `fleet_management_contract::confirm_fleet_treasury_update` and `get_pending_treasury_update` to support a timelocked treasury change flow
- `delivery_contract::get_escrow_contract` — a read-only getter for the configured escrow contract address, matching the equivalent getter every other contract that stores a peer-contract address already had
- Regression tests proving the FA-2 dispute-resolution-bypass fix holds: an unauthorized caller cannot invoke `escrow_contract::freeze_funds`, and `delivery_contract::cancel_delivery` is rejected once a delivery has reached `Disputed`
- Cross-contract integration tests: a full delivery → escrow → dispute_resolution → identity_reputation chain asserting a driver's reputation score actually decreases on `resolve_dispute_refund_sender`, and a full escrow → fleet_management chain asserting an active fleet driver's payout is routed to the fleet treasury rather than the driver directly
- Regression tests for the `Holdback` refund invariant, including a full delivery → escrow → identity_reputation chain that confirms a delivery, asserts the escrow reaches `Holdback` with the driver's reputation credited, and proves the sender can no longer reclaim the funds — plus coverage that the admin refund, dispute/freeze, holdback release, and `Locked`-state refund paths all still work

### Changed
- `fleet_management_contract` and `identity_reputation_contract` now use `shared_types::is_admin`/`StorageKey::Admin` instead of their own local single-admin helpers, matching `escrow_contract` and `delivery_contract` (see ADR-011's addendum for why `shared_types::governance::AdminManager` was removed instead of adopted)
- `Makefile.windows` now builds against `wasm32v1-none` instead of the legacy `wasm32-unknown-unknown` target; CI's wasm-target drift check now also scans `Makefile.windows` and `.vscode/`
- `escrow_contract::create_escrow` now validates `token` matches the protocol-configured token
- `fleet_management_contract::register_fleet` checks driver profile existence before calling `register_driver`, preventing panic for already-registered drivers
- `fleet_management_contract::update_fleet_treasury` now only *proposes* a treasury change; it takes effect only after a 3-day timelock and an explicit `confirm_fleet_treasury_update` call, giving active drivers advance notice via the new `fleet_treasury_change_proposed` event
- `settlement_contract` source moved from `src/lib.rs` to the flat `lib.rs` layout used by the other five contracts, for structural consistency
- Replaced the crate-level `#![allow(deprecated)]` in all six contracts with `#[allow(deprecated)]` scoped to the individual functions that call the deprecated `events().publish()`, so future unrelated deprecations are no longer silently suppressed
- `release.yml` and `deploy-testnet.yml` now pass `--locked` to `cargo build`, matching `ci.yml`, so every build path resolves the committed `Cargo.lock` exactly (and fails instead of silently updating it if the lockfile is stale). The published WASM artifacts and their recorded checksums now correspond to the dependency set CI tested, and rebuilding a tag reproduces the same bytes (#242)
- `escrow_contract::constants::PROTOCOL_VERSION` now carries a doc comment stating that it is deliberately independent of the crate/package version and the release tag: the crate version and tag track source/artifact history and must agree, while `PROTOCOL_VERSION` only bumps on an on-chain-observable protocol change (#246)
- Enhanced CI pipeline with linting and testing
- Improved error handling across all contracts
- Optimized storage TTL management
- **Upgraded Soroban SDK from 22.0.1 to 27.0.0** with full ecosystem compatibility
- **Updated WASM build target from `wasm32-unknown-unknown` to `wasm32v1-none`** for Soroban SDK 27.0.0 compatibility
- **Pinned Rust toolchain to 1.81.0** in CI workflows for consistent compilation across environments
- Added `#[allow(deprecated)]` annotations for SDK 27.0.0 `env.events().publish()` API deprecation (remains functional)

### Fixed
- `EscrowRecord` now carries a `delivery_id` field matching the delivery-side record, and every escrow creation path populates it with the correct id so `get_escrow(id)` and paired delivery/escrow state lookups stay self-describing and consistent (Issue #285)
- `dispute_resolution_contract::init` now guards on `AdminList` instead of the unrelated `DeliveryContract` key, preventing re-initialization from depending on a fragile peer-contract invariant (Issue #286)
- `escrow_contract::create_escrows_batch` now increments `TotalLocked(token)` by the sum of the batch, matching `create_escrow`'s fund accounting so `sweep_untracked_balance` can no longer drain batch-created escrows as "untracked" surplus (Issue #188)
- `escrow_contract::create_escrows_batch` now enforces the same guards as `create_escrow`: the batch token must match `ProtocolConfig::token` (`InvalidToken`) and every element amount must be positive (`InvalidAmount`) (Issue #189)

### Removed
- **BREAKING:** `FaniLabError::EscrowLocked` (discriminant 7) and `FaniLabError::InvalidAddress` (discriminant 10) — dead error variants never returned by any contract in the workspace. Off-chain code matching on these discriminant values should be updated; the numeric codes are not reused by other variants.
- `escrow_contract::get_status` — dead stub that always returned `DeliveryStatus::Pending`. Use `get_escrow(id).status` instead.
- Comprehensive deployment documentation
- API reference documentation
- Security audit checklist
- Testing guide with coverage requirements
- Governance model documentation
- Issue and PR templates
- Automated dependency updates via Dependabot
- Code formatting standards (rustfmt.toml)
- License compliance checking (cargo-deny)
- Automated release workflow
- Settlement contract integration for currency swaps
- Two-step admin transfer process
- Dispute split resolution mechanism
- Driver reputation tracking system
- Delivery transit status tracking

### Security
- **`escrow_contract::refund_escrow` now treats `Holdback` as an admin-only refund state.** Once the recipient confirms delivery, `delivery_contract::confirm_delivery` moves the escrow to `Holdback` and the driver is credited reputation for the completed delivery. `refund_escrow` accepted `Holdback` as a refundable state but gated only `Paused` behind the admin check, so the sender could still call it directly on the escrow contract and reclaim the full amount after taking delivery — leaving the driver unpaid while their reputation credit stood. `Holdback` is now gated exactly like `Paused` (Issue #93 / FA-2): the admin arbitration and dispute paths out of `Holdback` are unchanged, and refunds from `Locked` — including `delivery_contract::cancel_delivery` — are unaffected
- Added balance verification before transfers
- Implemented checks-effects-interactions pattern
- Enhanced access control on admin functions
- Added input validation on all public functions

## [0.2.0] - 2024-12-XX

### Added
- Delivery contract with full lifecycle management
- Escrow contract with dispute resolution
- Shared types library for cross-contract compatibility
- Event system for off-chain indexing
- Basic test coverage

### Fixed
- Storage key collision issues
- State transition validation bugs

## [0.1.0] - 2024-11-XX

### Added
- Initial project structure
- Basic escrow functionality
- Cargo workspace configuration
- README and contributing guidelines

---

## Release Process

1. Update CHANGELOG.md with changes
2. Update version in Cargo.toml files
3. Create git tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
4. Push tag: `git push origin v1.0.0`
5. GitHub Actions will create release automatically

## Version Guidelines

- **Major (X.0.0)**: Breaking changes, major features
- **Minor (0.X.0)**: New features, non-breaking changes
- **Patch (0.0.X)**: Bug fixes, minor improvements

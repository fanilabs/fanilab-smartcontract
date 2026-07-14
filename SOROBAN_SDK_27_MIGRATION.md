# Soroban SDK 27.0.0 Migration Summary

## Overview
This document summarizes the migration of FaniLab Smart Contracts from Soroban SDK 21.7.3 to SDK 27.0.0, ensuring compatibility with the latest Rust stable toolchain (1.94.1) and the modern Soroban ecosystem.

## Changes Made

### 1. SDK Version Upgrade
- **Before**: Soroban SDK 21.7.3
- **After**: Soroban SDK 27.0.0
- **Location**: `Cargo.toml` (workspace root)

### 2. WASM Target Migration
- **Before**: `wasm32-unknown-unknown`
- **After**: `wasm32v1-none`
- **Reason**: Soroban SDK 27.0.0 requires the new `wasm32v1-none` target

**Installation Command**:
```bash
rustup target add wasm32v1-none
```

### 3. Build Commands Updated
**Before**:
```bash
cargo build --target wasm32-unknown-unknown --release
```

**After**:
```bash
cargo build --target wasm32v1-none --release
```

### 4. Event Publishing API Deprecation
The `env.events().publish()` API is deprecated in SDK 27.0.0 but remains functional. Added `#[allow(deprecated)]` annotations to all contract modules to suppress warnings until the SDK team provides a stable replacement.

**Files Modified**:
- `contracts/delivery_contract/lib.rs`
- `contracts/escrow_contract/lib.rs`
- `contracts/dispute_resolution_contract/lib.rs`
- `contracts/identity_reputation_contract/lib.rs`
- `contracts/fleet_management_contract/lib.rs`
- `contracts/settlement_contract/src/lib.rs`

### 5. Unused Variable Fixes
Fixed unused variable warnings in `settlement_contract` by prefixing unused parameters with underscore (`_`).

### 6. CI/CD Workflow Updates
Updated all GitHub Actions workflows to use the correct toolchain and target:

#### `.github/workflows/ci.yml`:
- Target changed from `wasm32-unknown-unknown` to `wasm32v1-none`
- Rust toolchain: `stable` (latest)
- Added `--target wasm32v1-none` to clippy command

#### `.github/workflows/deploy-testnet.yml`:
- Target changed to `wasm32v1-none`
- Updated Stellar CLI installation to use latest version with `--features opt`
- Removed wasm-opt installation (no longer needed with new target)

#### `.github/workflows/release.yml`:
- Target changed to `wasm32v1-none`
- Simplified WASM optimization step

#### `.github/workflows/security-audit.yml`:
- No target-specific changes needed (uses stable toolchain)

### 7. Dependency Updates
Regenerated `Cargo.lock` with updated dependencies compatible with SDK 27.0.0:
- `stellar-xdr`: Updated to 27.0.0
- `soroban-env-common`: Updated to 27.0.0
- `soroban-env-host`: Updated to 27.0.0
- `soroban-env-guest`: Updated to 27.0.0
- And all related Soroban ecosystem crates

## Verification

### Build Status
✅ **Success**: All contracts build successfully with the new SDK

```bash
cargo build --target wasm32v1-none --release
```

### Formatting
✅ **Success**: Code formatting passes

```bash
cargo fmt --all -- --check
```

### Testing
Tests compile and run with the new SDK. Note: Test compilation may take longer due to increased dependency tree size in SDK 27.0.0.

## Breaking Changes

### None for Existing Contracts
The migration preserves all existing contract functionality. No API changes were required beyond the internal SDK updates.

### Event Publishing
While the `env.events().publish()` method is marked as deprecated, it continues to work correctly in SDK 27.0.0. The `#[contractevent]` macro approach is not yet fully functional in the current SDK version.

## Migration Checklist for Other Projects

If you're migrating your own Soroban project to SDK 27.0.0, follow these steps:

- [ ] Update `Cargo.toml` workspace dependencies to `soroban-sdk = "27.0.0"`
- [ ] Install the new WASM target: `rustup target add wasm32v1-none`
- [ ] Update all build commands to use `--target wasm32v1-none`
- [ ] Add `#[allow(deprecated)]` to contract modules if using `env.events().publish()`
- [ ] Update CI/CD workflows to use `wasm32v1-none` target
- [ ] Regenerate `Cargo.lock`: `cargo update`
- [ ] Run `cargo build --target wasm32v1-none --release` to verify
- [ ] Run `cargo test` to ensure tests pass
- [ ] Run `cargo clippy` to check for any new warnings

## Known Issues

### Event System Migration
The Soroban SDK team is working on a new event system using the `#[contractevent]` macro. However, as of SDK 27.0.0, this system is not fully functional. The deprecated `env.events().publish()` API remains the recommended approach until further notice.

### Test Compilation Time
SDK 27.0.0 has a larger dependency tree, which may result in longer initial compilation times for tests. Subsequent builds benefit from caching.

## Resources

- [Soroban SDK 27.0.0 Release Notes](https://github.com/stellar/rs-soroban-sdk/releases/tag/v27.0.0)
- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Examples](https://github.com/stellar/soroban-examples)

## Support

For issues related to this migration, please:
1. Check the [Stellar Discord](https://discord.gg/stellar) #soroban channel
2. Review [Soroban SDK GitHub Issues](https://github.com/stellar/rs-soroban-sdk/issues)
3. Consult the [Stellar Stack Exchange](https://stellar.stackexchange.com/)

---

**Migration Date**: 2026-07-14  
**Rust Version**: 1.94.1 (stable)  
**Soroban SDK Version**: 27.0.0  
**Migration Status**: ✅ Complete

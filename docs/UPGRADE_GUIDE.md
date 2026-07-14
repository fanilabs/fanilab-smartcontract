# Contract Upgrade Guide

Guide for upgrading FaniLab smart contracts on Stellar Soroban.

## Overview

Soroban contracts are upgradeable by design. This guide covers safe upgrade procedures.

## Pre-Upgrade Checklist

- [ ] New version tested thoroughly on Testnet
- [ ] Security audit completed (for major changes)
- [ ] Migration path documented
- [ ] Rollback plan prepared
- [ ] Users notified (48 hours advance notice)
- [ ] Downtime window scheduled (if needed)
- [ ] Backup of current state taken

## Upgrade Types

### 1. Bug Fix (Patch)
- Minor bug fixes
- No state changes
- No interface changes

### 2. Feature Addition (Minor)
- New functions added
- Existing functions unchanged
- May add new storage keys

### 3. Breaking Change (Major)
- Interface changes
- State migration required
- Requires coordination with frontend/backend

## Upgrade Process

### Step 1: Build New Version
```bash
# Update version in Cargo.toml
# Build optimized WASM
cargo build --target wasm32-unknown-unknown --release
wasm-opt -Oz target/wasm32-unknown-unknown/release/contract.wasm -o contract_v2.wasm
```

### Step 2: Deploy to Testnet
```bash
stellar contract deploy \
  --wasm contract_v2.wasm \
  --source deployer \
  --network testnet
```

### Step 3: Test Thoroughly
- Run full test suite
- Test migration if needed
- Verify all functions
- Check gas usage

### Step 4: Deploy to Mainnet
```bash
stellar contract deploy \
  --wasm contract_v2.wasm \
  --source deployer \
  --network mainnet
```

### Step 5: Update References
Update contract addresses in:
- Frontend applications
- Backend services
- Documentation
- Monitoring systems

### Step 6: Verify
```bash
# Test critical functions
stellar contract invoke --id $NEW_CONTRACT_ID --network mainnet -- get_admin
```

## State Migration

If state structure changes:

```rust
// Add migration function
pub fn migrate_to_v2(env: Env) {
    let admin = get_admin(&env);
    admin.require_auth();
    
    // Read old state
    let old_data = load_old_format(&env);
    
    // Transform to new format
    let new_data = transform(old_data);
    
    // Save new state
    save_new_format(&env, new_data);
}
```

## Rollback Procedure

If issues discovered:

1. **Assess Impact**: Determine severity
2. **Pause if Possible**: Call pause function if available
3. **Notify Users**: Immediate communication
4. **Deploy Previous Version**: Redeploy working version
5. **Update References**: Point back to old contract
6. **Post-Mortem**: Document what went wrong

## Version Compatibility

### Contract Versions
- Major: Breaking changes (1.0.0 → 2.0.0)
- Minor: New features (1.0.0 → 1.1.0)
- Patch: Bug fixes (1.0.0 → 1.0.1)

### Frontend Compatibility
Maintain compatibility matrix:
| Contract | Frontend | Compatible |
|----------|----------|------------|
| 1.0.x    | 1.0.x    | ✅         |
| 1.0.x    | 1.1.x    | ✅         |
| 2.0.x    | 1.x.x    | ❌         |

## Testing Upgrades

```rust
#[test]
fn test_upgrade_preserves_state() {
    // Deploy v1
    let v1_id = deploy_contract_v1(&env);
    
    // Set some state
    v1.init(&admin, &token, &250);
    
    // Deploy v2 at same address
    let v2_id = deploy_contract_v2(&env);
    
    // Verify state preserved
    assert_eq!(v2.get_admin(), admin);
}
```

## Communication Plan

### Pre-Upgrade (48h before)
- Twitter announcement
- Discord notification
- Email to active users
- Update status page

### During Upgrade
- Real-time updates
- Expected downtime
- Progress updates

### Post-Upgrade
- Success announcement
- New features highlighted
- Support channels open

## Emergency Upgrades

For critical security fixes:
1. Deploy fix immediately
2. Notify users during deployment
3. Post-mortem within 24h
4. Public disclosure after 90 days

---

**Last Updated**: January 2026

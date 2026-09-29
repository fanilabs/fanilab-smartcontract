# Contract Migration Guide

> **Scope:** This guide covers **state migration** — transforming on-chain storage when a contract upgrade changes data structures. It does not cover the mechanical process of deploying a new WASM binary; that is handled in the [Upgrade Guide](UPGRADE_GUIDE.md).
>
> **Related document:** [UPGRADE_GUIDE.md](UPGRADE_GUIDE.md) — the deployment steps that precede or accompany state migration (building, deploying, testing, rollback procedures). Read both documents together when performing a breaking-change upgrade.

## Overview

When upgrading Soroban contracts, you may need to transform existing state to match new data structures. This guide provides tested patterns for performing these migrations safely.

> **Note on `migrate_to_v2`:** The `migrate_to_v2` function pattern shown in this guide is **aspirational**. No production contract in this repository currently implements it or calls `update_current_contract_wasm`. Implementing migration tooling is tracked in issue [#81](https://github.com/fanilabs/fanilab-smartcontract/issues/81). The patterns below document the intended design so they can be implemented correctly when that work lands.

## Migration Pattern

The basic pattern for contract migration involves three steps:

1. **Read old state** from persistent or instance storage
2. **Transform to new format** using your migration logic
3. **Save new state** back to storage

## Example: Migrating Protocol Configuration

### Scenario

You're upgrading your escrow contract and need to add a new field `max_escrow_amount` to the `ProtocolConfig` struct. Existing contracts have the old format without this field.

### Migration Function

```rust
pub fn migrate_to_v2(env: Env) {
    let admin = get_admin(&env);
    admin.require_auth();

    // Read old configuration
    let old_config: ProtocolConfig = env
        .storage()
        .instance()
        .get(&StorageKey::ProtocolConfig)
        .expect("Config not found");

    // Transform to new format with default for new field
    let new_config = ProtocolConfigV2 {
        token: old_config.token,
        platform_fee_bps: old_config.platform_fee_bps,
        protocol_version: 2,
        slippage_tolerance_bps: old_config.slippage_tolerance_bps,
        max_escrow_amount: 1_000_000_000_000, // Default max 1B
    };

    // Save new configuration
    env.storage()
        .instance()
        .set(&StorageKey::ProtocolConfigV2, &new_config);

    env.events().publish(
        (Symbol::new(&env, "ProtocolMigrated"),),
        (admin, 2u32),
    );
}
```

### Migration Test

Always include a test that:
1. Seeds old-format state
2. Invokes the migration function
3. Asserts the new format is correctly populated with no data loss

```rust
#[test]
fn test_protocol_config_migration_preserves_data() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin).address();

    // Set up old state
    client.init(&admin, &token, &250);

    let original_fee = client.get_platform_fee();
    let original_slippage = client.get_slippage_tolerance();

    // Simulate migration
    client.migrate_to_v2(&admin);

    // Verify state is preserved
    assert_eq!(client.get_platform_fee_v2(), original_fee);
    assert_eq!(client.get_slippage_tolerance_v2(), original_slippage);
    assert_eq!(client.get_protocol_version(), 2);
}
```

## Best Practices

1. **Always require admin authorization** - Migrations should require the admin's signature to prevent unauthorized state changes
2. **Include a migration version** - Track which migration version the contract is on to handle multiple upgrades
3. **Emit a migration event** - Log when migrations occur for observability
4. **Test data preservation** - Verify that no data is lost during transformation
5. **Provide rollback plan** - Document how to recover if migration fails
6. **Backward-compatible reads** - Support reading both old and new formats during transition if needed

## Common Patterns

### Multi-Step Migration

For complex migrations involving multiple entities:

```rust
pub fn migrate_to_v2_batch(env: Env) {
    let admin = get_admin(&env);
    admin.require_auth();

    // Migrate global config
    migrate_protocol_config(&env);

    // Migrate all delivery records
    for delivery_id in get_all_delivery_ids(&env) {
        migrate_delivery_record(&env, delivery_id);
    }

    // Mark migration as complete
    env.storage()
        .instance()
        .set(&DataKey::MigrationVersion, &2u32);
}
```

### Gradual Migration

For very large state, consider migrating in stages:

```rust
pub fn migrate_batch(env: Env, start_index: u32, batch_size: u32) {
    let admin = get_admin(&env);
    admin.require_auth();

    let delivery_ids = get_all_delivery_ids(&env);
    let end_index = (start_index + batch_size).min(delivery_ids.len());

    for i in start_index..end_index {
        if let Some(delivery_id) = delivery_ids.get(i) {
            migrate_delivery_record(&env, delivery_id);
        }
    }

    // Save progress
    env.storage()
        .instance()
        .set(&DataKey::MigrationProgress, &end_index);
}
```

## Testing Checklist

- [ ] Migration function requires auth
- [ ] Old state is correctly read
- [ ] New state is correctly transformed
- [ ] New state is correctly saved
- [ ] No data is lost in transformation
- [ ] Migration event is emitted
- [ ] Version field is updated
- [ ] Subsequent operations use new format
- [ ] Rollback procedure is documented

## Deployment Steps

1. **Deploy new contract code** with migration function
2. **Run migration** via admin-authorized transaction
3. **Verify state** by reading key values
4. **Run full test suite** against live state
5. **Monitor events** for any issues
6. **Keep old code available** for rollback if needed

## Recovery Procedure

If migration fails or needs to be rolled back:

1. **Pause the protocol** to prevent new transactions
2. **Deploy previous contract version** if possible
3. **Restore from backup** if state is corrupted
4. **Document the incident** for post-mortem analysis
5. **Design improved migration** for next attempt

---

**Last Updated**: July 2026

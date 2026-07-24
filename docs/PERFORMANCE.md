# Performance Optimization Guide

## Overview

This guide covers performance optimization for FaniLab Smart Contracts on Stellar Soroban.

## Soroban Resource Model

### Resources Tracked
1. **CPU Instructions**: Computational cost
2. **Memory**: RAM usage during execution
3. **Storage**: Ledger entry reads/writes
4. **Events**: Event emission overhead

### Limits
- Max contract size: 64 KB (WASM)
- Max instructions per invocation: Configurable
- Max memory: 40 MB
- Max ledger entries: 256 per transaction

## Optimization Strategies

### 1. Minimize Storage Operations

**Bad**:
```rust
for i in 0..100 {
    env.storage().persistent().set(&i, &value);
}
```

**Good**:
```rust
let batch: Vec<Value> = collect_values();
env.storage().persistent().set(&key, &batch);
```

### 2. Use Appropriate Storage Types

- **Instance**: Contract configuration, admin
- **Persistent**: User data, delivery records
- **Temporary**: Session data (not used currently)

### 3. Optimize Data Structures

**Before**:
```rust
pub struct DeliveryRecord {
    pub field1: String,
    pub field2: String,
    pub field3: String,
    // Many strings = large storage
}
```

**After**:
```rust
pub struct DeliveryRecord {
    pub field1: Symbol,  // Smaller than String
    pub field2: u64,     // Compact integers
    // Only essential fields
}
```

### 4. Batch Operations

Group related operations:
```rust
pub fn batch_create_deliveries(env: Env, deliveries: Vec<DeliveryMetadata>) {
    for metadata in deliveries {
        // Process in single transaction
    }
}
```

### 5. Lazy Loading

Don't load unnecessary data:
```rust
// Only fetch what you need
pub fn get_delivery_status(env: Env, id: DeliveryId) -> DeliveryStatus {
    let record: DeliveryRecord = env.storage().persistent().get(&id)?;
    record.status  // Don't return full record
}
```

## Gas Profiling

### Measure Function Costs

```bash
# Deploy to testnet
stellar contract deploy --wasm contract.wasm --network testnet

# Invoke with fee tracking
stellar contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --fee 1000000 \
  -- function_name

# Check transaction for actual fee used
```

### Benchmark Tests

```rust
#[test]
fn bench_create_delivery() {
    let env = Env::default();
    let start_instructions = env.cpu_instructions();
    
    contract.create_delivery(...);
    
    let end_instructions = env.cpu_instructions();
    println!("Instructions used: {}", end_instructions - start_instructions);
}
```

## Contract Size Optimization

### 1. Remove Dead Code

```bash
# Check for unused code
cargo +nightly udeps
```

### 2. Optimize Build

```toml
[profile.release]
opt-level = "z"          # Optimize for size
lto = true               # Link-time optimization
codegen-units = 1        # Better optimization
strip = "symbols"        # Remove debug symbols
```

### 3. WASM Optimization

```bash
# Use wasm-opt
wasm-opt -Oz input.wasm -o output.wasm

# Verify size reduction
ls -lh *.wasm
```

## Memory Optimization

### 1. Avoid Large Vectors

```rust
// Avoid unbounded collections
pub fn process_all_deliveries(env: Env) {
    // Bad: Loads everything into memory
    let all: Vec<Delivery> = load_all(&env);
}

// Better: Process in chunks
pub fn process_delivery_batch(env: Env, start: u64, limit: u32) {
    for id in start..(start + limit as u64) {
        process_delivery(&env, id);
    }
}
```

### 2. Reuse Allocations

```rust
// Reuse vec instead of creating new ones
let mut buffer = Vec::new();
for item in items {
    buffer.clear();
    buffer.push(item);
    process(&buffer);
}
```

## Cross-Contract Call Optimization

### 1. Minimize Calls

```rust
// Bad: Multiple calls
let escrow1 = escrow.get_escrow(&id1);
let escrow2 = escrow.get_escrow(&id2);
let escrow3 = escrow.get_escrow(&id3);

// Better: Batch query (if supported)
let escrows = escrow.get_escrow_batch(&[id1, id2, id3]);
```

### 2. Cache Results

```rust
// Cache frequently accessed external data
let escrow_address = get_cached_escrow_address(&env);
```

## TTL Optimization

### Extend Smartly

```rust
// Don't extend on every read
if needs_extension(&env, &key) {
    env.storage().persistent().extend_ttl(&key, threshold, extend_to);
}

fn needs_extension(env: &Env, key: &StorageKey) -> bool {
    let ttl = env.storage().persistent().get_ttl(key);
    ttl < THRESHOLD
}
```

## Event Optimization

### 1. Compact Events

```rust
// Instead of full records, emit IDs
env.events().publish(
    (Symbol::new(&env, "delivery_created"),),
    delivery_id  // Just the ID, not entire record
);
```

### 2. Batch Events

```rust
// Single event for batch operation
env.events().publish(
    (Symbol::new(&env, "deliveries_processed"),),
    (start_id, end_id, count)
);
```

## Performance Testing

### Load Testing

```rust
#[test]
fn load_test_create_delivery() {
    let env = Env::default();
    
    for i in 0..1000 {
        let result = contract.create_delivery(...);
        assert!(result.is_ok());
    }
}
```

### Stress Testing

```rust
#[test]
fn stress_test_concurrent_operations() {
    // Test with many simultaneous deliveries
    // Test with large amounts
    // Test with rapid state transitions
}
```

## Monitoring Performance

### Key Metrics
- Average instructions per function
- Peak memory usage
- Storage growth rate
- Cross-contract call latency

### Alerts
- Function using > 100M instructions
- Storage entry > 32 KB
- Memory spike > 10 MB

## Best Practices

1. **Profile First**: Measure before optimizing
2. **Optimize Hot Paths**: Focus on frequently called functions
3. **Test Impact**: Verify optimization helps
4. **Document Tradeoffs**: Note readability vs performance
5. **Benchmark Regularly**: Track performance over time

## Common Pitfalls

### ❌ Avoid
- Unbounded loops
- Deep recursion
- Large string concatenation
- Excessive cloning
- Unnecessary storage reads

### ✅ Prefer
- Bounded iterations
- Iterative solutions
- Pre-allocated buffers
- Reference passing
- Cached values

## Performance Checklist

- [ ] Contract size < 60 KB
- [ ] No unbounded loops
- [ ] Storage operations minimized
- [ ] TTL management efficient
- [ ] Events are compact
- [ ] Cross-contract calls optimized
- [ ] Memory usage profiled
- [ ] Load tested with realistic data

---

**Last Updated**: January 2026

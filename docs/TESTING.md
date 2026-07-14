# Testing Guide

Comprehensive testing guide for FaniLab Smart Contracts.

## Overview

FaniLab uses a multi-layered testing approach:
1. **Unit Tests** - Test individual functions in isolation
2. **Integration Tests** - Test contract interactions
3. **Property-Based Tests** - Test invariants and edge cases
4. **Fuzzing** - Discover unexpected behaviors
5. **Gas Profiling** - Optimize resource usage

## Running Tests

### Quick Test
```bash
cargo test
```

### Verbose Output
```bash
cargo test -- --nocapture
```

### Specific Contract
```bash
cargo test -p escrow_contract
cargo test -p delivery_contract
```

### Single Test
```bash
cargo test test_name
```

## Unit Testing

### Test Structure
```rust
#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn test_function_name() {
        // Setup
        let env = Env::default();
        
        // Execute
        let result = function_under_test();
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Example: Escrow Test
```rust
#[test]
fn test_create_and_release_escrow() {
    let env = Env::default();
    let contract = EscrowContractClient::new(&env, &contract_id);
    
    // Initialize
    contract.init(&admin, &token, &250);
    
    // Create escrow
    contract.create_escrow(&sender, &recipient, &driver, &1, &token, &1000);
    
    // Release
    contract.release_escrow(&recipient, &1);
    
    // Verify
    let escrow = contract.get_escrow(&1);
    assert_eq!(escrow.status, EscrowState::Released);
}
```

## Coverage

### Install Coverage Tool
```bash
cargo install cargo-tarpaulin
```

### Generate Coverage Report
```bash
cargo tarpaulin --out Html --output-dir coverage
```

### View Report
```bash
# Open coverage/index.html in browser
```

### Coverage Goals
- **Overall**: > 80%
- **Critical paths**: 100%
- **Error handling**: > 90%

## Integration Testing

Tests for cross-contract interactions.

### Test File Structure
```
tests/
  ├── integration_tests/
  │   ├── delivery_escrow_flow.rs
  │   ├── dispute_resolution.rs
  │   └── settlement_integration.rs
```

### Example
```rust
#[test]
fn test_full_delivery_flow() {
    let env = Env::default();
    
    // Deploy contracts
    let escrow = deploy_escrow(&env);
    let delivery = deploy_delivery(&env);
    
    // Create delivery
    let delivery_id = delivery.create_delivery(...);
    
    // Fund escrow
    escrow.create_escrow(..., delivery_id, ...);
    
    // Assign driver
    delivery.assign_driver(..., delivery_id, ...);
    
    // Complete delivery
    delivery.confirm_delivery(..., delivery_id);
    
    // Verify escrow released
    let escrow_rec = escrow.get_escrow(&delivery_id);
    assert_eq!(escrow_rec.status, EscrowState::Released);
}
```

## Security Testing

### Access Control Tests
```rust
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_unauthorized_admin_action() {
    let env = Env::default();
    let attacker = Address::generate(&env);
    
    contract.update_platform_fee(&attacker, &500);
}
```

### Reentrancy Tests
```rust
#[test]
fn test_no_reentrancy_on_release() {
    // Attempt reentrancy attack
    // Verify state updated before external call
}
```

### Integer Overflow Tests
```rust
#[test]
fn test_fee_calculation_max_values() {
    let max_amount = i128::MAX;
    let fee = calculate_fee(max_amount, 10000);
    assert!(fee > 0);
    assert!(fee < max_amount);
}
```

## Property-Based Testing

### Install PropTest
```bash
cargo add proptest --dev
```

### Example
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_fee_always_less_than_amount(
        amount in 1i128..1_000_000_000i128,
        fee_bps in 0u32..1000u32
    ) {
        let fee = calculate_fee(amount, fee_bps);
        prop_assert!(fee < amount);
        prop_assert!(fee >= 0);
    }
}
```

## Performance Testing

### Gas Profiling
```bash
cargo test --release -- --nocapture | grep "instructions"
```

### Benchmarking
```rust
#[test]
fn bench_create_delivery() {
    let start = std::time::Instant::now();
    
    for _ in 0..1000 {
        contract.create_delivery(...);
    }
    
    let duration = start.elapsed();
    println!("Avg time: {:?}", duration / 1000);
}
```

## Best Practices

1. **Test Names**: Descriptive, start with `test_`
2. **Isolation**: Each test independent
3. **Setup**: Use helper functions
4. **Assertions**: Clear, specific
5. **Edge Cases**: Test boundaries
6. **Error Cases**: Test all errors
7. **State Changes**: Verify all effects
8. **Events**: Assert events emitted

## Continuous Integration

Tests run automatically on:
- Every commit to PR
- Daily security audits
- Before deployment

See `.github/workflows/ci.yml`

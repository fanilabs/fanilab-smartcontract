# Testing

This document describes how the test suites are organized and how to run them.

## Running tests

### TypeScript SDK

```bash
cd sdk/typescript
npm test
```

### Contracts

```bash
cargo test
```

## SDK encoder parity tests

The TypeScript SDK includes parity tests under `sdk/typescript/src/__tests__` that
validate the SDK encoders against the compiled contract specifications.

### What is validated

For every exposed contract method, the parity tests compare the SDK encoder
output against the contract spec extracted from the compiled WASM binaries:

- **Argument count** — the number of arguments the SDK encoder produces must
  match the number of parameters declared in the contract spec.
- **Argument order** — arguments must be encoded in the same order as declared
  in the contract spec.
- **ScVal types** — each encoded argument must use the ScVal type declared for
  the corresponding parameter in the contract spec.

### Extracting contract specs from WASM

Soroban contract specs are embedded in the compiled WASM binaries as custom
sections. The parity tests parse these spec entries directly from the WASM
artifacts so that the SDK encoders are checked against the actual compiled
contract interface rather than a hand-maintained copy.

To regenerate the WASM artifacts used by the parity tests, build the contracts:

```bash
cargo build --target wasm32-unknown-unknown --release
```

The resulting `.wasm` files are consumed by the parity tests to extract the
contract spec entries.

### Adding coverage for a new method

When a new method is exposed by a contract, the parity test will automatically
pick it up from the extracted contract spec. If the SDK encoder for that method
does not match the spec (argument count, order, or ScVal types), the test will
fail with a message identifying the mismatched method and argument.

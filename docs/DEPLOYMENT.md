# FaniLab Smart Contract Deployment Guide

This guide covers the complete deployment process for FaniLab smart contracts on Stellar Soroban.

## Table of Contents
- [Prerequisites](#prerequisites)
- [Network Configuration](#network-configuration)
- [Building Contracts](#building-contracts)
- [Testing Before Deployment](#testing-before-deployment)
- [Deploying to Testnet](#deploying-to-testnet)
- [Deploying to Mainnet](#deploying-to-mainnet)
- [Contract Initialization](#contract-initialization)
- [Verification](#verification)
- [Monitoring](#monitoring)
- [Rollback Procedures](#rollback-procedures)

## Prerequisites

### Required Tools
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32v1-none

# Stellar CLI (version 21.5.0 or later)
cargo install --locked stellar-cli --version 21.5.0

# WASM optimizer (optional but recommended)
cargo install wasm-opt
```

### Environment Setup
```bash
# Copy environment template
cp .env.example .env

# Edit .env with your configuration
nano .env
```

### Environment Variable Precedence

Every deployment and initialization script resolves settings with a single,
uniform precedence order:

1. **Pre-set environment variables** (exported in the shell, or injected by CI)
   always win.
2. **Values from the sourced `.env` file** are used only when the corresponding
   variable is not already set in the environment.
3. **Built-in defaults** in the script apply when neither of the above is set.

In other words, `.env` never overrides an already-exported variable. This rule
applies to **every** setting the scripts read — including `DEPLOYER_IDENTITY`,
`STELLAR_NETWORK`, `SOROBAN_RPC_URL`, `NETWORK_PASSPHRASE`, `ADMIN_ADDRESS`,
`TOKEN_ADDRESS`, `PLATFORM_FEE_BPS`, and the dispute limits. There are no
special-cased variables resolved before or after `.env` is sourced.

Example — conflicting values:
```bash
# .env contains: DEPLOYER_IDENTITY=deployer-from-file
# Shell exports:  DEPLOYER_IDENTITY=deployer-from-env
export DEPLOYER_IDENTITY=deployer-from-env
./scripts/initialize-all-contracts.sh testnet
# The script uses "deployer-from-env" — the exported value wins.
```

To deliberately use a value from `.env` while it is also exported, unset the
exported variable first (`unset DEPLOYER_IDENTITY`) so the file value applies.

### Required Environment Variables
```env
# Network Configuration
STELLAR_NETWORK=testnet  # or mainnet
SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

# Deployment Keys
CONTRACT_DEPLOYER_SECRET=S...  # Your secret key
ADMIN_ADDRESS=G...  # Admin public key

# Contract Addresses (filled after deployment)
ESCROW_CONTRACT_ID=
DELIVERY_CONTRACT_ID=
DISPUTE_CONTRACT_ID=
FLEET_CONTRACT_ID=
IDENTITY_CONTRACT_ID=
SETTLEMENT_CONTRACT_ID=

# Token Configuration
TOKEN_ADDRESS=  # Token contract for escrow
PLATFORM_FEE_BPS=250  # 2.5% platform fee
DISPUTE_TIME_LIMIT=86400  # Optional; minimum time for raising a dispute (seconds)
DISPUTE_RESOLUTION_LIMIT=86400  # Optional; minimum dispute resolution window (seconds)
```

## Network Configuration

### Testnet Setup
```bash
# Generate deployment identity
stellar keys generate deployer --network testnet

# Get public address
stellar keys address deployer

# Fund the account (Testnet only)
stellar keys fund deployer --network testnet

# Verify balance
stellar account balance deployer --network testnet
```

### Mainnet Setup
```bash
# Generate mainnet identity (use hardware wallet in production)
stellar keys generate deployer --network mainnet

# Fund the account with real XLM
# Transfer funds to the generated address

# Verify balance
stellar account balance deployer --network mainnet
```

## Building Contracts

### Standard Build
```bash
# Build all contracts
make build

# Or using cargo directly
cargo build --target wasm32v1-none --release
```

### Optimized Build (Recommended for Mainnet)
```bash
# Build with optimizations
cargo build --target wasm32v1-none --release

# Optimize WASM files
for contract in target/wasm32v1-none/release/*.wasm; do
    wasm-opt -Oz "$contract" -o "$contract.opt"
    mv "$contract.opt" "$contract"
done
```

### Verify Build
```bash
# Check WASM file sizes
ls -lh target/wasm32v1-none/release/*.wasm

# Verify contract interfaces
stellar contract inspect --wasm target/wasm32v1-none/release/escrow_contract.wasm
```

## Testing Before Deployment

### Run Full Test Suite
```bash
# Run all tests
cargo test --verbose

# Run specific contract tests
cargo test -p escrow_contract
cargo test -p delivery_contract

# Run tests with coverage
cargo tarpaulin --out Html --output-dir coverage
```

### Integration Testing
```bash
# Deploy to local network for testing
stellar network start local

# Deploy contracts locally using existing scripts
./scripts/deploy-all-contracts.sh local

# Run contract tests
cargo test --verbose

# Stop local network
stellar network stop local
```

## Deploying to Testnet

### Deploy Individual Contract
```bash
# Deploy escrow contract
stellar contract deploy \
  --wasm target/wasm32v1-none/release/escrow_contract.wasm \
  --source deployer \
  --network testnet

# Save the returned contract ID
export ESCROW_CONTRACT_ID=<returned_contract_id>
```

### Deploy All Contracts
```bash
# Use the deployment script
./scripts/deploy-all-contracts.sh testnet

# Contract IDs will be saved to contract-ids-testnet.json
```

### CI Deployment (GitHub Actions)

The **Deploy to Testnet** workflow (`.github/workflows/deploy-testnet.yml`) is
`workflow_dispatch`-only. It runs against the `testnet` GitHub Environment and
expects one secret:

| Secret | Holds | Used by |
| ------ | ----- | ------- |
| `TESTNET_DEPLOYER_SECRET` | The deployer account's secret (`S...`) key | The workflow's *Configure deployer identity* step |

That step imports the secret (via stdin, so it never reaches the logs) as the
Stellar CLI identity **`deployer`** — the exact identity every deploy and
initialize script signs with (`--source deployer`). Nothing downstream reads the
raw secret; the scripts only ever reference the `deployer` identity.

The deploy and initialize scripts abort early with actionable guidance if that
identity is missing, so a run with an unset or blank `TESTNET_DEPLOYER_SECRET`
fails with a clear message instead of a raw CLI error. To point the scripts at a
differently named identity, export `DEPLOYER_IDENTITY` before invoking them.

`DEPLOYER_IDENTITY` follows the same precedence rule as every other setting
(see [Environment Variable Precedence](#environment-variable-precedence)): an
exported value wins over a value in `.env`, and the built-in default (`deployer`)
applies only when neither is set. It is not resolved before `.env` is sourced.

The local `.env` variable that carries this same key is `CONTRACT_DEPLOYER_SECRET`
(see `.env.example`); it is only a convenience for humans importing the identity
locally and is not consumed by the scripts directly.

### Deployment Order
The contracts should be deployed in this order due to dependencies:

1. **shared_types** (library, not deployed)
2. **escrow_contract**
3. **delivery_contract**
4. **identity_reputation_contract**
5. **fleet_management_contract**
6. **dispute_resolution_contract**
7. **settlement_contract**

## Deploying to Mainnet

### Pre-Mainnet Checklist
- [ ] All tests passing on Testnet
- [ ] Security audit completed
- [ ] Code freeze implemented
- [ ] Deployment runbook reviewed
- [ ] Rollback plan documented
- [ ] Monitoring systems configured
- [ ] Emergency contacts notified
- [ ] Budget for deployment fees confirmed

### Mainnet Deployment
```bash
# Set network to mainnet
export STELLAR_NETWORK=mainnet
export SOROBAN_RPC_URL=https://soroban-mainnet.stellar.org

# Deploy with production keys (use hardware wallet)
./scripts/deploy-all-contracts.sh mainnet

# Contract IDs will be saved to contract-ids-mainnet.json
```

### Post-Deployment Verification
```bash
# Verify each contract is deployed
stellar contract info \
  --id $ESCROW_CONTRACT_ID \
  --network mainnet

# Check contract WASM hash matches build
stellar contract fetch \
  --id $ESCROW_CONTRACT_ID \
  --network mainnet \
  --out-file deployed.wasm

# Compare with local build
sha256sum deployed.wasm
sha256sum target/wasm32v1-none/release/escrow_contract.wasm
```

## Contract Initialization

### Initialize Escrow Contract
```bash
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --source deployer \
  --network $STELLAR_NETWORK \
  -- init \
  --admin $ADMIN_ADDRESS \
  --token $TOKEN_ADDRESS \
  --platform_fee_bps 250
```

### Initialize Delivery Contract
```bash
stellar contract invoke \
  --id $DELIVERY_CONTRACT_ID \
  --source deployer \
  --network $STELLAR_NETWORK \
  -- init \
  --admin $ADMIN_ADDRESS \
  --escrow_contract $ESCROW_CONTRACT_ID
```

### Initialize All Contracts
```bash
# Use the initialization script
./scripts/initialize-all-contracts.sh $STELLAR_NETWORK
```

The script initializes all six contracts and configures their immediate peer
contract integrations. It requires all six contract IDs in the network's
`contract-ids-<network>.json` file, plus `ADMIN_ADDRESS` and `TOKEN_ADDRESS`.
The dispute limits default to 86400 seconds (one day) and cannot be set below
that minimum. The escrow-to-settlement integration is timelocked by the
contract; configure and confirm it separately after its delay has elapsed.

To initialize one contract instead, use
`./scripts/initialize-contract.sh <contract_name> $STELLAR_NETWORK`. Supported
names are `escrow_contract`, `delivery_contract`, `identity_reputation_contract`,
`fleet_management_contract`, `dispute_resolution_contract`, and
`settlement_contract`.

## Verification

### Verify Contract State
```bash
# Check escrow contract admin
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- get_admin

# Check platform fee
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- get_platform_fee
```

### Verify Contract Events
```bash
# Stream contract events
stellar events \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  --start-ledger <ledger_number>
```

## Monitoring

### Set Up Monitoring
```bash
# Monitor contract events
stellar events \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  --follow

# Monitor account balance
watch -n 60 'stellar account balance deployer --network $STELLAR_NETWORK'
```

### Key Metrics to Track
- Transaction success rate
- Contract invocation frequency
- Gas consumption per operation
- Error rates by contract
- Account balance levels

## Rollback Procedures

### Emergency Rollback
```bash
# 1. Pause contract (if supported)
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- pause

# 2. Notify stakeholders
# 3. Investigate issue
# 4. Deploy fix or restore from backup
```

### Contract Upgrade
```bash
# Deploy new WASM
stellar contract upload \
  --wasm target/wasm32v1-none/release/escrow_contract.wasm \
  --source deployer \
  --network $STELLAR_NETWORK

# Upgrade contract instance
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- upgrade \
  --new_wasm_hash <wasm_hash>
```

## Troubleshooting

### Common Issues

**Contract deployment fails:**
- Check account balance
- Verify network connectivity
- Ensure WASM file is valid

**Initialization fails:**
- Verify all contract IDs are set
- Check admin address is valid
- Ensure token contract is deployed

**Transaction fails:**
- Check gas limits
- Verify contract state
- Review error messages

### Getting Help
- Check [Stellar documentation](https://developers.stellar.org/)
- Open an issue on GitHub
- Contact the development team

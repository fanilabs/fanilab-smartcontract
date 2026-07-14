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
rustup target add wasm32-unknown-unknown

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
cargo build --target wasm32-unknown-unknown --release
```

### Optimized Build (Recommended for Mainnet)
```bash
# Build with optimizations
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM files
for contract in target/wasm32-unknown-unknown/release/*.wasm; do
    wasm-opt -Oz "$contract" -o "$contract.opt"
    mv "$contract.opt" "$contract"
done
```

### Verify Build
```bash
# Check WASM file sizes
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Verify contract interfaces
stellar contract inspect --wasm target/wasm32-unknown-unknown/release/escrow_contract.wasm
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

# Deploy contracts locally
./scripts/deploy-local-test.sh

# Run integration tests
cargo test --test integration_tests

# Stop local network
stellar network stop local
```

## Deploying to Testnet

### Deploy Individual Contract
```bash
# Deploy escrow contract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/escrow_contract.wasm \
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
sha256sum target/wasm32-unknown-unknown/release/escrow_contract.wasm
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

## Verification

### Contract State Verification
```bash
# Verify escrow admin
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- get_admin

# Verify platform fee
stellar contract invoke \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- get_platform_fee

# Verify delivery contract escrow reference
stellar contract invoke \
  --id $DELIVERY_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  -- get_escrow_contract
```

### Test Transaction
```bash
# Create a test delivery (Testnet only)
stellar contract invoke \
  --id $DELIVERY_CONTRACT_ID \
  --source test_sender \
  --network testnet \
  -- create_delivery \
  --sender $TEST_SENDER_ADDRESS \
  --recipient $TEST_RECIPIENT_ADDRESS \
  --metadata '{"delivery_id":1,"origin":"Test A","destination":"Test B","cargo_description":{"weight_grams":1000,"category":"General","fragile":false},"created_at":1000000,"estimated_delivery":2000000}'
```

## Monitoring

### Contract Events
```bash
# Monitor escrow events
stellar contract events \
  --id $ESCROW_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  --count 100

# Monitor delivery events
stellar contract events \
  --id $DELIVERY_CONTRACT_ID \
  --network $STELLAR_NETWORK \
  --count 100
```

### Metrics to Monitor
- Contract invocation count
- Gas usage per transaction
- Error rates
- Storage growth
- Failed transactions
- Event emission patterns

### Alerting
Set up alerts for:
- Contract errors
- Unexpected state transitions
- High gas usage
- Failed cross-contract calls
- Storage threshold breaches

## Rollback Procedures

### Contract Upgrade
Soroban contracts are upgradeable. To rollback:

```bash
# Deploy previous version
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/escrow_contract_v1.wasm \
  --source deployer \
  --network $STELLAR_NETWORK

# Update references in dependent contracts
stellar contract invoke \
  --id $DELIVERY_CONTRACT_ID \
  --source admin \
  --network $STELLAR_NETWORK \
  -- update_escrow_contract \
  --new_escrow_contract $OLD_ESCROW_CONTRACT_ID
```

### Emergency Procedures
1. **Pause Operations**: Call admin pause functions if available
2. **Notify Users**: Announce maintenance via official channels
3. **Deploy Fix**: Deploy corrected contract version
4. **Resume Operations**: Unpause and monitor closely

## Cost Estimation

### Testnet
- Deployment: Free (using Friendbot)
- Transactions: Free

### Mainnet
- Contract Deployment: ~0.1 XLM per contract
- Contract Initialization: ~0.01 XLM per contract
- Storage: 0.00001 XLM per ledger entry
- Transactions: Variable (0.00001-0.1 XLM)

**Budget Recommendation**: Have at least 10 XLM available for full deployment

## Support & Troubleshooting

### Common Issues

**Issue**: Insufficient balance
```bash
# Solution: Fund the account
stellar account balance deployer --network $STELLAR_NETWORK
```

**Issue**: Contract already exists
```bash
# Solution: Use install and then deploy with custom contract ID
stellar contract install --wasm <wasm-file> --network $STELLAR_NETWORK
```

**Issue**: Authorization failed
```bash
# Solution: Ensure deployer has auth
stellar keys show deployer
```

### Getting Help
- Stellar Discord: https://discord.gg/stellar
- Developer Docs: https://developers.stellar.org
- Issue Tracker: GitHub Issues

## Security Best Practices

1. **Never commit private keys** to version control
2. **Use hardware wallets** for mainnet admin keys
3. **Test thoroughly** on Testnet before Mainnet
4. **Implement time-locks** for sensitive operations
5. **Monitor contracts** continuously post-deployment
6. **Maintain backups** of all deployment configurations
7. **Document everything** for audit trail
8. **Use multi-sig** for admin operations where possible

---

**Last Updated**: January 2026
**Version**: 1.0.0

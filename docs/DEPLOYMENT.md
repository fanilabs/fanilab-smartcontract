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

### Testnet Reset Recovery and Manifest Invalidation

When Stellar resets the public Testnet (typically quarterly), all contract instances and ledger state are permanently wiped while local manifests (`contract-ids-testnet.json`) still point to non-existent addresses.

#### Recovery Procedure
1. **Invalidate Outdated Manifests**: Remove or archive stale manifests rather than attempting to interact with missing contracts:
   ```bash
   rm -f contract-ids-testnet.json
   ```
2. **Re-fund Deployer Account**: Testnet resets invalidate all keypair balances. Re-fund the deployer account:
   ```bash
   stellar keys fund deployer --network testnet
   ```
3. **Execute Clean Deployment**: Run the full deployment sequence from scratch:
   ```bash
   ./scripts/deploy-all-contracts.sh testnet
   ```
4. **Re-initialize Cross-Contract Bindings**: Ensure dependent addresses (e.g., escrow contract reference in delivery contract) are linked via `./scripts/initialize-contract.sh`.


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
names are `escrow_contract`, `delivery_contract`,
`dispute_resolution_contract`, `fleet_management_contract`,
`identity_reputation_contract`, and `settlement_contract`.

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
  --wasm target/wasm32v1-none/release/escrow_contract_v1.wasm \
  --source deployer \
  --network $STELLAR_NETWORK

# Important Note: The current delivery contract does not have an escrow contract setter function.
# To update escrow contract references, you must redeploy the delivery contract with the new escrow address.
# Alternatively, create a new delivery contract deployment with the rollback escrow address.
```

**Note on Contract Dependencies**: Currently, the delivery contract stores the escrow contract address during initialization and does not provide a setter function. Rollback procedures involving escrow contract updates require redeployment of dependent contracts.

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

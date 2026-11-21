#!/bin/bash

# FaniLab Smart Contracts - Complete Deployment Script
# Usage: ./deploy-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"

echo "🚀 FaniLab Smart Contract Deployment"
echo "=================================="
echo "Network: $NETWORK"
echo "Date: $(date)"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check prerequisites
echo "${BLUE}Checking prerequisites...${NC}"

if ! command -v stellar &> /dev/null; then
    echo "${RED}❌ Stellar CLI not found. Please install it first.${NC}"
    echo "   cargo install --locked stellar-cli"
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "${RED}❌ Cargo not found. Please install Rust.${NC}"
    exit 1
fi

if ! stellar keys address "$DEPLOYER_IDENTITY" &> /dev/null; then
    echo "${RED}❌ Stellar CLI identity '$DEPLOYER_IDENTITY' not found.${NC}"
    echo "   Every deploy/initialize script signs with it (--source $DEPLOYER_IDENTITY)."
    echo "   • CI: the 'Configure deployer identity' step in"
    echo "     .github/workflows/deploy-testnet.yml provisions it from the"
    echo "     CONTRACT_DEPLOYER_SECRET secret (environment secret"
    echo "     TESTNET_DEPLOYER_SECRET). Make sure that secret is set for the"
    echo "     'testnet' environment."
    echo "   • Local: create it once with"
    echo "       stellar keys generate $DEPLOYER_IDENTITY --network $NETWORK"
    echo "     or import an existing key with"
    echo "       stellar keys add $DEPLOYER_IDENTITY --secret-key"
    exit 1
fi

echo "${GREEN}✓ Prerequisites OK${NC}"
echo ""

# Build contracts
echo "${BLUE}Building contracts...${NC}"
cd "$PROJECT_ROOT"
cargo build --target wasm32v1-none --release

if [ $? -ne 0 ]; then
    echo "${RED}❌ Build failed${NC}"
    exit 1
fi

echo "${GREEN}✓ Build successful${NC}"
echo ""

# Capture the source revision once so every deployment entry references the
# exact commit the artifacts were built from (independently verifiable).
SOURCE_REVISION=$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")

# Compute the SHA-256 of a WASM artifact so the manifest can bind the deployed
# contract ID to the exact bytes that were deployed.
wasm_sha256() {
    local wasm_path=$1
    if command -v sha256sum &> /dev/null; then
        sha256sum "$wasm_path" | awk '{print $1}'
    elif command -v shasum &> /dev/null; then
        shasum -a 256 "$wasm_path" | awk '{print $1}'
    else
        echo ""
    fi
}

# Deploy function
deploy_contract() {
    local contract_name=$1
    local wasm_path="$PROJECT_ROOT/target/wasm32v1-none/release/${contract_name}.wasm"
    
    echo "${YELLOW}Deploying $contract_name...${NC}"
    
    if [ ! -f "$wasm_path" ]; then
        echo "${RED}❌ WASM file not found: $wasm_path${NC}"
        return 1
    fi
    
    local contract_id=$(stellar contract deploy \
        --wasm "$wasm_path" \
        --source "$DEPLOYER_IDENTITY" \
        --network "$NETWORK" 2>&1)
    
    if [ $? -eq 0 ]; then
        echo "${GREEN}✓ $contract_name deployed: $contract_id${NC}"
        echo "$contract_id"
        return 0
    else
        echo "${RED}❌ Failed to deploy $contract_name${NC}"
        echo "$contract_id"
        return 1
    fi
}

# Initialize JSON output
echo "{" > "$OUTPUT_FILE"
echo "  \"network\": \"$NETWORK\"," >> "$OUTPUT_FILE"
echo "  \"deployed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"," >> "$OUTPUT_FILE"
echo "  \"source_revision\": \"$SOURCE_REVISION\"," >> "$OUTPUT_FILE"
echo "  \"contracts\": {" >> "$OUTPUT_FILE"

# Deploy contracts in order
CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")
DEPLOYED_IDS=()

for i in "${!CONTRACTS[@]}"; do
    contract="${CONTRACTS[$i]}"
    wasm_path="$PROJECT_ROOT/target/wasm32v1-none/release/${contract}.wasm"
    contract_id=$(deploy_contract "$contract")
    
    if [ $? -ne 0 ]; then
        echo "${RED}❌ Deployment failed for $contract${NC}"
        exit 1
    fi
    
    DEPLOYED_IDS+=("$contract_id")
    
    # Record the artifact hash and source revision alongside the contract ID so
    # the deployed bytes can be independently verified against the manifest.
    wasm_hash=$(wasm_sha256 "$wasm_path")
    if [ -z "$wasm_hash" ]; then
        echo "${RED}❌ Could not compute SHA-256 for $wasm_path${NC}"
        exit 1
    fi
    
    # Add to JSON (with comma except for last)
    if [ $i -lt $((${#CONTRACTS[@]} - 1)) ]; then
        echo "    \"$contract\": {\"contract_id\": \"$contract_id\", \"wasm_sha256\": \"$wasm_hash\", \"source_revision\": \"$SOURCE_REVISION\"}," >> "$OUTPUT_FILE"
    else
        echo "    \"$contract\": {\"contract_id\": \"$contract_id\", \"wasm_sha256\": \"$wasm_hash\", \"source_revision\": \"$SOURCE_REVISION\"}" >> "$OUTPUT_FILE"
    fi
    
    echo ""
done

# Close JSON
echo "  }" >> "$OUTPUT_FILE"
echo "}" >> "$OUTPUT_FILE"

echo ""
echo "${GREEN}=================================="
echo "✅ All contracts deployed successfully!"
echo "==================================${NC}"
echo ""
echo "Contract IDs saved to: $OUTPUT_FILE"
echo ""
echo "${BLUE}Next steps:${NC}"
echo "1. Initialize contracts with ./scripts/initialize-all-contracts.sh $NETWORK"
echo "2. Verify deployments"
echo "3. Configure frontend/backend with new contract IDs"
echo ""

# Display summary
echo "${BLUE}Deployment Summary:${NC}"
for i in "${!CONTRACTS[@]}"; do
    echo "  ${CONTRACTS[$i]}: ${DEPLOYED_IDS[$i]}"
done

exit 0

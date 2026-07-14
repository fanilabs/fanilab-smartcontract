#!/bin/bash

# FaniLab Smart Contracts - Complete Deployment Script
# Usage: ./deploy-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
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

echo "${GREEN}✓ Prerequisites OK${NC}"
echo ""

# Build contracts
echo "${BLUE}Building contracts...${NC}"
cd "$PROJECT_ROOT"
cargo build --target wasm32-unknown-unknown --release

if [ $? -ne 0 ]; then
    echo "${RED}❌ Build failed${NC}"
    exit 1
fi

echo "${GREEN}✓ Build successful${NC}"
echo ""

# Deploy function
deploy_contract() {
    local contract_name=$1
    local wasm_path="$PROJECT_ROOT/target/wasm32-unknown-unknown/release/${contract_name}.wasm"
    
    echo "${YELLOW}Deploying $contract_name...${NC}"
    
    if [ ! -f "$wasm_path" ]; then
        echo "${RED}❌ WASM file not found: $wasm_path${NC}"
        return 1
    fi
    
    local contract_id=$(stellar contract deploy \
        --wasm "$wasm_path" \
        --source deployer \
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
echo "  \"contracts\": {" >> "$OUTPUT_FILE"

# Deploy contracts in order
CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")
DEPLOYED_IDS=()

for i in "${!CONTRACTS[@]}"; do
    contract="${CONTRACTS[$i]}"
    contract_id=$(deploy_contract "$contract")
    
    if [ $? -ne 0 ]; then
        echo "${RED}❌ Deployment failed for $contract${NC}"
        exit 1
    fi
    
    DEPLOYED_IDS+=("$contract_id")
    
    # Add to JSON (with comma except for last)
    if [ $i -lt $((${#CONTRACTS[@]} - 1)) ]; then
        echo "    \"$contract\": \"$contract_id\"," >> "$OUTPUT_FILE"
    else
        echo "    \"$contract\": \"$contract_id\"" >> "$OUTPUT_FILE"
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

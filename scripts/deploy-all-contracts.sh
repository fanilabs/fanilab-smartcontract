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

# Contracts that must be present in a deployment manifest.
CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")

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

if ! command -v jq &> /dev/null; then
    echo "${RED}❌ jq not found. It is required to parse deployment manifests.${NC}"
    echo "   Install it from https://jqlang.github.io/jq/"
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

# Deploy function
deploy_contract() {
    local contract_name=$1
    local wasm_path="$PROJECT_ROOT/target/wasm32v1-none/release/${contract_name}.wasm"
    
    echo "${YELLOW}Deploying $contract_name...${NC}"
    
    if [ ! -f "$wasm_path" ]; then
        echo "${RED}❌ WASM file not found: $wasm_path${NC}" >&2
        return 1
    fi
    
    local contract_id=$(stellar contract deploy \
        --wasm "$wasm_path" \
        --source "$DEPLOYER_IDENTITY" \
        --network "$NETWORK" 2>&1)
    
    if [ $? -eq 0 ]; then
        echo "${GREEN}✓ $contract_name deployed: $contract_id${NC}" >&2
        echo "$contract_id"
        return 0
    else
        echo "${RED}❌ Failed to deploy $contract_name${NC}" >&2
        echo "$contract_id" >&2
        return 1
    fi
}

# Validate a deployment manifest with a real JSON parser.
# Rejects malformed JSON, duplicate keys, and missing/empty contract IDs.
validate_manifest() {
    local manifest=$1

    if [ ! -f "$manifest" ]; then
        echo "${RED}❌ Manifest not found: $manifest${NC}" >&2
        return 1
    fi

    # Parse with jq; malformed JSON makes jq exit non-zero.
    if ! jq empty "$manifest" > /dev/null 2>&1; then
        echo "${RED}❌ Malformed JSON in manifest: $manifest${NC}" >&2
        return 1
    fi

    # Detect duplicate keys anywhere in the document. jq's --stream emits
    # every key path; a duplicate key produces the same path twice.
    local dup_keys
    dup_keys=$(jq -r --stream 'select(length == 2) | .[0] | @json' "$manifest" 2>/dev/null \
        | sort | uniq -d)
    if [ -n "$dup_keys" ]; then
        echo "${RED}❌ Duplicate keys in manifest: $manifest${NC}" >&2
        echo "$dup_keys" >&2
        return 1
    fi

    # Schema checks: top-level object with a contracts object.
    if ! jq -e 'type == "object" and (.contracts | type == "object")' "$manifest" > /dev/null 2>&1; then
        echo "${RED}❌ Manifest schema invalid: expected top-level 'contracts' object${NC}" >&2
        return 1
    fi

    # Every expected contract must be present with a non-empty string ID.
    local missing=()
    local contract
    for contract in "${CONTRACTS[@]}"; do
        if ! jq -e --arg c "$contract" \
            '.contracts[$c] | type == "string" and length > 0' \
            "$manifest" > /dev/null 2>&1; then
            missing+=("$contract")
        fi
    done

    if [ ${#missing[@]} -gt 0 ]; then
        echo "${RED}❌ Manifest missing contract IDs: ${missing[*]}${NC}" >&2
        return 1
    fi

    return 0
}

# Initialize JSON output
echo "{" > "$OUTPUT_FILE"
echo "  \"network\": \"$NETWORK\"," >> "$OUTPUT_FILE"
echo "  \"deployed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"," >> "$OUTPUT_FILE"
echo "  \"contracts\": {" >> "$OUTPUT_FILE"

# Deploy contracts in order
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

# Validate the manifest we just wrote before declaring success.
if ! validate_manifest "$OUTPUT_FILE"; then
    echo "${RED}❌ Deployment manifest failed validation: $OUTPUT_FILE${NC}"
    exit 1
fi

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

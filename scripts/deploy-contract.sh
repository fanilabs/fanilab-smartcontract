#!/bin/bash

# FaniLab Smart Contracts - Single Contract Deployment Script
# Usage: ./deploy-contract.sh <contract_name> [network]

set -e

CONTRACT_NAME=$1
NETWORK=${2:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"

CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

if [ -z "$CONTRACT_NAME" ]; then
    echo "${RED}❌ Usage: $0 <contract_name> [network]${NC}" >&2
    echo "   Valid contract names: ${CONTRACTS[*]}" >&2
    exit 1
fi

CONTRACT_KNOWN=false
for c in "${CONTRACTS[@]}"; do
    if [ "$c" = "$CONTRACT_NAME" ]; then
        CONTRACT_KNOWN=true
        break
    fi
done

if [ "$CONTRACT_KNOWN" = false ]; then
    echo "${RED}❌ Unknown contract: $CONTRACT_NAME${NC}" >&2
    echo "   Valid contract names: ${CONTRACTS[*]}" >&2
    exit 1
fi

echo "🚀 FaniLab Smart Contract Deployment" >&2
echo "==================================" >&2
echo "Contract: $CONTRACT_NAME" >&2
echo "Network: $NETWORK" >&2
echo "Date: $(date)" >&2
echo "" >&2

# Check prerequisites
echo "${BLUE}Checking prerequisites...${NC}" >&2

if ! command -v stellar &> /dev/null; then
    echo "${RED}❌ Stellar CLI not found. Please install it first.${NC}" >&2
    echo "   cargo install --locked stellar-cli" >&2
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    echo "${RED}❌ Cargo not found. Please install Rust.${NC}" >&2
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "${RED}❌ jq not found. It is required to parse deployment manifests.${NC}" >&2
    echo "   Install it from https://jqlang.github.io/jq/" >&2
    exit 1
fi

if ! stellar keys address "$DEPLOYER_IDENTITY" &> /dev/null; then
    echo "${RED}❌ Stellar CLI identity '$DEPLOYER_IDENTITY' not found.${NC}" >&2
    echo "   Every deploy/initialize script signs with it (--source $DEPLOYER_IDENTITY)." >&2
    echo "   • CI: the 'Configure deployer identity' step in" >&2
    echo "     .github/workflows/deploy-testnet.yml provisions it from the" >&2
    echo "     CONTRACT_DEPLOYER_SECRET secret (environment secret" >&2
    echo "     TESTNET_DEPLOYER_SECRET). Make sure that secret is set for the" >&2
    echo "     'testnet' environment." >&2
    echo "   • Local: create it once with" >&2
    echo "       stellar keys generate $DEPLOYER_IDENTITY --network $NETWORK" >&2
    echo "     or import an existing key with" >&2
    echo "       stellar keys add $DEPLOYER_IDENTITY --secret-key" >&2
    exit 1
fi

echo "${GREEN}✓ Prerequisites OK${NC}" >&2
echo "" >&2

# Build the contract
echo "${BLUE}Building $CONTRACT_NAME...${NC}" >&2
cd "$PROJECT_ROOT"
cargo build --target wasm32v1-none --release -p "$CONTRACT_NAME"
echo "${GREEN}✓ Build successful${NC}" >&2
echo "" >&2

WASM_PATH="$PROJECT_ROOT/target/wasm32v1-none/release/${CONTRACT_NAME}.wasm"

if [ ! -f "$WASM_PATH" ]; then
    echo "${RED}❌ WASM file not found: $WASM_PATH${NC}" >&2
    exit 1
fi

# Deploy the contract
echo "${YELLOW}Deploying $CONTRACT_NAME...${NC}" >&2

# NOTE: only the raw stellar CLI output goes to stdout here; all status/log
# lines above and below are sent to stderr so a caller doing
# CONTRACT_ID=$(./deploy-contract.sh ...) captures a clean contract ID
# rather than a blob of interleaved log text.
CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source "$DEPLOYER_IDENTITY" \
    --network "$NETWORK")

echo "${GREEN}✓ $CONTRACT_NAME deployed: $CONTRACT_ID${NC}" >&2
echo "" >&2

# Merge this contract's ID into any existing output file for this network,
# so deploying contracts one at a time doesn't clobber previously deployed IDs.
# The manifest is parsed with jq (a real JSON parser) rather than grep so that
# malformed JSON, duplicate keys, and unexpected shapes are rejected instead of
# silently producing ambiguous IDs.
declare -A DEPLOYED_IDS
if [ -f "$OUTPUT_FILE" ]; then
    # Reject malformed JSON up front with a clear error.
    if ! jq -e . "$OUTPUT_FILE" > /dev/null 2>&1; then
        echo "${RED}❌ Malformed JSON in deployment manifest: $OUTPUT_FILE${NC}" >&2
        echo "   Fix or remove the file before deploying again." >&2
        exit 1
    fi

    # Reject duplicate keys anywhere in the manifest. jq keeps only the last
    # value for a duplicated key, so detect duplicates by comparing the raw
    # key occurrences against the number of keys jq actually parsed.
    duplicate_keys=$(jq -r '
        [paths(scalars) as $p | $p[-1]] | group_by(.) | map(select(length > 1) | .[0]) | .[]
    ' "$OUTPUT_FILE" 2>/dev/null || true)
    if [ -n "$duplicate_keys" ]; then
        echo "${RED}❌ Duplicate keys in deployment manifest: $OUTPUT_FILE${NC}" >&2
        echo "   Duplicated key(s): $duplicate_keys" >&2
        exit 1
    fi

    # Schema check: the manifest must be an object with a "contracts" object.
    if ! jq -e 'type == "object" and (.contracts | type == "object")' "$OUTPUT_FILE" > /dev/null 2>&1; then
        echo "${RED}❌ Invalid deployment manifest schema: $OUTPUT_FILE${NC}" >&2
        echo "   Expected an object with a \"contracts\" object." >&2
        exit 1
    fi

    # Schema check: every contract ID present must be a non-empty string.
    if ! jq -e '[.contracts[] | type == "string" and length > 0] | all' "$OUTPUT_FILE" > /dev/null 2>&1; then
        echo "${RED}❌ Invalid contract ID in deployment manifest: $OUTPUT_FILE${NC}" >&2
        echo "   Every contract ID must be a non-empty string." >&2
        exit 1
    fi

    for c in "${CONTRACTS[@]}"; do
        existing_id=$(jq -r --arg c "$c" '.contracts[$c] // empty' "$OUTPUT_FILE")
        if [ -n "$existing_id" ]; then
            DEPLOYED_IDS["$c"]="$existing_id"
        fi
    done
fi
DEPLOYED_IDS["$CONTRACT_NAME"]="$CONTRACT_ID"

{
    echo "{"
    echo "  \"network\": \"$NETWORK\","
    echo "  \"deployed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    echo "  \"contracts\": {"
    first=true
    for c in "${CONTRACTS[@]}"; do
        if [ -n "${DEPLOYED_IDS[$c]:-}" ]; then
            if [ "$first" = true ]; then
                first=false
            else
                echo ","
            fi
            printf '    "%s": "%s"' "$c" "${DEPLOYED_IDS[$c]}"
        fi
    done
    echo ""
    echo "  }"
    echo "}"
} > "$OUTPUT_FILE"

echo "${GREEN}✓ Contract ID saved to: $OUTPUT_FILE${NC}" >&2

echo "$CONTRACT_ID"

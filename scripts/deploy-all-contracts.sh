#!/bin/bash

# FaniLab Smart Contracts - Complete Deployment Script
# Usage: ./deploy-all-contracts.sh [testnet|mainnet]
#
# Configuration file precedence (highest to lowest):
#   1. Explicit path via --config <path> flag
#   2. Explicit path via CONFIG_FILE environment variable
#   3. Default repository .env at the project root

set -e

# Parse optional --config flag before positional args.
CONFIG_FILE="${CONFIG_FILE:-}"
POSITIONAL=()
while [ $# -gt 0 ]; do
    case "$1" in
        --config)
            if [ -z "${2:-}" ]; then
                echo "❌ --config requires a path argument" >&2
                exit 1
            fi
            CONFIG_FILE="$2"
            shift 2
            ;;
        --config=*)
            CONFIG_FILE="${1#--config=}"
            shift
            ;;
        *)
            POSITIONAL+=("$1")
            shift
            ;;
    esac
done

NETWORK=${POSITIONAL[0]:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"
# Incremental recovery journal: one line per successful deployment, appended
# as soon as each contract is deployed. Survives a mid-batch failure so a
# retry can reuse already-deployed contract IDs instead of redeploying.
JOURNAL_FILE="$PROJECT_ROOT/.deploy-journal-$NETWORK.log"

# Resolve config file: explicit path (flag/env) wins, else default repo .env.
if [ -n "$CONFIG_FILE" ]; then
    if [ ! -f "$CONFIG_FILE" ]; then
        echo "❌ Config file not found: $CONFIG_FILE" >&2
        exit 1
    fi
    echo "Using config file: $CONFIG_FILE"
else
    CONFIG_FILE="$PROJECT_ROOT/.env"
    if [ -f "$CONFIG_FILE" ]; then
        echo "Using default config file: $CONFIG_FILE"
    fi
fi

# Load config if present (explicit or default).
if [ -f "$CONFIG_FILE" ]; then
    set -a
    # shellcheck disable=SC1090
    . "$CONFIG_FILE"
    set +a
fi

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

# Look up a previously journaled contract ID (recovery after mid-batch failure).
journal_lookup() {
    local contract_name=$1
    [ -f "$JOURNAL_FILE" ] || return 1
    # Journal format: <contract_name> <contract_id>
    awk -v name="$contract_name" '$1 == name { id = $2 } END { if (id != "") { print id; exit 0 } exit 1 }' "$JOURNAL_FILE"
}

# Append a successful deployment to the journal immediately.
journal_record() {
    local contract_name=$1
    local contract_id=$2
    printf '%s %s\n' "$contract_name" "$contract_id" >> "$JOURNAL_FILE"
}

# Deploy contracts in order
CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")
DEPLOYED_IDS=()

for i in "${!CONTRACTS[@]}"; do
    contract="${CONTRACTS[$i]}"

    # Reuse an ID from a previous (failed) run if the journal already has it.
    if existing_id=$(journal_lookup "$contract"); then
        echo "${YELLOW}↺ Reusing journaled $contract: $existing_id${NC}"
        contract_id="$existing_id"
    else
        contract_id=$(deploy_contract "$contract")

        if [ $? -ne 0 ]; then
            echo "${RED}❌ Deployment failed for $contract${NC}"
            echo "   Partial progress saved to: $JOURNAL_FILE"
            echo "   Re-run this script to resume from the journal."
            exit 1
        fi

        # Record each successful deployment separately, as it happens.
        journal_record "$contract" "$contract_id"
    fi

    DEPLOYED_IDS+=("$contract_id")
    echo ""
done

# All deployments succeeded: finalize the manifest atomically (temp + rename)
# so readers never observe a partially written JSON file.
TMP_OUTPUT="$(mktemp "${OUTPUT_FILE}.XXXXXX")"
trap 'rm -f "$TMP_OUTPUT"' EXIT
{
    echo "{"
    echo "  \"network\": \"$NETWORK\","
    echo "  \"deployed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    echo "  \"contracts\": {"
    for i in "${!CONTRACTS[@]}"; do
        if [ $i -lt $((${#CONTRACTS[@]} - 1)) ]; then
            echo "    \"${CONTRACTS[$i]}\": \"${DEPLOYED_IDS[$i]}\","
        else
            echo "    \"${CONTRACTS[$i]}\": \"${DEPLOYED_IDS[$i]}\""
        fi
    done
    echo "  }"
    echo "}"
} > "$TMP_OUTPUT"
mv -f "$TMP_OUTPUT" "$OUTPUT_FILE"
trap - EXIT

# Batch complete: the journal is no longer needed for recovery.
rm -f "$JOURNAL_FILE"

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

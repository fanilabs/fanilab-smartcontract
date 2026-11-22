#!/bin/bash

# FaniLab Smart Contracts - Complete Deployment Script
# Usage: ./deploy-all-contracts.sh [testnet|mainnet] [--confirm-mainnet]
#
# Configuration file precedence (highest to lowest):
#   1. Explicit path via --config <path> flag
#   2. Explicit path via CONFIG_FILE environment variable
#   3. Default repository .env at the project root
#
# Mainnet safety: deploying to mainnet requires the explicit --confirm-mainnet
# flag. Without it the script aborts before any signed deployment begins.

set -e

# Parse optional --config and --confirm-mainnet flags before positional args.
CONFIG_FILE="${CONFIG_FILE:-}"
CONFIRM_MAINNET=0
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
        --confirm-mainnet)
            CONFIRM_MAINNET=1
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

# Minimum deployer balance (in stroops) required before the batch starts.
# Each contract deploy consumes fees + a minimum account reserve; this is a
# conservative floor so the batch does not fail partway through. Overridable
# via MIN_DEPLOYER_BALANCE_STROOPS (default 10 XLM = 100,000,000 stroops).
MIN_DEPLOYER_BALANCE_STROOPS="${MIN_DEPLOYER_BALANCE_STROOPS:-100000000}"

# Mainnet gate: refuse to begin signed deployments without explicit consent.
if [ "$NETWORK" = "mainnet" ] && [ "$CONFIRM_MAINNET" -ne 1 ]; then
    echo "❌ Refusing to deploy to mainnet without explicit acknowledgement." >&2
    echo "   Re-run with the --confirm-mainnet flag to proceed:" >&2
    echo "     ./scripts/deploy-all-contracts.sh mainnet --confirm-mainnet" >&2
    exit 1
fi

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

# Preflight: verify the deployer account is funded before deploying anything.
# Stops the batch before the first deployment if the required reserve is
# unavailable, so a partially-deployed batch is never left behind.
echo "${BLUE}Checking deployer funding...${NC}"
DEPLOYER_ADDRESS=$(stellar keys address "$DEPLOYER_IDENTITY" 2>/dev/null)
if [ -z "$DEPLOYER_ADDRESS" ]; then
    echo "${RED}❌ Could not resolve address for identity '$DEPLOYER_IDENTITY'.${NC}"
    exit 1
fi

# Query the account balance in stroops. `stellar account balance` prints the
# native balance; fall back to an empty value if the account is unfunded.
DEPLOYER_BALANCE_STROOPS=$(stellar account balance \
    --account "$DEPLOYER_ADDRESS" \
    --network "$NETWORK" 2>/dev/null || true)

if [ -z "$DEPLOYER_BALANCE_STROOPS" ]; then
    echo "${RED}❌ Deployer account '$DEPLOYER_ADDRESS' is not funded on $NETWORK.${NC}"
    echo "   Fund it before deploying (e.g. via friendbot on testnet):"
    echo "     curl \"https://friendbot.stellar.org?addr=$DEPLOYER_ADDRESS\""
    exit 1
fi

if [ "$DEPLOYER_BALANCE_STROOPS" -lt "$MIN_DEPLOYER_BALANCE_STROOPS" ] 2>/dev/null; then
    echo "${RED}❌ Insufficient deployer balance on $NETWORK.${NC}"
    echo "   Account:  $DEPLOYER_ADDRESS"
    echo "   Balance:  $DEPLOYER_BALANCE_STROOPS stroops"
    echo "   Required: $MIN_DEPLOYER_BALANCE_STROOPS stroops (MIN_DEPLOYER_BALANCE_STROOPS)"
    echo "   Fund the account and re-run this script."
    exit 1
fi

echo "${GREEN}✓ Deployer funded: $DEPLOYER_BALANCE_STROOPS stroops (min $MIN_DEPLOYER_BALANCE_STROOPS)${NC}"
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

# Show the source account and artifact hashes before any submission so the
# operator can verify exactly what will be signed and deployed.
echo "${BLUE}Deployment summary (pre-submission)...${NC}"
echo "   Network:        $NETWORK"
echo "   Source account: $DEPLOYER_ADDRESS (identity: $DEPLOYER_IDENTITY)"
echo "   Artifact hashes (sha256):"
for contract in escrow_contract delivery_contract dispute_resolution_contract fleet_management_contract identity_reputation_contract settlement_contract; do
    wasm_path="$PROJECT_ROOT/target/wasm32v1-none/release/${contract}.wasm"
    if [ -f "$wasm_path" ]; then
        if command -v sha256sum &> /dev/null; then
            wasm_hash=$(sha256sum "$wasm_path" | awk '{print $1}')
        else
            wasm_hash=$(shasum -a 256 "$wasm_path" | awk '{print $1}')
        fi
        echo "     $contract: $wasm_hash"
    else
        echo "     $contract: (missing)"
    fi
done
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
            echo "   Re-run this script to resume from the journal"
            exit 1
        fi

        journal_record "$contract" "$contract_id"
    fi

    DEPLOYED_IDS+=("$contract_id")
done

# Validate the manifest we just wrote before declaring success.
if ! validate_manifest "$OUTPUT_FILE"; then
    echo "${RED}❌ Deployment manifest failed validation: $OUTPUT_FILE${NC}"
    exit 1
fi

# Record the expected WASM hash for each deployed contract so that
# initialize-all-contracts.sh can verify on-chain identity before init.
echo ""
echo "${BLUE}Recording expected WASM hashes...${NC}"
HASH_FILE="$PROJECT_ROOT/contract-hashes-$NETWORK.json"
echo "{" > "$HASH_FILE"
echo "  \"network\": \"$NETWORK\"," >> "$HASH_FILE"
echo "  \"contracts\": {" >> "$HASH_FILE"
for i in "${!CONTRACTS[@]}"; do
    contract="${CONTRACTS[$i]}"
    expected_hash=$(expected_wasm_hash "$contract")
    if [ $? -ne 0 ] || [ -z "$expected_hash" ]; then
        echo "${RED}❌ Could not compute expected WASM hash for $contract${NC}"
        exit 1
    fi
    if [ $i -lt $((${#CONTRACTS[@]} - 1)) ]; then
        echo "    \"$contract\": \"$expected_hash\"," >> "$HASH_FILE"
    else
        echo "    \"$contract\": \"$expected_hash\"" >> "$HASH_FILE"
    fi
done
echo "  }" >> "$HASH_FILE"
echo "}" >> "$HASH_FILE"
echo "${GREEN}✓ Expected hashes saved to: $HASH_FILE${NC}"

echo ""
echo "${GREEN}✓ All contracts deployed${NC}"
echo ""

# Write contract IDs to output file
{
    echo "{"
    echo "  \"network\": \"$NETWORK\","
    echo "  \"deployed_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"contracts\": {"
    for i in "${!CONTRACTS[@]}"; do
        comma=","
        if [ $i -eq $((${#CONTRACTS[@]} - 1)) ]; then
            comma=""
        fi
        echo "    \"${CONTRACTS[$i]}\": \"${DEPLOYED_IDS[$i]}\"$comma"
    done
    echo "  }"
    echo "}"
} > "$OUTPUT_FILE"

echo "${GREEN}✓ Contract IDs saved to: $OUTPUT_FILE${NC}"
echo ""
echo "${BLUE}Next steps:${NC}"
echo "1. Initialize contracts: ./scripts/initialize-contract.sh $NETWORK"
echo "2. Verify deployment: ./scripts/verify-deployment.sh $NETWORK"
echo ""

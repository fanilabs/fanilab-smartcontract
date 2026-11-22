#!/bin/bash

# FaniLab Smart Contracts - Single Contract Deployment Script
# Usage: ./deploy-contract.sh <contract_name> [network] [--config <path>] [--confirm-mainnet]
#
# Configuration file precedence (highest to lowest):
#   1. --config <path> CLI flag
#   2. CONFIG_FILE environment variable
#   3. Default repository .env at the project root
#
# Mainnet safety: deploying to mainnet requires the explicit --confirm-mainnet
# flag. Without it the script aborts before any signed transaction is built.

set -e

CONFIG_FILE=""
CONFIRM_MAINNET=false
ARGS=()
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
            CONFIRM_MAINNET=true
            shift
            ;;
        *)
            ARGS+=("$1")
            shift
            ;;
    esac
done

CONTRACT_NAME=${ARGS[0]:-}
NETWORK=${ARGS[1]:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"
# Incremental recovery journal: each successful deployment is appended here
# immediately so a mid-batch failure can be recovered on retry. The final
# manifest ($OUTPUT_FILE) is only written atomically once all contracts exist.
JOURNAL_FILE="$PROJECT_ROOT/.contract-ids-$NETWORK.journal"

# Resolve the configuration file: explicit flag > env var > default .env.
if [ -z "$CONFIG_FILE" ]; then
    CONFIG_FILE="${CONFIG_FILE_ENV:-}"
fi
if [ -z "$CONFIG_FILE" ]; then
    CONFIG_FILE="$PROJECT_ROOT/.env"
fi

if [ -f "$CONFIG_FILE" ]; then
    # shellcheck disable=SC1090
    set -a
    . "$CONFIG_FILE"
    set +a
fi

CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")

# Minimum native (XLM) balance, in stroops, the deployer must hold before a
# deployment is attempted. Each contract deploy consumes fees + a storage
# reserve; this reserve is a conservative floor so a batch never starts on an
# underfunded account. Overridable via env for networks with different costs.
MIN_DEPLOYER_RESERVE_STROOPS="${MIN_DEPLOYER_RESERVE_STROOPS:-100000000}" # 10 XLM

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

if [ -z "$CONTRACT_NAME" ]; then
    echo "${RED}❌ Usage: $0 <contract_name> [network] [--config <path>] [--confirm-mainnet]${NC}" >&2
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

# Mainnet acknowledgement gate: refuse to proceed without an explicit flag so
# automation can never begin signed mainnet deployments by accident.
if [ "$NETWORK" = "mainnet" ] && [ "$CONFIRM_MAINNET" != true ]; then
    echo "${RED}❌ Refusing to deploy to mainnet without explicit acknowledgement.${NC}" >&2
    echo "   Re-run with --confirm-mainnet to proceed with signed mainnet deployments." >&2
    exit 1
fi

echo "🚀 FaniLab Smart Contract Deployment" >&2
echo "==================================" >&2
echo "Contract: $CONTRACT_NAME" >&2
echo "Network: $NETWORK" >&2
echo "Config: $CONFIG_FILE" >&2
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

# Preflight funding: verify the deployer holds at least the required reserve
# before any deployment is attempted, so a batch never starts on an
# underfunded account and fails partway through.
DEPLOYER_ADDRESS=$(stellar keys address "$DEPLOYER_IDENTITY")
if ! DEPLOYER_BALANCE=$(stellar account balance "$DEPLOYER_ADDRESS" --network "$NETWORK" 2>/dev/null); then
    echo "${RED}❌ Unable to read deployer balance for '$DEPLOYER_IDENTITY' ($DEPLOYER_ADDRESS) on $NETWORK.${NC}" >&2
    echo "   The account may be unfunded. Fund it before deploying:" >&2
    echo "       stellar keys fund $DEPLOYER_IDENTITY --network $NETWORK" >&2
    exit 1
fi

# Parse the native XLM balance (in stroops) from the CLI output. The CLI
# reports balances as "<amount> XLM" lines; take the first XLM amount.
DEPLOYER_BALANCE_STROOPS=$(printf '%s\n' "$DEPLOYER_BALANCE" \
    | grep -Eo '[0-9]+(\.[0-9]+)? XLM' \
    | head -n1 \
    | awk '{ printf "%d", $1 * 10000000 }')

if [ -z "$DEPLOYER_BALANCE_STROOPS" ]; then
    echo "${RED}❌ Could not determine deployer XLM balance for '$DEPLOYER_IDENTITY'.${NC}" >&2
    echo "   Raw balance output:" >&2
    printf '%s\n' "$DEPLOYER_BALANCE" >&2
    exit 1
fi

if [ "$DEPLOYER_BALANCE_STROOPS" -lt "$MIN_DEPLOYER_RESERVE_STROOPS" ]; then
    echo "${RED}❌ Insufficient deployer funds on $NETWORK.${NC}" >&2
    echo "   Required reserve: $MIN_DEPLOYER_RESERVE_STROOPS stroops" >&2
    echo "   Available balance: $DEPLOYER_BALANCE_STROOPS stroops" >&2
    echo "   Fund '$DEPLOYER_IDENTITY' before deploying:" >&2
    echo "       stellar keys fund $DEPLOYER_IDENTITY --network $NETWORK" >&2
    exit 1
fi

echo "${GREEN}✓ Prerequisites OK (deployer balance: $DEPLOYER_BALANCE_STROOPS stroops)${NC}" >&2
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

# Show the source account and artifact hash before any signed submission so
# the operator can verify exactly what is about to be deployed.
WASM_SHA256=$(sha256sum "$WASM_PATH" | awk '{ print $1 }')
echo "${BLUE}Deployment summary (pre-submission):${NC}" >&2
echo "   Source account: $DEPLOYER_IDENTITY ($DEPLOYER_ADDRESS)" >&2
echo "   Network:        $NETWORK" >&2
echo "   Artifact:       $WASM_PATH" >&2
echo "   SHA-256:        $WASM_SHA256" >&2
echo "" >&2

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

# Record this successful deployment in the incremental recovery journal
# immediately, so a mid-batch failure never loses an already-deployed ID.
# Format: one "<contract_name> <contract_id>" line per successful deploy.
if [ -f "$JOURNAL_FILE" ]; then
    grep -v "^$CONTRACT_NAME " "$JOURNAL_FILE" > "$JOURNAL_FILE.tmp" || true
    mv "$JOURNAL_FILE.tmp" "$JOURNAL_FILE"
fi
printf '%s %s\n' "$CONTRACT_NAME" "$CONTRACT_ID" >> "$JOURNAL_FILE"

# Merge this contract's ID into any existing output file for this network,
# so deploying contracts one at a time doesn't clobber previously deployed IDs.
# IDs are recovered from the journal first (survives mid-batch failures), then
# from the existing manifest, then the just-deployed contract.
declare -A DEPLOYED_IDS
if [ -f "$JOURNAL_FILE" ]; then
    while read -r name id; do
        [ -n "$name" ] && [ -n "$id" ] && DEPLOYED_IDS["$name"]="$id"
    done < "$JOURNAL_FILE"
fi
if [ -f "$OUTPUT_FILE" ]; then
    while IFS= read -r line; do
        name=$(printf '%s' "$line" | sed -n 's/.*"\([a-z_]*\)"[[:space:]]*:[[:space:]]*"\([A-Z0-9]*\)".*/\1/p')
        id=$(printf '%s' "$line" | sed -n 's/.*"\([a-z_]*\)"[[:space:]]*:[[:space:]]*"\([A-Z0-9]*\)".*/\2/p')
        if [ -n "$name" ] && [ -n "$id" ]; then
            DEPLOYED_IDS["$name"]="$id"
        fi
    done < "$OUTPUT_FILE"
fi
DEPLOYED_IDS["$CONTRACT_NAME"]="$CONTRACT_ID"

# Write the manifest atomically: render into a temporary sibling file, validate
# it, then rename over the target. A crash or interruption mid-write leaves the
# previous manifest intact instead of a truncated one.
TMP_FILE="$(mktemp "${OUTPUT_FILE}.XXXXXX.tmp")"
cleanup_tmp() {
    rm -f "$TMP_FILE"
}
trap cleanup_tmp EXIT

{
    echo "{"
    echo "  \"network\": \"$NETWORK\","
    echo "  \"deployed_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
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
} > "$TMP_FILE"

# Validate the temporary file before promoting it. Prefer a real JSON parser
# when available, otherwise fall back to a basic structural sanity check.
if command -v python3 &> /dev/null; then
    if ! python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$TMP_FILE"; then
        echo "${RED}❌ Generated manifest failed JSON validation; keeping existing $OUTPUT_FILE${NC}" >&2
        exit 1
    fi
elif command -v jq &> /dev/null; then
    if ! jq empty "$TMP_FILE" &> /dev/null; then
        echo "${RED}❌ Generated manifest failed JSON validation; keeping existing $OUTPUT_FILE${NC}" >&2
        exit 1
    fi
else
    if [ ! -s "$TMP_FILE" ] || [ "$(head -c 1 "$TMP_FILE")" != "{" ] || [ "$(tail -c 1 "$TMP_FILE")" != "}" ]; then
        echo "${RED}❌ Generated manifest failed sanity check; keeping existing $OUTPUT_FILE${NC}" >&2
        exit 1
    fi
fi

# Atomic promotion: rename is atomic on the same filesystem, so readers see
# either the old manifest or the fully written new one, never a partial file.
mv -f "$TMP_FILE" "$OUTPUT_FILE"
trap - EXIT

echo "${GREEN}✓ Contract ID saved to: $OUTPUT_FILE${NC}" >&2

echo "${GREEN}✓ Contract ID recorded in $OUTPUT_FILE${NC}" >&2
echo "" >&2
echo "${GREEN}🎉 Deployment complete!${NC}" >&2
echo "Contract ID: $CONTRACT_ID" >&2

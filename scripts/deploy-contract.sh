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

# Networks this script knows how to deploy to. The network argument is
# interpolated into the manifest path (contract-ids-$NETWORK.json) and passed
# to the Stellar CLI, so it must be validated against this allowlist before any
# path is constructed or any file is written.
SUPPORTED_NETWORKS=("testnet" "mainnet" "futurenet" "local")

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

# Reject path separators / traversal sequences outright. This runs before any
# manifest path is built so invalid input never touches the filesystem.
case "$NETWORK" in
    */*|*\\*|*..*)
        echo "${RED}❌ Invalid network name: '$NETWORK'${NC}" >&2
        echo "   Network names must not contain path separators or '..'." >&2
        echo "   Supported networks: ${SUPPORTED_NETWORKS[*]}" >&2
        exit 1
        ;;
esac

NETWORK_KNOWN=false
for n in "${SUPPORTED_NETWORKS[@]}"; do
    if [ "$n" = "$NETWORK" ]; then
        NETWORK_KNOWN=true
        break
    fi
done

if [ "$NETWORK_KNOWN" = false ]; then
    echo "${RED}❌ Unsupported network: '$NETWORK'${NC}" >&2
    echo "   Supported networks: ${SUPPORTED_NETWORKS[*]}" >&2
    exit 1
fi

OUTPUT_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"
# Shared lock guarding the manifest read-modify-write below. Concurrent
# single-contract deploys serialize on this file so neither loses the other's
# added contract ID.
LOCK_FILE="$OUTPUT_FILE.lock"

CONTRACTS=("escrow_contract" "delivery_contract" "dispute_resolution_contract" "fleet_management_contract" "identity_reputation_contract" "settlement_contract")

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

# Record the exact artifact being deployed so the manifest can later prove
# which WASM (and which source revision) produced each contract ID.
WASM_SHA256=$(sha256sum "$WASM_PATH" | awk '{print $1}')
SOURCE_REVISION=$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown")

echo "${BLUE}Artifact SHA-256: $WASM_SHA256${NC}" >&2
echo "${BLUE}Source revision: $SOURCE_REVISION${NC}" >&2
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

# Merge this contract's ID into any existing output file for this network,
# so deploying contracts one at a time doesn't clobber previously deployed IDs.
#
# The whole read-modify-write cycle runs under an exclusive flock on
# $LOCK_FILE so two concurrent single-contract deploys cannot both read the
# same manifest and overwrite each other's added IDs. The lock is held on fd 9
# for the duration of the subshell and released automatically on exit.
update_manifest() {
    declare -A DEPLOYED_IDS
    declare -A DEPLOYED_AT
    declare -A DEPLOYED_HASH
    declare -A DEPLOYED_REV
    if [ -f "$OUTPUT_FILE" ]; then
        for c in "${CONTRACTS[@]}"; do
            existing_id=$(grep -o "\"$c\": *\"[^\"]*\"" "$OUTPUT_FILE" | grep -o '"[^"]*"$' | tr -d '"')
            if [ -n "$existing_id" ]; then
                DEPLOYED_IDS["$c"]="$existing_id"
            fi
            # Preserve any previously recorded per-contract timestamp so only
            # the contract being deployed now gets a fresh one.
            existing_ts=$(grep -o "\"$c\": *\"[^\"]*\"" "$OUTPUT_FILE" | grep -o '"[^"]*"$' | tr -d '"')
            if [ -n "$existing_ts" ]; then
                DEPLOYED_AT["$c"]="$existing_ts"
            fi
            # Preserve previously recorded artifact hash / source revision so
            # only the contract being deployed now gets fresh values.
            existing_hash=$(grep -o "\"$c\": *\"[^\"]*\"" "$OUTPUT_FILE" | grep -o '"[^"]*"$' | tr -d '"')
            if [ -n "$existing_hash" ]; then
                DEPLOYED_HASH["$c"]="$existing_hash"
            fi
            existing_rev=$(grep -o "\"$c\": *\"[^\"]*\"" "$OUTPUT_FILE" | grep -o '"[^"]*"$' | tr -d '"')
            if [ -n "$existing_rev" ]; then
                DEPLOYED_REV["$c"]="$existing_rev"
            fi
        done
    fi
    DEPLOYED_IDS["$CONTRACT_NAME"]="$CONTRACT_ID"
    DEPLOYED_AT["$CONTRACT_NAME"]="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    DEPLOYED_HASH["$CONTRACT_NAME"]="$WASM_SHA256"
    DEPLOYED_REV["$CONTRACT_NAME"]="$SOURCE_REVISION"

    {
        echo "{"
        echo "  \"network\": \"$NETWORK\","
        # Legacy single timestamp kept for backward compatibility with
        # consumers of the old manifest format. It reflects the most recent
        # deployment; per-contract times live in "deployed_at_by_contract".
        echo "  \"deployed_at\": \"${DEPLOYED_AT[$CONTRACT_NAME]}\","
        echo "  \"deployed_at_by_contract\": {"
        first_ts=true
        for c in "${CONTRACTS[@]}"; do
            if [ -n "${DEPLOYED_AT[$c]:-}" ]; then
                if [ "$first_ts" = true ]; then
                    first_ts=false
                else
                    echo ","
                fi
                printf '    "%s": "%s"' "$c" "${DEPLOYED_AT[$c]}"
            fi
        done
        echo ""
        echo "  },"
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
        echo "  },"
        # Artifact association: for each deployed contract, record the SHA-256
        # of the WASM that was deployed and the git revision it was built from.
        # Anyone can independently verify by rebuilding at the recorded
        # revision and comparing the resulting WASM hash.
        echo "  \"artifact

/* … truncated 1047 chars — edit only what you need near the top … */

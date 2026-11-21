#!/bin/bash

# FaniLab Smart Contracts - Initialization Script
# Usage: ./initialize-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"
# Opt-in escape hatch for cases where ADMIN_ADDRESS is intentionally a
# multisig (or otherwise differs from the local signing identity). When set
# to a truthy value the signer/admin compatibility preflight is skipped.
ALLOW_MULTISIGNER_ADMIN="${ALLOW_MULTISIGNER_ADMIN:-false}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONTRACT_IDS_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"
# Durable receipt recording each confirmed initialization stage and its
# transaction hash. Written incrementally so an interrupted run can be
# reconciled/resumed by an operator.
RECEIPT_FILE="${INIT_RECEIPT_FILE:-$PROJECT_ROOT/init-receipt-$NETWORK.json}"
# When true, stages already recorded as confirmed in the receipt are skipped.
RESUME_INIT="${RESUME_INIT:-true}"

echo "🔧 Initializing FaniLab Smart Contracts"
echo "======================================"
echo "Network: $NETWORK"
echo "Receipt: $RECEIPT_FILE"
echo ""

# Check if contract IDs file exists
if [ ! -f "$CONTRACT_IDS_FILE" ]; then
    echo "❌ Contract IDs file not found: $CONTRACT_IDS_FILE"
    echo "Please deploy contracts first using deploy-all-contracts.sh"
    exit 1
fi

# Verify the manifest's declared network matches the selected network before
# any contract invocation. A mismatch means the IDs belong to a different
# network and every invoke below would target the wrong deployment.
MANIFEST_NETWORK=$(grep -o '"network": *"[^"]*"' "$CONTRACT_IDS_FILE" | head -n 1 | grep -o '"[^"]*"$' | tr -d '"')
if [ -z "$MANIFEST_NETWORK" ]; then
    echo "❌ Could not read the 'network' field from $CONTRACT_IDS_FILE"
    echo "   Expected a \"network\": \"<name>\" entry in the manifest."
    exit 1
fi
if [ "$MANIFEST_NETWORK" != "$NETWORK" ]; then
    echo "❌ Network mismatch: selected network is '$NETWORK', but"
    echo "   $CONTRACT_IDS_FILE declares network '$MANIFEST_NETWORK'."
    echo "   Refusing to initialize contracts against the wrong network."
    exit 1
fi
echo "✓ Manifest network '$MANIFEST_NETWORK' matches selected network '$NETWORK'"

# Load environment variables
if [ -f "$PROJECT_ROOT/.env" ]; then
    source "$PROJECT_ROOT/.env"
else
    echo "⚠️  No .env file found. Using defaults."
fi

# Verify the signing identity exists before any invoke is attempted.
if ! stellar keys address "$DEPLOYER_IDENTITY" &> /dev/null; then
    echo "❌ Stellar CLI identity '$DEPLOYER_IDENTITY' not found."
    echo "   Every deploy/initialize script signs with it (--source $DEPLOYER_IDENTITY)."
    echo "   • CI: the 'Configure deployer identity' step in"
    echo "     .github/workflows/deploy-testnet.yml provisions it from the"
    echo "     CONTRACT_DEPLOYER_SECRET secret (environment secret"
    echo "     TESTNET_DEPLOYER_SECRET)."
    echo "   • Local: create it once with"
    echo "       stellar keys generate $DEPLOYER_IDENTITY --network $NETWORK"
    echo "     or import an existing key with"
    echo "       stellar keys add $DEPLOYER_IDENTITY --secret-key"
    exit 1
fi

# Parse contract IDs from JSON
get_contract_id() {
    grep -o "\"$1\": *\"[^\"]*\"" "$CONTRACT_IDS_FILE" | grep -o '"[^"]*"$' | tr -d '"'
}

ESCROW_ID=$(get_contract_id "escrow_contract")
DELIVERY_ID=$(get_contract_id "delivery_contract")
DISPUTE_ID=$(get_contract_id "dispute_resolution_contract")
FLEET_ID=$(get_contract_id "fleet_management_contract")
IDENTITY_ID=$(get_contract_id "identity_reputation_contract")
SETTLEMENT_ID=$(get_contract_id "settlement_contract")

for contract in \
    "escrow_contract:$ESCROW_ID" \
    "delivery_contract:$DELIVERY_ID" \
    "dispute_resolution_contract:$DISPUTE_ID" \
    "fleet_management_contract:$FLEET_ID" \
    "identity_reputation_contract:$IDENTITY_ID" \
    "settlement_contract:$SETTLEMENT_ID"; do
    if [ -z "${contract#*:}" ]; then
        echo "❌ ${contract%%:*} ID not found in $CONTRACT_IDS_FILE"
        exit 1
    fi
done

if [ -z "$ADMIN_ADDRESS" ] || [ -z "$TOKEN_ADDRESS" ]; then
    echo "❌ ADMIN_ADDRESS and TOKEN_ADDRESS must be set (in .env or the environment)"
    exit 1
fi

# Preflight: the signing identity must be able to authenticate as the
# configured admin, otherwise every admin-gated invoke below fails after
# partial setup. Fail before any mutation.
case "$(printf '%s' "$ALLOW_MULTISIGNER_ADMIN" | tr '[:upper:]' '[:lower:]')" in
    1|true|yes)
        echo "⚠️  ALLOW_MULTISIGNER_ADMIN set: skipping signer/admin compatibility check."
        echo "   Ensure ADMIN_ADDRESS ($ADMIN_ADDRESS) can authorize the calls below."
        ;;
    *)
        SIGNER_ADDRESS=$(stellar keys address "$DEPLOYER_IDENTITY" 2>/dev/null | tr -d '[:space:]')
        if [ -z "$SIGNER_ADDRESS" ]; then
            echo "❌ Could not resolve the address for identity '$DEPLOYER_IDENTITY'."
            echo "   Run 'stellar keys address $DEPLOYER_IDENTITY' to diagnose."
            exit 1
        fi
        if [ "$SIGNER_ADDRESS" != "$ADMIN_ADDRESS" ]; then
            echo "❌ Signer/admin mismatch: the CLI identity '$DEPLOYER_IDENTITY'"
            echo "   resolves to $SIGNER_ADDRESS, but ADMIN_ADDRESS is $ADMIN_ADDRESS."
            echo "   Admin-gated invokes would fail after partial setup."
            echo "   To proceed, either:"
            echo "     • set ADMIN_ADDRESS=$SIGNER_ADDRESS (or point DEPLOYER_IDENTITY"
            echo "       at the identity that controls ADMIN_ADDRESS), or"
            echo "     • if the admin is intentionally a multisig, re-run with"
            echo "       ALLOW_MULTISIGNER_ADMIN=true and complete the multisig"
            echo "       authorization out of band."
            exit 1
        fi
        echo "✓ Signer '$DEPLOYER_IDENTITY' matches ADMIN_ADDRESS"
        ;;
esac

# Preflight: verify each deployed contract actually exists on-chain and that
# its WASM identity matches the expected value recorded in the manifest.
# IDs are otherwise trusted blindly, so a stale/typo'd manifest entry would
# send init calls to a missing or wrong contract. Abort before any mutation.
#
# Expected identity is read from an optional "<name>_wasm_hash" field in the
# manifest (populated by deploy-all-contracts.sh). When absent, the check
# falls back to confirming the contract exists on-chain.
verify_deployed_contract() {
    local name=$1
    local contract_id=$2
    local expected_hash
    expected_hash=$(get_contract_id "${name}_wasm_hash")

    local on_chain
    if ! on_chain=$(stellar contract info wasm-hash \
            --id "$contract_id" \
            --network "$NETWORK" 2>/dev/null | tr -d '[:space:]'); then
        echo "❌ Contract '$name' ($contract_id) was not found on network '$NETWORK'."
        echo "   The manifest may be stale; redeploy before initializing."
        exit 1
    fi
    if [ -z "$on_chain" ]; then
        echo "❌ Could not read the WASM hash for '$name' ($contract_id) on '$NETWORK'."
        exit 1
    fi

    if [ -n "$expected_hash" ]; then
        if [ "$on_chain" != "$expected_hash" ]; then
            echo "❌ Contract '$name' identity mismatch on '$NETWORK'."
            echo "   Expected WASM hash: $expected_hash"
            echo "   Deployed WASM hash: $on_chain"
            echo "   Refusing to initialize a contract that does not match the manifest."
            exit 1
        fi
        echo "✓ $name ($contract_id) verified (wasm $on_chain)"
    else
        echo "✓ $name ($contract_id) exists on-chain (wasm $on_chain; no expected hash in manifest)"
    fi
}

echo ""
echo "Verifying deployed contracts..."
verify_deployed_contract "escrow_contract" "$ESCROW_ID"
verify_deployed_contract "delivery_contract" "$DELIVERY_ID"
verify_deployed_contract "dispute_resolution_contract" "$DISPUTE_ID"
verify_deployed_contract "fleet_management_contract" "$FLEET_ID"
verify_deployed_contract "identity_reputation_contract" "$IDENTITY_ID"
verify_deployed_contract "settlement_contract" "$SETTLEMENT_ID"

echo ""

# ---------------------------------------------------------------------------
# Durable initialization receipt
# ---------------------------------------------------------------------------
# The receipt is a JSON document written incrementally after each confirmed
# stage. It records the network, the manifest, and for every stage the
# contract, the invoked function, the transaction hash, and a timestamp.
# Operators can inspect it after an interruption to see exactly which stages
# completed and resume from there (RESUME_INIT=true, the default).

receipt_init() {
    if [ -f "$RECEIPT_FILE" ]; then
        echo "↻ Found existing receipt: $RECEIPT_FILE"
        echo "  Confirmed stages will be skipped (set RESUME_INIT=false to re-run all)."
        return
    fi
    local started_at
    started_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    cat > "$RECEIPT_FILE" <<EOF
{
  "network": "$NETWORK",
  "manifest": "$CONTRACT_IDS_FILE",
  "started_at": "$started_at",
  "stages": []
}
EOF
    echo "✓ Created receipt: $RECEIPT_FILE"
}

# Returns 0 if the given stage is already recorded as confirmed.
receipt_has_stage() {
    local stage=$1
    [ -f "$RECEIPT_FILE" ] || return 1
    grep -q "\"stage\": *\"$stage\"" "$RECEIPT_FILE"
}

# Append a confirmed stage (with its transaction hash) to the receipt.
receipt_record_stage() {
    local stage=$1
    local contract_id=$2
    local function_name=$3
    local tx_hash=$4
    local recorded_at
    recorded_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    local entry
    entry=$(printf '    {"stage": "%s", "contract": "%s", "function": "%s", "tx_hash": "%s", "confirmed_at": "%s"}' \
        "$stage" "$contract_id" "$function_name" "$tx_hash" "$recorded_at")

    # Insert the entry before the closing "stages" array bracket. The receipt
    # is small and machine-generated, so a targeted rewrite is sufficient.
    local tmp
    tmp=$(mktemp)
    awk -v entry="$entry" '
        /^  \]\$/ && !done {
            if (last) { print last "," }
            print entry
            done = 1
            next
        }
        { if (last) print last; last = $0 }
        END { if (last) print last }
    ' "$RECEIPT_FILE" > "$tmp"
    mv "$tmp" "$RECEIPT_FILE"
}

receipt_init

echo ""

invoke_contract() {
    local contract_id=$1
    shift
    ste

/* … truncated 2179 chars — edit only what you need near the top … */

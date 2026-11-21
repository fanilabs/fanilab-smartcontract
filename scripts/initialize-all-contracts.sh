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

echo "🔧 Initializing FaniLab Smart Contracts"
echo "======================================"
echo "Network: $NETWORK"
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

echo ""

invoke_contract() {
    local contract_id=$1
    shift
    stellar contract invoke \
        --id "$contract_id" \
        --source "$DEPLOYER_IDENTITY" \
        --network "$NETWORK" \
        -- "$@"
}

DISPUTE_TIME_LIMIT=${DISPUTE_TIME_LIMIT:-86400}
DISPUTE_RESOLUTION_LIMIT=${DISPUTE_RESOLUTION_LIMIT:-86400}

echo "Escrow Contract: $ESCROW_ID"
echo "Delivery Contract: $DELIVERY_ID"
echo "Dispute Resolution Contract: $DISPUTE_ID"
echo "Fleet Management Contract: $FLEET_ID"
echo "Identity Reputation Contract: $IDENTITY_ID"
echo "Settlement Contract: $SETTLEMENT_ID"
echo ""

echo "Initializing Escrow Contract..."
invoke_contract "$ESCROW_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --token "$TOKEN_ADDRESS" \
    --platform_fee_bps "${PLATFORM_FEE_BPS:-250}"
echo "✓ Escrow initialized"

echo "Initializing Delivery Contract..."
invoke_contract "$DELIVERY_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
echo "✓ Delivery initialized"

echo "Initializing Dispute Resolution Contract..."
invoke_contract "$DISPUTE_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --escrow_contract "$ESCROW_ID" \
    --dispute_time_limit "$DISPUTE_TIME_LIMIT" \
    --dispute_resolution_limit "$DISPUTE_RESOLUTION_LIMIT"
echo "✓ Dispute resolution initialized"

echo "Initializing Fleet Management Contract..."
invoke_contract "$FLEET_ID" init \
    --admin "$ADMIN_ADDRESS"
echo "✓ Fleet management initialized"

echo "Initializing Identity Reputation Contract..."
invoke_contract "$IDENTITY_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --dispute_contract "$DISPUTE_ID"
echo "✓ Identity reputation initialized"

echo "Initializing Settlement Contract..."
invoke_contract "$SETTLEMENT_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
echo "✓ Settlement initialized"
echo ""

echo "Wiring contract integrations..."
invoke_contract "$DELIVERY_ID" set_escrow_contract \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_contract "$DELIVERY_ID" set_dispute_resolution_contract \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"
invoke_contract "$DELIVERY_ID" set_identity_reputation_contract \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

invoke_contract "$DISPUTE_ID" set_delivery_contract \
    --caller "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
invoke_contract "$DISPUTE_ID" set_escrow_contract \
    --caller "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_contract "$DISPUTE_ID" set_identity_reputation_contract \
    --caller "$ADMIN_ADDRESS" \
    --reputation_contract "$IDENTITY_ID"

invoke_contract "$FLEET_ID" set_escrow_contract \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_contract "$FLEET_ID" set_identity_contract \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

invoke_contract "$IDENTITY_ID" set_delivery_contract \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
invoke_contract "$IDENTITY_ID" set_dispute_contr

/* … truncated 812 chars — edit only what you need near the top … */

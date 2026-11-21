#!/bin/bash

# FaniLab Smart Contracts - Initialization Script
# Usage: ./initialize-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"
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

invoke_contract() {
    local contract_id=$1
    shift
    stellar contract invoke \
        --id "$contract_id" \
        --source "$DEPLOYER_IDENTITY" \
        --network "$NETWORK" \
        -- "$@"
}

# Read a currently-installed peer address from a contract's view function.
# Returns the address on success, or an empty string if the view is not
# available (e.g. not yet wired). Used to skip redundant timed peer setters
# that would otherwise queue a pending proposal for an already-installed peer.
read_active_address() {
    local contract_id=$1
    shift
    invoke_contract "$contract_id" "$@" 2>/dev/null | tr -d '"' | tr -d '[:space:]' || true
}

# Invoke a timed peer setter only when the target differs from the active
# installed address. Skips identical assignments so initial wiring leaves no
# redundant pending proposals for current dependencies.
set_peer_if_changed() {
    local contract_id=$1
    local view_fn=$2
    local setter_fn=$3
    local current=$4
    shift 4

    local active
    active=$(read_active_address "$contract_id" "$view_fn")
    if [ -n "$active" ] && [ "$active" = "$current" ]; then
        echo "  ↷ $setter_fn already set to $current (skipping)"
        return 0
    fi
    invoke_contract "$contract_id" "$setter_fn" "$@"
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
set_peer_if_changed "$DELIVERY_ID" get_escrow_contract set_escrow_contract "$ESCROW_ID" \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
set_peer_if_changed "$DELIVERY_ID" get_dispute_resolution_contract set_dispute_resolution_contract "$DISPUTE_ID" \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"
set_peer_if_changed "$DELIVERY_ID" get_identity_reputation_contract set_identity_reputation_contract "$IDENTITY_ID" \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

set_peer_if_changed "$DISPUTE_ID" get_delivery_contract set_delivery_contract "$DELIVERY_ID" \
    --caller "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
set_peer_if_changed "$DISPUTE_ID" get_escrow_contract set_escrow_contract "$ESCROW_ID" \
    --caller "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
set_peer_if_changed "$DISPUTE_ID" get_identity_reputation_contract set_identity_reputation_contract "$IDENTITY_ID" \
    --caller "$ADMIN_ADDRESS" \
    --reputation_contract "$IDENTITY_ID"

set_peer_if_changed "$FLEET_ID" get_escrow_contract set_escrow_contract "$ESCROW_ID" \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
set_peer_if_changed "$FLEET_ID" get_identity_contract set_identity_contract "$IDENTITY_ID" \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

set_peer_if_changed "$IDENTITY_ID" get_delivery_contract set_delivery_contract "$DELIVERY_ID" \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
set_peer_if_changed "$IDENTITY_ID" get_dispute_contract set_dispute_contract "$DISPUTE_ID" \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"

set_peer_if_changed "$SETTLEMENT_ID" get_escrow_contract set_escrow_contract "$ESCROW_ID" \
    --admin "$ADMIN_ADDRESS" \
    --new_escrow "$ESCROW_ID"

set_peer_if_changed "$ESCROW_ID" get_delivery_contract set_delivery_contract "$DELIVERY_ID" \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
set_peer_if_changed "$ESCROW_ID" get_dispute_resolution_contract set_dispute_resolution_contract "$DISPUTE_ID" \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"
set_peer_if_changed "$ESCROW_ID" get_identity_reputation_contract set_identity_reputation_contract "$IDENTITY_ID" \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"
set_peer_if_changed "$ESCROW_ID" get_fleet_management_contract set_fleet_management_contract "$FLEET_ID" \
    --admin "$ADMIN_ADDRESS" \
    --fleet_contract "$FLEET_ID"

echo "✓ Contract integrations configured"
echo ""
echo "✅ All contracts initialized successfully!"
exit 0

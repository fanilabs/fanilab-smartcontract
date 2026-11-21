#!/bin/bash

# FaniLab Smart Contracts - Initialization Script
# Usage: ./initialize-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"
# Per-stage timeout (seconds) applied to every CLI invoke so a hung RPC call
# cannot block the whole initialization sequence. Override with STAGE_TIMEOUT.
STAGE_TIMEOUT="${STAGE_TIMEOUT:-120}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONTRACT_IDS_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"

# Tracks the last stage that completed successfully so a timeout/failure can
# report exactly where the sequence stopped.
LAST_CONFIRMED_STAGE="(none)"

# Run a CLI command under a bounded timeout. On timeout or failure, report the
# last confirmed stage and exit non-zero instead of blocking indefinitely.
run_with_timeout() {
    local stage=$1
    shift
    local rc=0
    if command -v timeout &> /dev/null; then
        timeout "$STAGE_TIMEOUT" "$@" || rc=$?
    else
        "$@" || rc=$?
    fi
    if [ "$rc" -eq 124 ]; then
        echo ""
        echo "❌ Stage '$stage' timed out after ${STAGE_TIMEOUT}s"
        echo "   Last confirmed stage: $LAST_CONFIRMED_STAGE"
        echo "   Re-run with a larger STAGE_TIMEOUT if the network is slow."
        exit 1
    elif [ "$rc" -ne 0 ]; then
        echo ""
        echo "❌ Stage '$stage' failed (exit $rc)"
        echo "   Last confirmed stage: $LAST_CONFIRMED_STAGE"
        exit "$rc"
    fi
    LAST_CONFIRMED_STAGE=$stage
}

echo "🔧 Initializing FaniLab Smart Contracts"
echo "======================================"
echo "Network: $NETWORK"
echo "Stage timeout: ${STAGE_TIMEOUT}s"
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

# Bounded invoke: wraps invoke_contract with the per-stage timeout and stage
# label so a hung RPC call is reported with the last confirmed transaction.
invoke_stage() {
    local stage=$1
    shift
    run_with_timeout "$stage" invoke_contract "$@"
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
invoke_stage "escrow.init" "$ESCROW_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --token "$TOKEN_ADDRESS" \
    --platform_fee_bps "${PLATFORM_FEE_BPS:-250}"
echo "✓ Escrow initialized"

echo "Initializing Delivery Contract..."
invoke_stage "delivery.init" "$DELIVERY_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
echo "✓ Delivery initialized"

echo "Initializing Dispute Resolution Contract..."
invoke_stage "dispute.init" "$DISPUTE_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --escrow_contract "$ESCROW_ID" \
    --dispute_time_limit "$DISPUTE_TIME_LIMIT" \
    --dispute_resolution_limit "$DISPUTE_RESOLUTION_LIMIT"
echo "✓ Dispute resolution initialized"

echo "Initializing Fleet Management Contract..."
invoke_stage "fleet.init" "$FLEET_ID" init \
    --admin "$ADMIN_ADDRESS"
echo "✓ Fleet management initialized"

echo "Initializing Identity Reputation Contract..."
invoke_stage "identity.init" "$IDENTITY_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --dispute_contract "$DISPUTE_ID"
echo "✓ Identity reputation initialized"

echo "Initializing Settlement Contract..."
invoke_stage "settlement.init" "$SETTLEMENT_ID" init \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
echo "✓ Settlement initialized"
echo ""

echo "Wiring contract integrations..."
invoke_stage "delivery.set_escrow_contract" "$DELIVERY_ID" set_escrow_contract \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_stage "delivery.set_dispute_resolution_contract" "$DELIVERY_ID" set_dispute_resolution_contract \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"
invoke_stage "delivery.set_identity_reputation_contract" "$DELIVERY_ID" set_identity_reputation_contract \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

invoke_stage "dispute.set_delivery_contract" "$DISPUTE_ID" set_delivery_contract \
    --caller "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
invoke_stage "dispute.set_escrow_contract" "$DISPUTE_ID" set_escrow_contract \
    --caller "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_stage "dispute.set_identity_reputation_contract" "$DISPUTE_ID" set_identity_reputation_contract \
    --caller "$ADMIN_ADDRESS" \
    --reputation_contract "$IDENTITY_ID"

invoke_stage "fleet.set_escrow_contract" "$FLEET_ID" set_escrow_contract \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
invoke_stage "fleet.set_identity_contract" "$FLEET_ID" set_identity_contract \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"

invoke_stage "identity.set_delivery_contract" "$IDENTITY_ID" set_delivery_contract \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
invoke_stage "identity.set_dispute_contract" "$IDENTITY_ID" set_dispute_contract \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"

invoke_stage "settlement.set_escrow_contract" "$SETTLEMENT_ID" set_escrow_contract \
    --admin "$ADMIN_ADDRESS" \
    --new_escrow "$ESCROW_ID"

invoke_stage "escrow.set_delivery_contract" "$ESCROW_ID" set_delivery_contract \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID"
invoke_stage "escrow.set_dispute_resolution_contract" "$ESCROW_ID" set_dispute_resolution_contract \
    --admin "$ADMIN_ADDRESS" \
    --dispute_contract "$DISPUTE_ID"
invoke_stage "escrow.set_identity_reputation_contract" "$ESCROW_ID" set_identity_reputation_contract \
    --admin "$ADMIN_ADDRESS" \
    --identity_contract "$IDENTITY_ID"
invoke_stage "escrow.set_fleet_management_contract" "$ESCROW_ID" set_fleet_management_contract \
    --admin "$ADMIN_ADDRESS" \
    --fleet_contract "$FLEET_ID"

echo "✓ Contract integrations configured"
echo ""
echo "✅ All contracts initialized successfully!"
exit 0

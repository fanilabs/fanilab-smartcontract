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

# ---------------------------------------------------------------------------
# Preflight validation
#
# Every initialization parameter is validated up front so that a bad setting
# aborts the run *before* the first state-mutating invoke. All errors are
# aggregated and reported together.
# ---------------------------------------------------------------------------
PREFLIGHT_ERRORS=()

# Validate a Stellar StrKey address: account (G...) or contract (C...).
# StrKey is base32 (RFC 4648 alphabet) and 56 characters long.
validate_address() {
    local label=$1
    local value=$2
    if [ -z "$value" ]; then
        PREFLIGHT_ERRORS+=("$label is required but empty")
        return
    fi
    if ! [[ "$value" =~ ^[GC][A-Z2-7]{55}$ ]]; then
        PREFLIGHT_ERRORS+=("$label is not a valid Stellar address (expected G.../C... StrKey, 56 chars): '$value'")
    fi
}

# Validate an unsigned integer within an inclusive range.
validate_uint() {
    local label=$1
    local value=$2
    local min=$3
    local max=$4
    if [ -z "$value" ]; then
        PREFLIGHT_ERRORS+=("$label is required but empty")
        return
    fi
    if ! [[ "$value" =~ ^[0-9]+$ ]]; then
        PREFLIGHT_ERRORS+=("$label must be a non-negative integer: '$value'")
        return
    fi
    if [ "$value" -lt "$min" ] || [ "$value" -gt "$max" ]; then
        PREFLIGHT_ERRORS+=("$label must be between $min and $max: '$value'")
    fi
}

PLATFORM_FEE_BPS=${PLATFORM_FEE_BPS:-250}
DISPUTE_TIME_LIMIT=${DISPUTE_TIME_LIMIT:-86400}
DISPUTE_RESOLUTION_LIMIT=${DISPUTE_RESOLUTION_LIMIT:-86400}

# Address parameters
validate_address "ADMIN_ADDRESS" "$ADMIN_ADDRESS"
validate_address "TOKEN_ADDRESS" "$TOKEN_ADDRESS"
validate_address "escrow_contract" "$ESCROW_ID"
validate_address "delivery_contract" "$DELIVERY_ID"
validate_address "dispute_resolution_contract" "$DISPUTE_ID"
validate_address "fleet_management_contract" "$FLEET_ID"
validate_address "identity_reputation_contract" "$IDENTITY_ID"
validate_address "settlement_contract" "$SETTLEMENT_ID"

# Numeric parameters
# Platform fee is expressed in basis points (0-10000 = 0%-100%).
validate_uint "PLATFORM_FEE_BPS" "$PLATFORM_FEE_BPS" 0 10000
# Dispute time limits are durations in seconds; must be positive.
validate_uint "DISPUTE_TIME_LIMIT" "$DISPUTE_TIME_LIMIT" 1 31536000
validate_uint "DISPUTE_RESOLUTION_LIMIT" "$DISPUTE_RESOLUTION_LIMIT" 1 31536000

if [ ${#PREFLIGHT_ERRORS[@]} -gt 0 ]; then
    echo "❌ Preflight validation failed; no contracts were initialized:"
    for err in "${PREFLIGHT_ERRORS[@]}"; do
        echo "   • $err"
    done
    exit 1
fi

echo "✓ Preflight validation passed"
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

# Determine whether a contract has already been initialized by querying its
# admin view. Returns 0 (initialized) when the view yields a non-empty address,
# 1 (not initialized) otherwise. This lets a rerun after a partial failure
# resume without re-invoking init on contracts that are already configured.
is_initialized() {
    local contract_id=$1
    local admin
    admin=$(read_active_address "$contract_id" get_admin)
    [ -n "$admin" ]
}

# Run a contract's init only when it has not been initialized yet. Existing
# configuration is preserved on resume; uninitialized contracts still get
# initialized exactly as before.
init_if_needed() {
    local contract_id=$1
    local label=$2
    shift 2

    if is_initialized "$contract_id"; then
        echo "  ↷ $label already initialized (skipping)"
        return 0
    fi
    invoke_contract "$contract_id" init "$@"
    echo "✓ $label initialized"
}

echo "Escrow Contract: $ESCROW_ID"
echo "Delivery Contract: $DELIVERY_ID"
echo "Dispute Resolution Contract: $DISPUTE_ID"
echo "Fleet Management Contract: $FLEET_ID"
echo "Identity Reputation Contract: $IDENTITY_ID"
echo "Settlement Contract: $SETTLEMENT_ID"
echo ""

echo "Initializing Escrow Contract..."
init_if_needed "$ESCROW_ID" "Escrow" \
    --admin "$ADMIN_ADDRESS" \
    --token "$TOKEN_ADDRESS" \
    --platform_fee_bps "$PLATFORM_FEE_BPS"

echo "Initializing Delivery Contract..."
init_if_needed "$DELIVERY_ID" "Delivery" \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"

echo "Initializing Dispute Resolution Contract..."
init_if_needed "$DISPUTE_ID" "Dispute resolution" \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --escrow_contract "$ESCROW_ID" \
    --dispute_time_limit "$DISPUTE_TIME_LIMIT" \
    --dispute_resolution_limit "$DISPUTE_RESOLUTION_LIMIT"

echo "Initializing Fleet Management Contract..."
init_if_needed "$FLEET_ID" "Fleet management" \
    --admin "$ADMIN_ADDRESS"

echo "Initializing Identity Reputation Contract..."
init_if_needed "$IDENTITY_ID" "Identity reputation" \
    --admin "$ADMIN_ADDRESS" \
    --delivery_contract "$DELIVERY_ID" \
    --dispute_contract "$DISPUTE_ID"

echo "Initializing Settlement Contract..."
init_if_needed "$SETTLEMENT_ID" "Settlement" \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"
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

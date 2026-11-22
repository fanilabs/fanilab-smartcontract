#!/bin/bash

# FaniLab Smart Contracts - Single Contract Initialization Script
# Usage: ./initialize-contract.sh <contract_name> [network]
#
# Configuration file precedence (highest to lowest):
#   1. Explicit path via --config <path> flag
#   2. Explicit path via CONFIG_FILE environment variable
#   3. Default repository .env ($PROJECT_ROOT/.env)

set -e

CONTRACT_NAME=$1
NETWORK=${2:-testnet}
# Name of the Stellar CLI identity every deploy/initialize script signs with.
# Overridable, but the default must match the identity provisioned by
# .github/workflows/deploy-testnet.yml and documented in docs/DEPLOYMENT.md.
DEPLOYER_IDENTITY="${DEPLOYER_IDENTITY:-deployer}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONTRACT_IDS_FILE="$PROJECT_ROOT/contract-ids-$NETWORK.json"

# Resolve the configuration file path. An explicit path (--config flag or
# CONFIG_FILE env var) takes precedence over the default repository .env.
CONFIG_FILE="${CONFIG_FILE:-}"
if [ "$2" = "--config" ] || [ "$3" = "--config" ]; then
    if [ "$2" = "--config" ]; then
        CONFIG_FILE="$3"
    else
        CONFIG_FILE="$4"
    fi
fi
if [ -z "$CONFIG_FILE" ]; then
    CONFIG_FILE="$PROJECT_ROOT/.env"
fi

SUPPORTED_CONTRACTS=(
    "escrow_contract"
    "delivery_contract"
    "dispute_resolution_contract"
    "fleet_management_contract"
    "identity_reputation_contract"
    "settlement_contract"
)

if [ -z "$CONTRACT_NAME" ]; then
    echo "❌ Usage: $0 <contract_name> [network] [--config <path>]"
    echo "   Supported contract names: ${SUPPORTED_CONTRACTS[*]}"
    exit 1
fi

echo "🔧 Initializing FaniLab Smart Contract"
echo "======================================"
echo "Contract: $CONTRACT_NAME"
echo "Network: $NETWORK"
echo "Config: $CONFIG_FILE"
echo ""

# Check if contract IDs file exists
if [ ! -f "$CONTRACT_IDS_FILE" ]; then
    echo "❌ Contract IDs file not found: $CONTRACT_IDS_FILE"
    echo "Please deploy the contract first using deploy-contract.sh or deploy-all-contracts.sh"
    exit 1
fi

# Load environment variables from the resolved configuration file.
if [ -f "$CONFIG_FILE" ]; then
    source "$CONFIG_FILE"
else
    echo "⚠️  No config file found at '$CONFIG_FILE'. Using defaults."
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

get_contract_id() {
    grep -o "\"$1\": *\"[^\"]*\"" "$CONTRACT_IDS_FILE" | grep -o '"[^"]*"$' | tr -d '"'
}

case "$CONTRACT_NAME" in
    escrow_contract)
        ESCROW_ID=$(get_contract_id "escrow_contract")
        if [ -z "$ESCROW_ID" ]; then
            echo "❌ escrow_contract ID not found in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ] || [ -z "$TOKEN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS and TOKEN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Escrow Contract: $ESCROW_ID"
        echo ""
        echo "Initializing Escrow Contract..."
        stellar contract invoke \
            --id "$ESCROW_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS" \
            --token "$TOKEN_ADDRESS" \
            --platform_fee_bps "${PLATFORM_FEE_BPS:-250}"

        echo "✓ Escrow initialized"
        ;;
    delivery_contract)
        DELIVERY_ID=$(get_contract_id "delivery_contract")
        ESCROW_ID=$(get_contract_id "escrow_contract")
        if [ -z "$DELIVERY_ID" ]; then
            echo "❌ delivery_contract ID not found in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ESCROW_ID" ]; then
            echo "❌ escrow_contract must be deployed first (its ID was not found in $CONTRACT_IDS_FILE)"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Delivery Contract: $DELIVERY_ID"
        echo "Escrow Contract: $ESCROW_ID"
        echo ""
        echo "Initializing Delivery Contract..."
        stellar contract invoke \
            --id "$DELIVERY_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS" \
            --escrow_contract "$ESCROW_ID"

        echo "✓ Delivery initialized"
        ;;
    dispute_resolution_contract)
        DISPUTE_ID=$(get_contract_id "dispute_resolution_contract")
        DELIVERY_ID=$(get_contract_id "delivery_contract")
        ESCROW_ID=$(get_contract_id "escrow_contract")
        if [ -z "$DISPUTE_ID" ] || [ -z "$DELIVERY_ID" ] || [ -z "$ESCROW_ID" ]; then
            echo "❌ dispute_resolution_contract, delivery_contract, and escrow_contract IDs must be present in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Initializing Dispute Resolution Contract..."
        stellar contract invoke \
            --id "$DISPUTE_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS" \
            --delivery_contract "$DELIVERY_ID" \
            --escrow_contract "$ESCROW_ID" \
            --dispute_time_limit "${DISPUTE_TIME_LIMIT:-86400}" \
            --dispute_resolution_limit "${DISPUTE_RESOLUTION_LIMIT:-86400}"

        echo "✓ Dispute resolution initialized"
        ;;
    fleet_management_contract)
        FLEET_ID=$(get_contract_id "fleet_management_contract")
        if [ -z "$FLEET_ID" ]; then
            echo "❌ fleet_management_contract ID not found in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Initializing Fleet Management Contract..."
        stellar contract invoke \
            --id "$FLEET_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS"

        echo "✓ Fleet management initialized"
        ;;
    identity_reputation_contract)
        IDENTITY_ID=$(get_contract_id "identity_reputation_contract")
        DELIVERY_ID=$(get_contract_id "delivery_contract")
        DISPUTE_ID=$(get_contract_id "dispute_resolution_contract")
        if [ -z "$IDENTITY_ID" ] || [ -z "$DELIVERY_ID" ] || [ -z "$DISPUTE_ID" ]; then
            echo "❌ identity_reputation_contract, delivery_contract, and dispute_resolution_contract IDs must be present in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Initializing Identity Reputation Contract..."
        stellar contract invoke \
            --id "$IDENTITY_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS" \
            --delivery_contract "$DELIVERY_ID" \
            --dispute_contract "$DISPUTE_ID"

        echo "✓ Identity reputation initialized"
        ;;
    settlement_contract)
        SETTLEMENT_ID=$(get_contract_id "settlement_contract")
        ESCROW_ID=$(get_contract_id "escrow_contract")
        if [ -z "$SETTLEMENT_ID" ] || [ -z "$ESCROW_ID" ]; then
            echo "❌ settlement_contract and escrow_contract IDs must be present in $CONTRACT_IDS_FILE"
            exit 1
        fi
        if [ -z "$ADMIN_ADDRESS" ]; then
            echo "❌ ADMIN_ADDRESS must be set (in $CONFIG_FILE or the environment)"
            exit 1
        fi

        echo "Initializing Settlement Contract..."
        stellar contract invoke \
            --id "$SETTLEMENT_ID" \
            --source "$DEPLOYER_IDENTITY" \
            --network "$NETWORK" \
            -- init \
            --admin "$ADMIN_ADDRESS" \
            --escrow_contract "$ESCROW_ID"

        echo "✓ Settlement initialized"
        ;;
    *)
        echo "❌ Unknown contract name: $CONTRACT_NAME"
        echo "   Supported contract names: ${SUPPORTED_CONTRACTS[*]}"
        exit 1
        ;;
esac

echo ""
echo "✅ Initialization complete for $CONTRACT_NAME"

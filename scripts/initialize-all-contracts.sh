#!/bin/bash

# FaniLab Smart Contracts - Initialization Script
# Usage: ./initialize-all-contracts.sh [testnet|mainnet]

set -e

NETWORK=${1:-testnet}
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

# Parse contract IDs from JSON
ESCROW_ID=$(grep -o '"escrow_contract": "[^"]*' "$CONTRACT_IDS_FILE" | grep -o '[^"]*$')
DELIVERY_ID=$(grep -o '"delivery_contract": "[^"]*' "$CONTRACT_IDS_FILE" | grep -o '[^"]*$')

echo "Escrow Contract: $ESCROW_ID"
echo "Delivery Contract: $DELIVERY_ID"
echo ""

# Initialize Escrow Contract
echo "Initializing Escrow Contract..."
stellar contract invoke \
    --id "$ESCROW_ID" \
    --source deployer \
    --network "$NETWORK" \
    -- init \
    --admin "$ADMIN_ADDRESS" \
    --token "$TOKEN_ADDRESS" \
    --platform_fee_bps 250

echo "✓ Escrow initialized"
echo ""

# Initialize Delivery Contract
echo "Initializing Delivery Contract..."
stellar contract invoke \
    --id "$DELIVERY_ID" \
    --source deployer \
    --network "$NETWORK" \
    -- init \
    --admin "$ADMIN_ADDRESS" \
    --escrow_contract "$ESCROW_ID"

echo "✓ Delivery initialized"
echo ""

echo "✅ All contracts initialized successfully!"
exit 0

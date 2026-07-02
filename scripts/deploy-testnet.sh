#!/bin/bash
set -e

echo "=================================================="
echo "  StepFi Contracts — Testnet Deployment Script"
echo "=================================================="

# Check required tools
command -v stellar >/dev/null 2>&1 || { echo "Error: stellar CLI not found. Install: curl -L https://github.com/stellar/stellar-cli/releases/latest/download/stellar-cli-x86_64-unknown-linux-gnu.tar.gz | tar -xz -C ~/.cargo/bin/"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo not found. Install Rust first."; exit 1; }

NETWORK="testnet"
SOURCE="${DEPLOYER_ALIAS:-stepfi-deployer}"

echo ""
echo "Network: $NETWORK"
echo "Source:  $SOURCE"
echo ""

# Build deps first (creditline uses contractimport! which reads dep WASMs)
echo "Step 1 — Building dependency contracts..."
cargo build --target wasm32-unknown-unknown --release \
  -p liquidity-pool-contract \
  -p vendor-registry-contract \
  -p parameters-contract \
  -p reputation-contract 2>&1 | tail -3

echo "Step 2 — Building creditline-contract..."
cargo build --target wasm32-unknown-unknown --release -p creditline-contract 2>&1 | tail -3
echo "Build complete."
echo ""

WASM_DIR="target/wasm32-unknown-unknown/release"

# Optimize WASMs (required: strips reference-types that Soroban runtime rejects)
echo "Step 3 — Optimizing WASMs..."
for c in parameters_contract reputation_contract vendor_registry_contract liquidity_pool_contract creditline_contract; do
  stellar contract optimize --wasm "$WASM_DIR/${c}.wasm" 2>&1 | tail -1
done
echo ""

# Deploy in dependency order
echo "Step 4 — Deploying contracts..."

echo "Deploying parameters-contract..."
PARAMETERS_ID=$(stellar contract deploy \
  --wasm $WASM_DIR/parameters_contract.optimized.wasm \
  --source $SOURCE \
  --network $NETWORK 2>&1 | tail -1)
echo "  PARAMETERS_CONTRACT_ID=$PARAMETERS_ID"

echo "Deploying reputation-contract..."
REPUTATION_ID=$(stellar contract deploy \
  --wasm $WASM_DIR/reputation_contract.optimized.wasm \
  --source $SOURCE \
  --network $NETWORK 2>&1 | tail -1)
echo "  REPUTATION_CONTRACT_ID=$REPUTATION_ID"

echo "Deploying vendor-registry-contract..."
VENDOR_REGISTRY_ID=$(stellar contract deploy \
  --wasm $WASM_DIR/vendor_registry_contract.optimized.wasm \
  --source $SOURCE \
  --network $NETWORK 2>&1 | tail -1)
echo "  VENDOR_REGISTRY_CONTRACT_ID=$VENDOR_REGISTRY_ID"

echo "Deploying liquidity-pool-contract..."
LIQUIDITY_POOL_ID=$(stellar contract deploy \
  --wasm $WASM_DIR/liquidity_pool_contract.optimized.wasm \
  --source $SOURCE \
  --network $NETWORK 2>&1 | tail -1)
echo "  LIQUIDITY_POOL_CONTRACT_ID=$LIQUIDITY_POOL_ID"

echo "Deploying creditline-contract..."
CREDITLINE_ID=$(stellar contract deploy \
  --wasm $WASM_DIR/creditline_contract.optimized.wasm \
  --source $SOURCE \
  --network $NETWORK 2>&1 | tail -1)
echo "  CREDIT_LINE_CONTRACT_ID=$CREDITLINE_ID"

# Step 5 — Initialize each contract
echo ""
echo "Step 5 — Initializing contracts..."

ADMIN_PUBKEY=$(stellar keys address $SOURCE 2>/dev/null)
# Native XLM Stellar Asset Contract (testnet). Override with TOKEN_ID env var
# for a custom SEP-41 token.
TOKEN_ID="${TOKEN_ID:-$(stellar contract id asset --asset native --network $NETWORK 2>/dev/null)}"
TREASURY="${TREASURY:-$ADMIN_PUBKEY}"
MERCHANT_FUND="${MERCHANT_FUND:-$ADMIN_PUBKEY}"

echo "  Admin:         $ADMIN_PUBKEY"
echo "  Token:         $TOKEN_ID"
echo "  Treasury:      $TREASURY"
echo "  Merchant fund: $MERCHANT_FUND"

echo "Initializing parameters..."
stellar contract invoke --id $PARAMETERS_ID --source $SOURCE --network $NETWORK \
  -- initialize_defaults --admin $SOURCE 2>&1 | tail -1

echo "Initializing reputation..."
# reputation has no `initialize` — first set_admin call (no prior admin) seeds it.
stellar contract invoke --id $REPUTATION_ID --source $SOURCE --network $NETWORK \
  -- set_admin --new_admin $ADMIN_PUBKEY 2>&1 | tail -1

echo "Initializing vendor_registry..."
stellar contract invoke --id $VENDOR_REGISTRY_ID --source $SOURCE --network $NETWORK \
  -- initialize --admin $ADMIN_PUBKEY 2>&1 | tail -1

echo "Initializing liquidity_pool..."
stellar contract invoke --id $LIQUIDITY_POOL_ID --source $SOURCE --network $NETWORK \
  -- initialize \
  --token $TOKEN_ID \
  --treasury $TREASURY \
  --admin $ADMIN_PUBKEY \
  --merchant_fund $MERCHANT_FUND 2>&1 | tail -1

echo "Initializing creditline..."
stellar contract invoke --id $CREDITLINE_ID --source $SOURCE --network $NETWORK \
  -- initialize \
  --vendor_registry $VENDOR_REGISTRY_ID \
  --liquidity_pool $LIQUIDITY_POOL_ID \
  --reputation_contract $REPUTATION_ID \
  --token $TOKEN_ID \
  --admin $ADMIN_PUBKEY 2>&1 | tail -1

echo ""
echo "Step 6 — Writing .env.contracts and deployed-testnet.json..."

cat > .env.contracts << ENVEOF
PARAMETERS_CONTRACT_ID=$PARAMETERS_ID
REPUTATION_CONTRACT_ID=$REPUTATION_ID
VENDOR_REGISTRY_CONTRACT_ID=$VENDOR_REGISTRY_ID
LIQUIDITY_POOL_CONTRACT_ID=$LIQUIDITY_POOL_ID
CREDIT_LINE_CONTRACT_ID=$CREDITLINE_ID
ENVEOF

DEPLOYER_PUBKEY=$(stellar keys address $SOURCE 2>/dev/null)
TODAY=$(date -u +%Y-%m-%d)
cat > contracts/deployed-testnet.json << JSONEOF
{
  "network": "testnet",
  "deployer": "$DEPLOYER_PUBKEY",
  "deployedAt": "$TODAY",
  "token": {
    "asset": "native",
    "sac": "$TOKEN_ID"
  },
  "contracts": {
    "parameters": {
      "id": "$PARAMETERS_ID",
      "initialized": true,
      "initializedAt": "$TODAY",
      "initMethod": "initialize_defaults(admin)"
    },
    "reputation": {
      "id": "$REPUTATION_ID",
      "initialized": true,
      "initializedAt": "$TODAY",
      "initMethod": "set_admin(new_admin)"
    },
    "vendorRegistry": {
      "id": "$VENDOR_REGISTRY_ID",
      "initialized": true,
      "initializedAt": "$TODAY",
      "initMethod": "initialize(admin)"
    },
    "liquidityPool": {
      "id": "$LIQUIDITY_POOL_ID",
      "initialized": true,
      "initializedAt": "$TODAY",
      "initMethod": "initialize(token, treasury, admin, merchant_fund)"
    },
    "creditline": {
      "id": "$CREDITLINE_ID",
      "initialized": true,
      "initializedAt": "$TODAY",
      "initMethod": "initialize(vendor_registry, liquidity_pool, reputation_contract, token, admin)"
    }
  }
}
JSONEOF

echo ""
echo "Most recent deployment (2026-05-11):"
echo "  parameters:     CA6JVOSYPCCEIVJA5O2SBJIDVUXJOK5U6YF25M3EAJCNZLWKMSZPTKXT"
echo "  reputation:     CC3BO57ZRJGA63QJBIBSOMI25Z3X2I5CYTARYRAUXUAILX6L3OWBL5SB"
echo "  vendorRegistry: CCZ6T6NYCDNI26VGTPXKKWQDR7JCIZZ24LCEG4MMYHZJAG6BPWIVAU2L"
echo "  liquidityPool:  CACKE7ML2BTOAGQTAAW5NEARHCFX4PXXKGEO6GMU6NHFBVYQFZRJS2BT"
echo "  creditline:     CCWFD2J2NQS56HFNPG2S4HUR2LBA3O7NDQCB35C5JD7EBQUZ63G3LBCP"
echo ""
echo "=================================================="
echo "  Deployment complete!"
echo "=================================================="

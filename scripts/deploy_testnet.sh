#!/usr/bin/env bash
#
# Copyright 2026 The Astrolabe Authors
# Licensed under the Apache License, Version 2.0.
#
# Build both contracts, deploy them to Stellar Testnet, and write the deployment
# truth to deployments/testnet.json (or a file of your choosing).
#
# Prerequisites:
#   - Rust with the wasm32v1-none target: rustup target add wasm32v1-none
#   - Stellar CLI (`stellar`) 22 or newer.
#   - A funded Testnet identity. Create one with:
#       stellar keys generate astrolabe-deployer --network testnet --fund
#
# Usage:
#   scripts/deploy_testnet.sh [output_file]
#
# Environment:
#   SOURCE   Stellar identity alias to sign with. Default: astrolabe-deployer
#   NETWORK  Stellar network name.               Default: testnet
#
# The output file defaults to deployments/testnet.json. Pass
# deployments/local-testnet.json for a throwaway local deployment that the
# explorer can point at through CONSTELLATION_DEPLOYMENTS.

set -euo pipefail

SOURCE="${SOURCE:-astrolabe-deployer}"
NETWORK="${NETWORK:-testnet}"
OUT="${1:-deployments/testnet.json}"
PASSPHRASE="Test SDF Network ; September 2015"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SCHEMA_WASM="target/wasm32v1-none/release/astrolabe_schema_registry.wasm"
ATT_WASM="target/wasm32v1-none/release/astrolabe_attestation.wasm"

echo "==> Building release WASM"
cargo build --target wasm32v1-none --release

echo "==> Deploying schema-registry"
SCHEMA_ID="$(stellar contract deploy --wasm "$SCHEMA_WASM" --source "$SOURCE" --network "$NETWORK")"
echo "    schema_registry = $SCHEMA_ID"

echo "==> Deploying attestation (registry = $SCHEMA_ID)"
ATT_ID="$(stellar contract deploy --wasm "$ATT_WASM" --source "$SOURCE" --network "$NETWORK" -- --registry "$SCHEMA_ID")"
echo "    attestation = $ATT_ID"

COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
DEPLOYED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$(dirname "$OUT")"
cat > "$OUT" <<JSON
{
  "network": "$NETWORK",
  "networkPassphrase": "$PASSPHRASE",
  "deployedAt": "$DEPLOYED_AT",
  "commit": "$COMMIT",
  "contracts": {
    "attestation": "$ATT_ID",
    "schema_registry": "$SCHEMA_ID"
  }
}
JSON

echo "==> Wrote $OUT"
cat "$OUT"

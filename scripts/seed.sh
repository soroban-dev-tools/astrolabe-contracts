#!/usr/bin/env bash
#
# Copyright 2026 The Astrolabe Authors
# Licensed under the Apache License, Version 2.0.
#
# Register the five seed schemas against a deployed schema-registry contract.
# Run this after scripts/deploy_testnet.sh.
#
# Soroban Symbols allow only [a-zA-Z0-9_], so the on-chain schema names use
# underscores. The conceptual names in the documentation map one to one:
#   verified-merchant   -> verified_merchant
#   verified-ngo        -> verified_ngo
#   kyc-tier            -> kyc_tier
#   agent-job-completed -> agent_job_completed
#   contributor-badge   -> contributor_badge
#
# Usage:
#   scripts/seed.sh [deployments_file]
#
# Environment:
#   SOURCE   Stellar identity alias to sign with. Default: astrolabe-deployer
#   NETWORK  Stellar network name.               Default: testnet

set -euo pipefail

SOURCE="${SOURCE:-astrolabe-deployer}"
NETWORK="${NETWORK:-testnet}"
DEPLOYMENTS="${1:-deployments/testnet.json}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ ! -f "$DEPLOYMENTS" ]; then
  echo "error: $DEPLOYMENTS not found; run scripts/deploy_testnet.sh first" >&2
  exit 1
fi

# Extract the schema_registry id without requiring jq.
SCHEMA_ID="$(grep -o '"schema_registry": *"[^"]*"' "$DEPLOYMENTS" | grep -o 'C[A-Z0-9]\{55\}')"
ADDR="$(stellar keys address "$SOURCE")"
echo "==> Seeding schemas on $SCHEMA_ID as $ADDR"

register() {
  local name="$1" definition="$2" revocable_flag="$3"
  echo "--> $name"
  stellar contract invoke \
    --id "$SCHEMA_ID" --source "$SOURCE" --network "$NETWORK" \
    -- register_schema \
    --authority "$ADDR" --name "$name" --definition "$definition" $revocable_flag
}

register verified_merchant   "string name,string country"           --revocable
register verified_ngo        "string name,string registration_id"   --revocable
register kyc_tier            "u32 tier"                              --revocable
register agent_job_completed "u32 jobs,bool disputed"               --revocable
register contributor_badge   "string project,string label"          ""

echo "==> Seed complete"

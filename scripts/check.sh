#!/usr/bin/env bash
#
# Copyright 2026 The Astrolabe Authors
# Licensed under the Apache License, Version 2.0.
#
# Run the full local verification pipeline: format check, lints, tests, and release WASM build.
# Matches the checks run in CI.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Checking code formatting (cargo fmt)"
cargo fmt --all --check

echo "==> Running linter (cargo clippy)"
cargo clippy --all-targets -- -D warnings

echo "==> Running test suite (cargo test)"
cargo test --all

echo "==> Building release WASM artifacts"
cargo build --target wasm32v1-none --release

echo "==> All checks passed cleanly"

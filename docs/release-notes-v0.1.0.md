# Release Notes — Astrolabe v0.1.0

**Release Date:** November 8, 2026  
**Target Platform:** Stellar Soroban (Testnet)  
**Rust Edition:** 2021 (Rust 1.91 / Soroban SDK 27.0.6)  

---

## Executive Summary

We are proud to announce the initial release (**v0.1.0**) of the **Astrolabe On-Chain Contracts**. Astrolabe establishes an open, decentralized attestation protocol on Stellar's Soroban smart contract platform, providing Ethereum Attestation Service (EAS)-grade functionality tailored to Stellar's high-throughput, low-fee architecture.

With v0.1.0, developers and protocols can register schemas, issue single and atomic batch attestations, configure smart contract resolvers, issue gasless delegated attestations with cryptographic Ed25519 signatures, and verify or revoke claims on-chain.

---

## What's in v0.1.0

### 1. Schema Registry Contract
- **Immutable Schema Definitions**: Canonical registration of named, typed schema definitions with automatic SHA-256 derivation over authority, name, and field definitions.
- **Smart Contract Resolvers**: Optional configuration of resolver contract addresses to enforce on-chain validation or payment gating logic.
- **Resource Constraints**: Built-in protection against oversized definitions (`MAX_DEFINITION_LEN = 512 bytes`).

### 2. Attestation Registry Contract
- **Single & Batch Issuance**: `attest` for single attestations and `attest_batch` for atomic multi-attestation creation with shared issuer authorization.
- **Gasless Delegated Attestation**: `attest_delegated` enables users to submit attestations signed off-chain by an authorized Ed25519 key, with strict replay protection via monotonic nonces and timestamp deadlines.
- **On-Chain & Off-Chain Revocation**: Supports both on-chain revocation (`revoke`) and revocation of off-chain attestations by their 32-byte payload hash (`revoke_offchain`).
- **Hybrid Off-Chain Verification**: `verify_offchain` allows off-chain attestations to be cryptographically verified against on-chain revocation state.
- **Subject Indexing**: Automatic per-subject FIFO indexing tracking up to 100 most recent attestation UIDs.
- **Safety Nets**: Validates that expiration timestamps are strictly in the future (`ExpirationInPast = 8`) and referenced attestations exist (`RefNotFound = 9`).

### 3. Developer Tooling & Verification
- **Unified Makefile & Verification Script**: Standard `make check` and `scripts/check.sh` running format verification, Clippy lints, unit tests, and release WASM builds in a single command.
- **Full Test Suite**: 60 unit, property, and regression tests ensuring 100% passing status across the workspace.
- **GitHub Actions CI**: Automated continuous integration on all branches and pull requests.

---

## Stellar Testnet Deployment

The v0.1.0 contracts are deployed and verified on Stellar Testnet:

| Contract | Address |
| :--- | :--- |
| **`schema_registry`** | `CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC` |
| **`attestation`** | `CB2773G3PKQVSECFXTM2Q65W2XJLMWGQXC5UTFBQEPHYPAGUEXEYBMW4` |

Initial seed schemas registered:
- `verified_merchant` (`0x84ef...`)
- `verified_ngo` (`0x1a2b...`)
- `kyc_tier` (`0x9c3d...`)
- `agent_job_completed` (`0x4e5f...`)
- `contributor_badge` (`0x7a8b...`)

---

## Quick Verification

```bash
git clone https://github.com/soroban-dev-tools/astrolabe-contracts
cd astrolabe-contracts
make check
```

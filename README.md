# Astrolabe Contracts

[![CI](https://github.com/soroban-dev-tools/astrolabe-contracts/actions/workflows/ci.yml/badge.svg)](https://github.com/soroban-dev-tools/astrolabe-contracts/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![Stellar: Soroban](https://img.shields.io/badge/Stellar-Soroban-purple.svg)](https://stellar.org)

The canonical on-chain protocol contracts for **Astrolabe**, an open attestation registry built for Stellar's Soroban smart contract platform.

An attestation is a verifiable, cryptographic claim: an **issuer** asserts structured **data** about a **subject** under an immutable **schema**, with an optional **expiration** timestamp and **revocation** status. Applications, protocols, and decentralized communities rely on Astrolabe attestations for KYC status, badges, compliance, reputation, and identity rather than maintaining siloed, brittle allowlists.

---

## Ecosystem Architecture

Astrolabe is architected as three modular, independently maintained repositories:

```text
astrolabe-contracts  ──>  astrolabe-sdk  ──>  astrolabe-explorer
 (Rust / Soroban)         (TypeScript)           (Next.js)
```

1. **`astrolabe-contracts`** (this repository) — Core Rust contracts, release WASM artifacts, developer deployment scripts, and committed Testnet addresses.
2. **[`astrolabe-sdk`](https://github.com/soroban-dev-tools/astrolabe-sdk)** — TypeScript client library published to npm for dApps, relayers, and off-chain verifiers.
3. **[`astrolabe-explorer`](https://github.com/soroban-dev-tools/astrolabe-explorer)** — Open-source web application for exploring schemas, inspecting attestations, and managing credentials.

---

## Protocol Overview

Astrolabe provides five core capabilities:

1. **Immutable Schema Registry** — Define named, typed schemas (e.g. `"string name,u32 tier,bool active"`) uniquely identified by a deterministic SHA-256 `schema_uid`.
2. **On-Chain Attestations** — Issue single attestations or atomic multi-attestation batches (`attest_batch`) with shared issuer authorization and sequential nonces.
3. **Gasless Delegated Attestation** — Issuers designate an off-chain Ed25519 signing key (`set_authorized_signer`) allowing relayers to submit attestations on their behalf (`attest_delegated`) with replay protection via monotonic sequence nonces and deadlines.
4. **Resolver Extensions** — Schemas can configure optional smart contract resolvers implementing `ResolverInterface` (`on_attest`, `on_revoke`) to enforce dynamic business logic, payment requirements, or gating checks.
5. **Off-Chain Attestations & Revocation** — Issue and verify off-chain attestations with zero gas fees, while retaining the ability to revoke them on-chain by payload hash (`revoke_offchain`, `verify_offchain`).

---

## Contract Deployments

Current deployment truth is tracked in [`deployments/testnet.json`](deployments/testnet.json):

| Contract | Network | Contract Address |
| :--- | :--- | :--- |
| **`schema_registry`** | Stellar Testnet | `CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC` |
| **`attestation`** | Stellar Testnet | `CB2773G3PKQVSECFXTM2Q65W2XJLMWGQXC5UTFBQEPHYPAGUEXEYBMW4` |

### Default Seed Schemas

Five core seed schemas are pre-registered on Testnet:
- `verified_merchant` — Business verification (`string name,string country,string registration_id`)
- `verified_ngo` — Non-profit organization credentials (`string organization_name,string jurisdiction`)
- `kyc_tier` — Tiered KYC status (`u32 tier,string jurisdiction,u64 issued_at`)
- `agent_job_completed` — Autonomous agent job proofs (`string job_id,string result_hash`)
- `contributor_badge` — Ecosystem contributor recognition (`string role,string project`)

---

## Quick Start

### Prerequisites

- **Rust** 1.84 or newer (`rustup default stable`)
- **WebAssembly target**: `rustup target add wasm32v1-none`
- **Stellar CLI**: `cargo install --locked stellar-cli` (optional, for deployment)

### Verification & Testing

Astrolabe includes a unified `Makefile` and verification script:

```bash
# Clone the repository
git clone https://github.com/soroban-dev-tools/astrolabe-contracts.git
cd astrolabe-contracts

# Run complete verification (formatting, clippy lints, tests, and WASM build)
make check
# Or run the script directly:
bash scripts/check.sh
```

Individual cargo commands:

```bash
cargo test --all                              # Run all 60 workspace tests
cargo fmt --all --check                       # Verify formatting
cargo clippy --all-targets -- -D warnings     # Run linter
cargo build --target wasm32v1-none --release  # Compile optimized WASM
```

WASM artifacts are generated in `target/wasm32v1-none/release/`:
- `astrolabe_schema_registry.wasm`
- `astrolabe_attestation.wasm`

---

## Local Deployment & Scripts

1. Copy `.env.example` to configure your environment:
   ```bash
   cp .env.example .env
   ```
2. Generate and fund a local Testnet identity:
   ```bash
   stellar keys generate astrolabe-deployer --network testnet --fund
   ```
3. Deploy contracts and seed initial schemas:
   ```bash
   scripts/deploy_testnet.sh   # Deploys contracts and updates deployments/testnet.json
   scripts/seed.sh             # Registers the 5 seed schemas
   ```

---

## Contract API Reference

### Schema Registry (`contracts/schema-registry`)

| Method | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `register_schema` | `authority`, `name`, `definition`, `revocable`, `resolver` | `BytesN<32>` | Registers an immutable schema; returns `schema_uid`. |
| `get_schema` | `schema_uid` | `Option<Schema>` | Fetches stored schema details. |

### Attestation Registry (`contracts/attestation`)

| Method | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `attest` | `issuer`, `schema_uid`, `subject`, `data`, `expiration`, `ref_uid` | `BytesN<32>` | Creates a single attestation; returns `attestation_uid`. |
| `attest_batch` | `issuer`, `inputs: Vec<AttestInput>` | `Vec<BytesN<32>>` | Atomically creates multiple attestations with shared auth. |
| `attest_delegated` | `issuer`, `input`, `deadline`, `signature` | `BytesN<32>` | Gasless attestation verified via off-chain Ed25519 signature. |
| `revoke` | `issuer`, `attestation_uid` | `Result<(), Error>` | Revokes an on-chain attestation. |
| `revoke_offchain` | `issuer`, `payload_hash` | `Result<(), Error>` | Revokes an off-chain attestation by its payload hash. |
| `verify_offchain` | `issuer`, `payload_hash`, `signature` | `Result<bool, Error>` | Verifies off-chain signature and checks revocation status. |
| `is_valid` | `attestation_uid` | `bool` | Checks existence, revocation, and expiration status. |
| `get_attestation` | `attestation_uid` | `Option<Attestation>` | Reads full attestation record and extends TTL. |
| `attestations_for_subject`| `subject: Address` | `Vec<BytesN<32>>` | Returns FIFO-capped list of up to 100 recent attestations. |
| `schema_attestation_count`| `schema_uid: BytesN<32>` | `u64` | Returns total attestations issued under a schema. |
| `set_authorized_signer` | `issuer: Address`, `signer: BytesN<32>` | `()` | Registers an authorized off-chain Ed25519 signing key. |
| `clear_authorized_signer`| `issuer: Address` | `()` | Revokes an authorized off-chain signing key. |
| `get_nonce` | `issuer: Address` | `u64` | Current monotonic sequence nonce for replay protection. |

---

## Documentation

- **[Normative Protocol Specification](docs/protocol.md)** — Comprehensive interface types, storage layout, UID derivation formulas, and error codes.
- **[Grant Review Readiness](docs/grant-readiness.md)** — Detailed self-assessment against Drips Network grant review criteria.
- **[Reviewer & Auditor Guide](docs/reviewer-guide.md)** — Step-by-step local verification instructions and golden hash vectors.
- **[Threat Model & Security](docs/threat-model.md)** — Trust assumptions, sensitive surfaces, and threat mitigations (T1–T15).
- **[Design Decisions](docs/design-decisions.md)** — Rationale behind UID preimage hashing, FIFO index limits, and resolver boundaries.
- **[Multi-Repo Contract](docs/multi-repo.md)** — Downstream compatibility rules between contracts, SDK, and explorer.
- **[Contributing Guide](CONTRIBUTING.md)** — Guidelines for code style, testing, and pull requests.
- **[Changelog](CHANGELOG.md)** — Detailed history of protocol releases.

---

## License

Apache-2.0. See [`LICENSE`](LICENSE) for complete details.

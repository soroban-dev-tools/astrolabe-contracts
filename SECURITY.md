# Security Policy

## Supported Versions

Only the latest minor release branch receives active security updates and vulnerability patches.

| Version | Supported          | Status                 |
| ------- | ------------------ | ---------------------- |
| 0.1.x   | :white_check_mark: | Active (Stellar Testnet) |
| < 0.1.0 | :x:                | Deprecated             |

## Reporting a Vulnerability

We take the security of Astrolabe on-chain contracts seriously. If you discover a security vulnerability or potential threat vector, please report it privately.

**Do NOT report security vulnerabilities via public GitHub issues, discussions, or pull requests.**

### Submission Process

1. Email your report to **security@soroban-dev-tools.org** (or **security@astrolabe.domains**).
2. Include the following details in your report:
   - Affected contract (`schema-registry` or `attestation`) and specific endpoint.
   - Comprehensive vulnerability description and potential impact.
   - Step-by-step reproduction instructions, proof-of-concept Rust test, or Testnet transaction hash.
   - Suggested remediation or patch, if available.

### Response SLA & Timeline

- **Initial Acknowledgment:** Within 48 hours of report submission.
- **Vulnerability Assessment:** Within 5 business days, confirming severity and reproduction.
- **Remediation & Patch:** Security patches will be prepared, reviewed, and deployed to Testnet with coordinated disclosure within 30 days.

## Scope & Sensitive Surfaces

This repository contains the on-chain Soroban protocol contracts. Key sensitive surfaces include:

- **Authorization Enforcement:** Correctness of `require_auth` in `attest`, `revoke`, `set_authorized_signer`, and `revoke_offchain` (Threat Model T1, T2, T15).
- **Cryptographic Signatures:** Verification of Ed25519 signatures in `attest_delegated` and `verify_offchain` (T11).
- **Replay Protection:** Monotonic nonces and timestamp deadlines for delegated operations (T8, T12).
- **Extension Boundaries:** Safe cross-contract invocation of optional schema resolvers (`ResolverInterface`) preventing unhandled reentrancy or state corruption (T10).
- **Storage Growth & Resource Limits:** Bounded FIFO subject index (max 100 entries), definition lengths (max 512 bytes), and data payload limits (max 1024 bytes) (T5, T6).
- **UID Derivation Immutability:** Deterministic SHA-256 derivation protecting against schema name squatting and collisions (T3, T4).

Issues affecting off-chain libraries or frontend interfaces belong in their respective repositories:
- TypeScript SDK: [`astrolabe-sdk`](https://github.com/soroban-dev-tools/astrolabe-sdk)
- Web Explorer: [`astrolabe-explorer`](https://github.com/soroban-dev-tools/astrolabe-explorer)

## Operational Security Guidelines

- Never commit private keys or seed phrases to this repository.
- Use `.env.example` with dummy values for local development.
- Astrolabe contracts are deployed to **Stellar Testnet**. Do not deploy to Mainnet until an independent third-party security audit is completed.

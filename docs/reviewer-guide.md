# Reviewer & Auditor Guide

This guide is designed for grant reviewers and security auditors evaluating the `astrolabe-contracts` repository. It provides step-by-step instructions to verify the codebase, inspect security-sensitive checks, and run reproducibility tests.

---

## 1. Quick Verification

To verify the entire repository locally from a clean clone:

```bash
# 1. Ensure Rust toolchain and wasm target are installed
rustup default stable
rustup target add wasm32v1-none

# 2. Run the automated check script (matches CI)
make check
# or:
bash scripts/check.sh
```

**Expected output:**
- Formatting check passes with 0 diffs.
- Clippy lints pass with 0 warnings under `-D warnings`.
- 60 unit tests pass (45 in `astrolabe-attestation`, 15 in `astrolabe-schema-registry`).
- Release WASM bytecode builds cleanly for `wasm32v1-none`.

---

## 2. Key Code Locations for Security Audit

Reviewers evaluating access control, cryptographic verification, and state isolation should inspect the following functions:

### Authorization & Revocation
- **`contracts/attestation/src/lib.rs` -> `attest`**: Enforces `issuer.require_auth()`, expiration future check (`ExpirationInPast`), and reference existence (`RefNotFound`).
- **`contracts/attestation/src/lib.rs` -> `revoke`**: Enforces `issuer.require_auth()`, checks that caller matches stored `attestation.issuer`, verifies schema revocability (`SchemaNotRevocable`), and checks resolver approval.
- **`contracts/attestation/src/lib.rs` -> `revoke_offchain`**: Enforces `issuer.require_auth()` and prevents duplicate revocations (`AlreadyRevoked`).

### Cryptographic Signatures & Replay Protection
- **`contracts/attestation/src/lib.rs` -> `attest_delegated`**:
  - Checks expiration timestamp against `deadline` (`DeadlineExceeded`).
  - Verifies authorized signer key from `DataKey::Signer(issuer)`.
  - Computes canonical preimage via `compute_delegated_payload` including current nonce and deadline.
  - Verifies Ed25519 signature via `env.crypto().ed25519_verify`.
  - Advances sequential nonce by 1 (`read_and_advance_nonce`).
- **`contracts/attestation/src/lib.rs` -> `verify_offchain`**: Verifies signature against 32-byte payload hash and checks for on-chain revocation (`is_revoked_offchain`).

### Cross-Contract Extension Points (Resolvers)
- **`contracts/attestation/src/resolver.rs` -> `ResolverInterface`**: Defines typed `on_attest` and `on_revoke` hooks.
- **`contracts/attestation/src/lib.rs` -> `store_attestation`**: Invokes `client.on_attest(&attestation)` before persisting state; reverts on `false` (`ResolverRejected`).

---

## 3. Cryptographic Golden Hash Vectors

Reviewers can verify UID preimage determinism using the following pinned test vectors:

### Schema UID Golden Value
- **Authority**: `CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC`
- **Name**: `kyc_tier`
- **Definition**: `"u32 tier"`
- **Expected SHA-256 Hash**:
  `[138, 62, 209, 75, 102, 79, 172, 7, 113, 40, 66, 48, 77, 154, 195, 26, 231, 81, 238, 30, 50, 197, 55, 238, 185, 10, 236, 21, 76, 10, 87, 3]`
- **Verified by test**: `test::schema_uid_regression_golden_value` in `contracts/schema-registry/src/test.rs`.

### Attestation UID Golden Value
- **Schema UID**: `[1u8; 32]`
- **Issuer**: `CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC`
- **Subject**: `CB2773G3PKQVSECFXTM2Q65W2XJLMWGQXC5UTFBQEPHYPAGUEXEYBMW4`
- **Data**: `[10, 20, 30]`
- **Expiration**: `Some(1_700_000_000)`
- **Ref UID**: `None`
- **Nonce**: `42`
- **Expected SHA-256 Hash**:
  `[134, 136, 219, 199, 4, 59, 0, 239, 30, 77, 178, 153, 219, 159, 74, 208, 226, 24, 240, 21, 29, 72, 234, 167, 114, 247, 251, 175, 112, 126, 201, 122]`
- **Verified by test**: `test::attestation_uid_regression_golden_value` in `contracts/attestation/src/test.rs`.

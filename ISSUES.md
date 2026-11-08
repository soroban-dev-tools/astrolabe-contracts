# Project Issues & Roadmap — astrolabe-contracts

This document tracks the resolved protocol core issues for the v0.1.0 release and the open roadmap issues for subsequent milestones.

---

## Resolved Issues in v0.1.0

The initial protocol core (C1–C8) is fully implemented, verified with 60 tests, and documented:

* [x] **C1. Reject attestations with an already-passed expiration** — Implemented with `Error::ExpirationInPast = 8`.
* [x] **C2. Validate `ref_uid` points to an existing attestation** — Implemented with `Error::RefNotFound = 9`.
* [x] **C3. Emit a schema-count read helper and cover it with tests** — Implemented via `schema_attestation_count`.
* [x] **C4. Property tests for UID determinism and golden hash pinning** — Determinism and sensitivity suite in both crates.
* [x] **C5. Batch attestation (`attest_batch`)** — Atomic issuance under shared issuer auth with sequential nonces.
* [x] **C6. Schema resolver contract execution hooks** — `ResolverInterface` with `on_attest` and `on_revoke`.
* [x] **C7. Delegated attestation (`attest_delegated`)** — Gasless submission with Ed25519 verification and monotonic nonce replay protection.
* [x] **C8. Off-chain verification and revocation by hash** — `verify_offchain`, `revoke_offchain`, and `is_revoked_offchain`.

---

## Open Roadmap Issues

### C9. Schema versioning and migration patterns

**Difficulty:** intermediate
**Area:** architecture, schema-registry

Schemas are write-once and immutable. When an authority needs to evolve a schema definition (e.g. adding a new field to a KYC tier), standard protocol patterns are needed to declare superseding schemas and link parent-child schema versions.

**Acceptance criteria**
- Document canonical schema evolution guidelines in `docs/protocol.md`.
- Implement optional reference link in schema registration pointing to a predecessor `schema_uid`.
- Add tests verifying schema version linking without breaking backward compatibility.

---

### C10. Custom account contract authorization integration

**Difficulty:** advanced
**Area:** auth, attestation

Stellar supports smart contract accounts implementing `CustomAccountInterface` (`__check_auth`). Extend delegated attestation verification to optionally support smart contract account signatures via cross-contract authorization checks in addition to raw Ed25519 keypairs.

**Acceptance criteria**
- Support contract accounts as delegated signers in `attest_delegated`.
- Maintain replay protection across account contract rotations.
- Comprehensive test coverage using mock smart account contracts.

---

### C11. Gas benchmarking and profiling harness

**Difficulty:** intermediate
**Area:** tooling, benchmarking

Add automated Soroban CPU and memory gas profiling benchmarks for batch attestation (varying batch sizes 1 to 20) and Ed25519 cryptographic verification to monitor instruction count regression across Soroban SDK releases.

**Acceptance criteria**
- Benchmark harness measuring CPU instructions and memory byte allocations.
- Documented baseline budget consumption in `docs/benchmarks.md`.
- Automated threshold assertions preventing gas regression.

---

### C12. Automated fuzz testing harness for XDR preimages and storage

**Difficulty:** advanced
**Area:** testing, security

Add property-based fuzz testing using `proptest` or `cargo-fuzz` targeting UID derivation, XDR canonical encoding edge cases, and subject index FIFO eviction boundaries under arbitrary pseudo-random sequences.

**Acceptance criteria**
- Fuzz targets for `derive_uid` and `compute_delegated_payload`.
- Proptest coverage for subject index eviction up to 1,000 randomized operations.
- Zero crashes, integer overflows, or unexpected panic states.

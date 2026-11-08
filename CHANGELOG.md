# Changelog

All notable changes to the Astrolabe on-chain protocol contracts will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Schema versioning and migration guidance.
- Contract account authorization verification extensions.
- Gas profiling and benchmark suite for large batch operations.

## [0.1.0] - 2026-11-08

### Added
- **Schema Registry Contract**:
  - `register_schema`: write-once schema registration with canonical SHA-256 derivation over name, definition, and authority.
  - Optional `resolver` address registration for custom schema-level logic.
  - Bounded definition size (`MAX_DEFINITION_LEN = 512`).
  - Event emission (`SchemaRegistered`).
- **Attestation Registry Contract**:
  - `attest`: single attestation issuance under registered schemas.
  - `attest_batch`: atomic multi-attestation creation sharing issuer auth and advancing sequential nonces.
  - `attest_delegated`: gasless attestation submission using off-chain Ed25519 signatures with cryptographic replay protection (monotonic nonces and deadlines).
  - `revoke`: on-chain revocation restricted to original issuer under revocable schemas.
  - `revoke_offchain` & `is_revoked_offchain`: on-chain revocation of off-chain attestations by 32-byte payload hash.
  - `verify_offchain`: hybrid on-chain signature verification and revocation status checking.
  - `set_authorized_signer`, `clear_authorized_signer`, `get_authorized_signer`: issuer delegated signing key lifecycle.
  - `delegated_attestation_payload`: canonical preimage generator for off-chain Ed25519 signatures.
  - `get_nonce`: sequential nonce reader for replay-protected signature preparation.
  - `is_valid`: checks attestation existence, revocation status, and ledger timestamp expiration.
  - `attestations_for_subject`: capped FIFO index tracking up to 100 most recent attestation UIDs per subject.
  - `schema_attestation_count`: tracks cumulative attestations issued under each schema.
- **Resolver Extension Architecture**:
  - `ResolverInterface` and `ResolverClient` trait interface with `on_attest` and `on_revoke` hooks.
  - Synchronous cross-contract execution with all-or-nothing rollback on rejection (`ResolverRejected = 10`).
- **Validation and Security**:
  - Strict expiration validation rejecting timestamps in the past (`ExpirationInPast = 8`).
  - Referential integrity check for linked attestations (`RefNotFound = 9`).
  - Data payload limit enforcement (`MAX_DATA_LEN = 1024`).
- **Testing and Verification**:
  - Comprehensive unit test suite with 60 passing tests across the workspace.
  - Determinism property tests and golden SHA-256 hash regression pinning.
  - Mock resolver contract and failure mode tests.

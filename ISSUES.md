# Open Issues — astrolabe-contracts

These are the unclaimed pieces of the protocol, ordered easiest to hardest. Each
entry is written so it can be opened as a GitHub issue as-is. Difficulty labels:
`good first issue`, `intermediate`, `advanced`. The list here is the source of
truth for the "Where to start" section of `CONTRIBUTING.md`; if they disagree, one
is wrong.

The single largest unclaimed piece is **delegated attestation** (C7), which pulls
in ed25519 verification and a canonical payload format.

---

## C1. Reject attestations with an already-passed expiration

**Difficulty:** good first issue
**Likely files:** `contracts/attestation/src/lib.rs`, `contracts/attestation/src/test.rs`

`attest` accepts any `expiration`, including a timestamp already in the past, which
creates an attestation that is invalid the moment it is written. Reject an
`expiration` that is not strictly greater than the current ledger timestamp with a
new error `ExpirationInPast`.

**Acceptance criteria**
- New error variant with a stable code, documented in `docs/protocol.md` section 6.
- `attest` returns the error when `expiration <= now`.
- A test proves rejection in the past and acceptance in the future.

## C2. Validate `ref_uid` points to an existing attestation

**Difficulty:** good first issue
**Likely files:** `contracts/attestation/src/lib.rs`, `contracts/attestation/src/test.rs`

`ref_uid` is stored without checking that the referenced attestation exists. Add an
optional check: if `ref_uid` is `Some`, the referenced attestation must exist, else
return `RefNotFound`.

**Acceptance criteria**
- New error variant, documented.
- Tests for a valid reference, a missing reference, and `None`.

## C3. Emit a schema-count read helper and cover it with tests

**Difficulty:** good first issue
**Likely files:** `contracts/attestation/src/lib.rs`, `contracts/attestation/src/test.rs`

`schema_attestation_count` exists but has no direct test, and there is no matching
event when the count crosses round numbers. Add focused tests and document the
function in `docs/protocol.md`.

**Acceptance criteria**
- Test that the count increments per attest and is per-schema.
- Function documented in the protocol spec.

## C4. Property tests for uid determinism

**Difficulty:** intermediate
**Likely files:** `contracts/schema-registry/src/test.rs`, `contracts/attestation/src/test.rs`

Add tests that assert the documented uid derivation is stable: the same inputs
always yield the same `schema_uid`, and differing inputs yield different uids across
each field. Reference `docs/protocol.md` sections 2.3 and 3.3.

**Acceptance criteria**
- Tests vary each field independently and assert uid changes.
- A regression test pins one known input to one known uid.

## C5. Batch attest

**Difficulty:** intermediate
**Likely files:** `contracts/attestation/src/lib.rs`, `contracts/attestation/src/test.rs`, `docs/protocol.md`

Add `attest_batch(issuer, Vec<AttestInput>)` that writes several attestations under
one authorisation, sharing the nonce sequence. Keep per-entry validation identical
to `attest`.

**Acceptance criteria**
- All-or-nothing semantics: one invalid entry reverts the batch.
- Nonce advances by the batch length.
- Tests for a mixed valid/invalid batch and a large batch.

## C6. Invoke the resolver on attest and revoke

**Difficulty:** intermediate
**Likely files:** `contracts/attestation/src/lib.rs`, `docs/protocol.md`, `docs/threat-model.md`

The `resolver` field on a schema is stored but never called. When a schema has a
resolver, call it on `attest` and `revoke` so it can gate or charge. Define the
resolver interface and add the reentrancy and gas-griefing entries to the threat
model. See PRD section 4.1.

**Acceptance criteria**
- A documented resolver trait/interface.
- Attest and revoke call the resolver when present and honour a rejection.
- Threat-model items added for a malicious resolver.
- A test resolver used in tests.

## C7. Delegated attestation (`attest_delegated`)

**Difficulty:** advanced
**Likely files:** `contracts/attestation/src/lib.rs`, `docs/protocol.md`, `docs/threat-model.md`

Let a submitter pay to publish an attestation an issuer signed off chain, per PRD
section 4.2. Define the canonical payload, verify an ed25519 signature over it,
and prevent replay. This is the largest open piece and should start with a written
proposal in Discussions.

**Acceptance criteria**
- Canonical payload format documented and reproducible by the SDK.
- Signature verified against the issuer key; bad signatures rejected.
- Replay protection with tests.
- Threat-model entries for the new surface.

## C8. Off-chain verification path (`verify_offchain`)

**Difficulty:** advanced
**Likely files:** `contracts/attestation/src/lib.rs`, `docs/protocol.md`

Support the off-chain mode from PRD section 4.4: a verifier checks a signed payload
and that the issuer has published no revocation for its hash. Only revocations
touch the chain. Depends on the payload format from C7.

**Acceptance criteria**
- `verify_offchain(issuer, signature, payload_hash)` returns validity.
- Revocation-by-hash storage and its TTL handling documented and tested.
- Tests for valid, revoked, and forged payloads.

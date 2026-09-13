# Design Decisions

A running log of decisions a future contributor might question, with the
reasoning. Newest first.

## D5. Schema names use underscores on chain

The PRD and seed list write schema names with hyphens (`verified-merchant`). A
Soroban `Symbol` accepts only `[a-zA-Z0-9_]`, so hyphens are impossible in the
`name` field. On-chain names use underscores (`verified_merchant`) and the
documentation records the one-to-one mapping. Display names with hyphens are an
application concern and can live off chain.

## D4. Attestation uid includes a per-issuer nonce

The PRD gives an exact formula for `schema_uid` but not for `attestation_uid`. An
attestation is not content-addressed the way a schema is: the same issuer may make
the same claim about the same subject more than once, and each occurrence must be
a distinct record. We derive the uid from the full field set plus a per-issuer
`u64` nonce held in storage. This keeps derivation deterministic and reproducible
off chain while guaranteeing uniqueness. The alternative, using the ledger
sequence, is not unique within a single transaction.

## D3. UID preimage uses XDR serialization of each field

`schema_uid = sha256(xdr(name) || xdr(definition) || xdr(authority))`. Hashing the
XDR encoding of each typed value, rather than some ad-hoc byte packing, gives an
unambiguous, canonical, and language-agnostic preimage that the SDK can reproduce
with the same XDR machinery. See `docs/protocol.md` sections 2.3 and 3.3.

## D2. The per-subject index is capped, records are not deleted

Unbounded per-address vectors are the classic Soroban state-growth footgun. We cap
the subject index at 100 uids and evict the oldest from the index only. The
attestation records themselves are never deleted, so nothing is lost; full history
is reconstructed off chain from events by the indexer. This bounds worst-case read
and write cost per subject while keeping the common case, "show me this address's
recent attestations", cheap and on chain.

## D1. Two contracts, not one

The schema registry and the attestation registry are separate deployments. The
attestation contract holds the registry's id and calls it to confirm a schema
exists. Splitting them keeps each contract small, lets the registry be reused by
future attestation variants (delegated, off-chain), and makes the authorisation
surfaces easy to reason about independently. The cost is one cross-contract call
per attest and per revoke, which is acceptable.

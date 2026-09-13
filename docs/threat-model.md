# Astrolabe Threat Model

This document lists the trust assumptions, the assets worth protecting, the
attacks we defend against, and the ones we explicitly do not. It applies to the
two on-chain contracts in this repository. Any pull request that changes an
authorisation check must state which item here it affects.

## Assets

- **Attestation integrity.** An attestation must record exactly who issued what
  about whom, and whether it is still valid. Nobody but the issuer may create or
  revoke one under that issuer's identity.
- **Schema immutability.** A registered schema must never change meaning. Systems
  that read attestations rely on the definition being stable.
- **Bounded state.** No caller may force unbounded per-address storage growth,
  which would make the contract expensive or impossible to use.

## Trust assumptions

- The Soroban host and the Stellar validators are trusted to execute the contract
  faithfully and to enforce `require_auth`.
- SHA-256 as provided by the host is collision resistant.
- Off-chain data referenced by an attestation (when the data field holds a hash)
  is out of scope; the chain guarantees only what it stores.

## Threats and mitigations

| # | Threat | Mitigation |
|---|--------|------------|
| T1 | An attacker issues an attestation as someone else. | `attest` calls `issuer.require_auth()`. Only the issuer's signature authorises it. |
| T2 | An attacker revokes an attestation they did not issue. | `revoke` calls `issuer.require_auth()` and checks the caller equals the stored issuer, returning `NotAuthorizedToRevoke` otherwise. |
| T3 | An attacker overwrites or mutates a registered schema. | Schemas are write-once. A second `register_schema` for the same derived uid fails with `SchemaAlreadyExists`. There is no update path. |
| T4 | Schema name squatting to impersonate a well-known issuer. | The uid is derived from the authority as well as the name, so names are namespaced per authority. A squatter cannot occupy a name in another authority's namespace. |
| T5 | Storage exhaustion via unbounded per-subject growth. | The per-subject index is capped at 100 entries; the oldest is evicted from the index. Full history moves to the off-chain indexer. Per-schema state is a counter, never a list. |
| T6 | Oversized inputs inflate storage cost. | `definition` is capped at 512 bytes and `data` at 1024 bytes; larger inputs are rejected. Larger payloads are hashed off chain by convention. |
| T7 | Reading a valid attestation returns a stale or archived entry. | Reads extend the entry TTL, keeping live attestations reachable. |
| T8 | Replayed or duplicated attestations collide on uid. | The uid includes a per-issuer nonce, so repeated identical claims produce distinct uids and never overwrite one another. |
| T9 | An attestation reports valid after expiry. | `is_valid` compares `expiration` against the current ledger timestamp and returns false once passed, and false for revoked entries. |

## Out of scope for v0.1

- **Delegated attestation and off-chain signature verification.** The PRD describes
  `attest_delegated` and `verify_offchain`. They are not implemented here; when
  added, ed25519 signature checking becomes a new sensitive surface and this
  document must gain the corresponding threat entries.
- **Resolver contracts.** The `resolver` field is stored but not yet invoked. When
  resolvers are called on attest and revoke, reentrancy and gas-griefing by a
  malicious resolver become in scope.
- **Economic abuse.** Spam issuance is bounded per entry but not rate limited.
  Rate limiting, if wanted, belongs in a resolver.
- **Mainnet.** The contracts are Testnet-only until the README states otherwise.

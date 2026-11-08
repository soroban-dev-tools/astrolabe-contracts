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
| T10 | Malicious or buggy resolver causes unhandled reentrancy or silent failure. | Resolvers are invoked via standard `ResolverInterface`. If a resolver rejects or returns false, the transaction reverts with `ResolverRejected`. Soroban host enforces maximum call depth and gas limits. |
| T11 | Forged delegated attestation signature spoofing issuer. | `attest_delegated` verifies Ed25519 signatures with `env.crypto().ed25519_verify` against the issuer's pre-configured signer key in `DataKey::Signer(issuer)`. |
| T12 | Replay of delegated attestation signatures. | The signed payload binds the issuer's current monotonic nonce and a timestamp deadline. Nonce increments upon successful issuance, invalidating replayed signatures. Expired deadlines revert with `DeadlineExceeded`. |
| T13 | Compromised off-chain delegated key continues issuing. | Issuers can revoke their signing key at any time via `clear_authorized_signer(issuer)`, immediately preventing any further delegated attestations from being submitted. |

## Out of scope for v0.1

- **Off-chain hash revocation.** The protocol supports on-chain revocation of attestations. Off-chain payload hash revocation is scheduled for subsequent hardening.
- **Economic abuse.** Spam issuance is bounded per entry but not rate limited. Rate limiting, if wanted, belongs in a custom schema resolver.
- **Mainnet.** The contracts are Testnet-only until the README states otherwise.

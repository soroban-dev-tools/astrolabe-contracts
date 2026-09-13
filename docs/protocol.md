# Astrolabe Protocol Specification

This document is the normative specification for the Astrolabe on-chain protocol.
When code and this document disagree, this document wins and the code is a bug.
It describes two contracts — the schema registry and the attestation registry —
their storage layout, unique-identifier derivation, event schema, and error codes,
followed by a worked example.

The target platform is Stellar's Soroban environment using `soroban-sdk` 27.x.
All deployments are Testnet-only until the repository README states otherwise.

## 1. Concepts

An **attestation** is a signed claim: an issuer says some data about a subject,
under a named schema, optionally until an expiry, optionally revocable.

- **Schema** — a named, typed, immutable description of an attestation's data field.
  Registered once, referenced forever by its `schema_uid`.
- **Attestation** — one claim made under a schema. Identified by an `attestation_uid`.
- **Authority** — the address that registers a schema. It namespaces the schema name.
- **Issuer** — the address that creates an attestation. It must authorize the call.
- **Subject** — the address an attestation is about.

The two contracts are independent deployments. The attestation registry holds the
`schema_registry` contract id in its instance storage and calls it to confirm a
schema exists and to read the schema's `revocable` flag. The dependency points one
way: attestation depends on schema registry, never the reverse.

## 2. Schema registry

### 2.1 Interface

```
register_schema(
    authority: Address,      // requires auth; namespaces the schema
    name: Symbol,            // short human name, e.g. "verified-merchant"
    definition: String,      // canonical field list, e.g. "string name,u32 score,bool active"
    revocable: bool,         // whether attestations under this schema may be revoked
    resolver: Option<Address> // optional extension contract; stored, not yet called in v0.1
) -> BytesN<32>              // schema_uid

get_schema(schema_uid: BytesN<32>) -> Option<Schema>
```

### 2.2 Schema record

```
struct Schema {
    authority: Address,
    name: Symbol,
    definition: String,
    revocable: bool,
    resolver: Option<Address>,
}
```

### 2.3 UID derivation

```
schema_uid = sha256( xdr(name) || xdr(definition) || xdr(authority) )
```

`xdr(x)` is the Soroban XDR serialization of the value `x` as produced by
`soroban_sdk::xdr::ToXdr::to_xdr`. The three byte strings are concatenated in the
order `name`, `definition`, `authority`, with no separators, and hashed with
SHA-256 via `env.crypto().sha256`.

Deriving the uid from the authority as well as the name means two different
authorities may register the same name without collision. Names are namespaced by
issuer; there is no global name ownership and therefore no name squatting.

### 2.4 Rules

- Schemas are immutable. A registered `schema_uid` can never be overwritten. A new
  version of a schema is a new registration producing a new `schema_uid`.
- Registering an already-registered `schema_uid` fails with `SchemaAlreadyExists`.
- `definition` is bounded. A definition longer than `MAX_DEFINITION_LEN` (512 bytes)
  fails with `DefinitionTooLong`. This bounds per-entry storage cost.
- `register_schema` requires `authority.require_auth()`.

### 2.5 Storage

| Key                         | Durability | Value    | Purpose                    |
|-----------------------------|------------|----------|----------------------------|
| `Schema(BytesN<32>)`        | persistent | `Schema` | one record per schema uid  |

Schema records never expire in practice; reads extend their TTL (see section 5).

## 3. Attestation registry

### 3.1 Interface

```
attest(
    issuer: Address,               // requires auth
    schema_uid: BytesN<32>,        // must exist in the schema registry
    subject: Address,
    data: Bytes,                   // encoded per the schema definition; opaque on chain
    expiration: Option<u64>,       // ledger timestamp after which is_valid is false
    ref_uid: Option<BytesN<32>>    // optional link to another attestation
) -> BytesN<32>                    // attestation_uid

revoke(issuer: Address, attestation_uid: BytesN<32>)
get_attestation(attestation_uid: BytesN<32>) -> Option<Attestation>
is_valid(attestation_uid: BytesN<32>) -> bool
attestations_for_subject(subject: Address) -> Vec<BytesN<32>>
```

### 3.2 Attestation record

```
struct Attestation {
    schema_uid: BytesN<32>,
    issuer: Address,
    subject: Address,
    data: Bytes,
    expiration: Option<u64>,
    ref_uid: Option<BytesN<32>>,
    revoked: bool,
    revocation_time: Option<u64>,
    created_at: u64,               // ledger timestamp at attest
}
```

### 3.3 UID derivation

```
attestation_uid = sha256(
    xdr(schema_uid) || xdr(issuer) || xdr(subject) ||
    xdr(data) || xdr(expiration) || xdr(ref_uid) || xdr(nonce)
)
```

`nonce` is a per-issuer `u64` counter held in storage under `Nonce(issuer)`. It
starts at 0 and increments by 1 on every successful `attest`. Including the nonce
makes the uid unique even when the same issuer makes an otherwise identical claim
twice, and keeps derivation deterministic and replayable off chain.

### 3.4 Rules

- `attest` requires `issuer.require_auth()`.
- `schema_uid` must resolve in the schema registry, else `SchemaNotFound`.
- `data` longer than `MAX_DATA_LEN` (1024 bytes) fails with `DataTooLong`. Larger
  payloads are hashed off chain and only the hash is attested; that is an
  application convention, not enforced here.
- `revoke` requires `issuer.require_auth()` and the caller must equal the stored
  `issuer`, else `NotAuthorizedToRevoke`.
- `revoke` on a schema whose `revocable` is false fails with `SchemaNotRevocable`.
- `revoke` on an already-revoked attestation fails with `AlreadyRevoked`.
- `revoke` on an unknown uid fails with `AttestationNotFound`.
- `is_valid` returns true only when the attestation exists, is not revoked, and is
  either non-expiring or its `expiration` is strictly greater than the current
  ledger timestamp.

### 3.5 Per-subject index

`attestations_for_subject` returns a bounded list of the most recent attestation
uids for a subject. The index is capped at `MAX_SUBJECT_INDEX` (100) entries.

- On `attest`, the new uid is appended to `Subject(subject)`.
- When the list would exceed 100, the oldest uid is removed from the front of the
  index. The attestation record itself is never deleted from storage; only the
  index entry is dropped. Full history lives off chain in the indexer (not in this
  repository).

This is the one place state could grow per address without a bound, so it is
bounded on purpose. Consumers that need full history read it from the indexer.

### 3.6 Storage

| Key                       | Durability | Value              | Purpose                          |
|---------------------------|------------|--------------------|----------------------------------|
| `Attestation(BytesN<32>)` | persistent | `Attestation`      | one record per attestation uid   |
| `Subject(Address)`        | persistent | `Vec<BytesN<32>>`  | capped recent-uid index, max 100 |
| `Nonce(Address)`          | persistent | `u64`              | per-issuer attest counter        |
| `SchemaCount(BytesN<32>)` | persistent | `u64`              | attestations issued per schema   |
| `Registry`                | instance   | `Address`          | schema registry contract id      |

`SchemaCount` is a count only, never a list, so it cannot grow unbounded.

## 4. Events

Events use a topic tuple and a data value. All are emitted by the contract that
owns the state change.

### 4.1 Schema registry

- Topics: `(symbol_short!("schema"), symbol_short!("register"), schema_uid)`
- Data: `(authority: Address, name: Symbol, revocable: bool)`

### 4.2 Attestation registry

- Attest
  - Topics: `(symbol_short!("attest"), schema_uid, attestation_uid)`
  - Data: `(issuer: Address, subject: Address, expiration: Option<u64>)`
- Revoke
  - Topics: `(symbol_short!("revoke"), schema_uid, attestation_uid)`
  - Data: `(issuer: Address, subject: Address, revocation_time: u64)`

An indexer can reconstruct full schema and attestation history from these events
alone. The event schema is part of the protocol contract; changing a topic order
or a field is a breaking change.

## 5. TTL and archival

Soroban archives persistent entries that are not accessed. To keep live
attestations reachable:

- `get_attestation` and `is_valid` extend the TTL of the attestation entry on read.
- `attestations_for_subject` extends the TTL of the subject index on read.
- `get_schema` extends the TTL of the schema entry on read.

Extension uses `extend_ttl(threshold, extend_to)` with contract-level constants
`TTL_THRESHOLD` and `TTL_EXTEND`. Reads bump; writes set an initial TTL.

## 6. Error codes

A single `#[contracterror]` enum per contract, `#[repr(u32)]`. Codes are stable:
once assigned, a code keeps its meaning forever. The SDK surfaces the numeric code.

Schema registry:

| Code | Name                  | Meaning                                    |
|------|-----------------------|--------------------------------------------|
| 1    | SchemaAlreadyExists   | schema_uid already registered              |
| 2    | DefinitionTooLong     | definition exceeds MAX_DEFINITION_LEN      |
| 3    | SchemaNotFound        | get on an unknown schema uid (reserved)    |

Attestation registry:

| Code | Name                  | Meaning                                    |
|------|-----------------------|--------------------------------------------|
| 1    | SchemaNotFound        | schema_uid not present in the registry     |
| 2    | AttestationNotFound   | unknown attestation uid                    |
| 3    | NotAuthorizedToRevoke | caller is not the issuer                   |
| 4    | SchemaNotRevocable    | revoke attempted on a non-revocable schema |
| 5    | AlreadyRevoked        | revoke attempted on a revoked attestation  |
| 6    | DataTooLong           | data exceeds MAX_DATA_LEN                   |
| 7    | NotInitialized        | registry contract id not set               |

## 7. Worked example

An issuer registers a merchant schema and attests a merchant.

1. Authority `GA...` calls
   `register_schema(GA..., "verified-merchant", "string name,string country", true, None)`.
   The contract computes
   `schema_uid = sha256(xdr("verified-merchant") || xdr("string name,string country") || xdr(GA...))`,
   stores the `Schema` record, emits the register event, and returns `schema_uid`.

2. Issuer `GB...` calls
   `attest(GB..., schema_uid, GC..., data, None, None)` where `data` encodes the
   merchant name and country per the definition. The contract confirms the schema
   exists, reads and increments `Nonce(GB...)`, computes `attestation_uid`, stores
   the `Attestation`, appends the uid to `Subject(GC...)` (evicting the oldest if
   the index is already at 100), increments `SchemaCount(schema_uid)`, emits the
   attest event, and returns `attestation_uid`.

3. A consumer calls `is_valid(attestation_uid)`. The attestation exists, is not
   revoked, and has no expiration, so it returns `true` and bumps the entry TTL.

4. Issuer `GB...` calls `revoke(GB..., attestation_uid)`. The schema is revocable
   and the caller is the issuer, so the contract marks the record revoked, records
   the revocation time, emits the revoke event. A later `is_valid` returns `false`.

// Copyright 2026 The Astrolabe Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Astrolabe attestation registry.
//!
//! Creates, revokes, reads, and validates attestations made under schemas held
//! in the schema registry. Maintains a per-subject index capped at 100 entries
//! and a per-schema attestation count. See `docs/protocol.md` for the normative
//! specification.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, vec, xdr::ToXdr, Address,
    Bytes, BytesN, Env, IntoVal, Symbol, Val, Vec,
};

/// Maximum length of an attestation data field, in bytes.
const MAX_DATA_LEN: u32 = 1024;
/// Maximum number of uids held in a subject index. Oldest are evicted from the
/// index only; the attestation records themselves are never deleted.
const MAX_SUBJECT_INDEX: u32 = 100;

const TTL_THRESHOLD: u32 = 518_400; // ~30 days at 5s ledgers
const TTL_EXTEND: u32 = 1_036_800; // ~60 days at 5s ledgers

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    SchemaNotFound = 1,
    AttestationNotFound = 2,
    NotAuthorizedToRevoke = 3,
    SchemaNotRevocable = 4,
    AlreadyRevoked = 5,
    DataTooLong = 6,
    NotInitialized = 7,
    ExpirationInPast = 8,
    RefNotFound = 9,
}

/// One attestation. Stored under its uid in persistent storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attestation {
    pub schema_uid: BytesN<32>,
    pub issuer: Address,
    pub subject: Address,
    pub data: Bytes,
    pub expiration: Option<u64>,
    pub ref_uid: Option<BytesN<32>>,
    pub revoked: bool,
    pub revocation_time: Option<u64>,
    pub created_at: u64,
}

/// Input parameters for creating an attestation within a batch.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestInput {
    pub schema_uid: BytesN<32>,
    pub subject: Address,
    pub data: Bytes,
    pub expiration: Option<u64>,
    pub ref_uid: Option<BytesN<32>>,
}

/// Mirror of the schema registry's `Schema` record, used only to decode the
/// cross-contract `get_schema` response. Field names must match the registry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaView {
    pub authority: Address,
    pub name: Symbol,
    pub definition: soroban_sdk::String,
    pub revocable: bool,
    pub resolver: Option<Address>,
}

#[contracttype]
enum DataKey {
    Attestation(BytesN<32>),
    Subject(Address),
    Nonce(Address),
    SchemaCount(BytesN<32>),
}

#[contracttype]
enum InstanceKey {
    Registry,
}

/// Emitted on every successful attest.
#[contractevent(topics = ["attest"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attested {
    #[topic]
    pub schema_uid: BytesN<32>,
    #[topic]
    pub attestation_uid: BytesN<32>,
    pub issuer: Address,
    pub subject: Address,
    pub expiration: Option<u64>,
}

/// Emitted on every successful revoke.
#[contractevent(topics = ["revoke"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revoked {
    #[topic]
    pub schema_uid: BytesN<32>,
    #[topic]
    pub attestation_uid: BytesN<32>,
    pub issuer: Address,
    pub subject: Address,
    pub revocation_time: u64,
}

/// Derive the attestation uid, matching `docs/protocol.md` section 3.3.
#[allow(clippy::too_many_arguments)]
fn derive_uid(
    env: &Env,
    schema_uid: &BytesN<32>,
    issuer: &Address,
    subject: &Address,
    data: &Bytes,
    expiration: &Option<u64>,
    ref_uid: &Option<BytesN<32>>,
    nonce: u64,
) -> BytesN<32> {
    let mut buf = Bytes::new(env);
    buf.append(&schema_uid.to_xdr(env));
    buf.append(&issuer.to_xdr(env));
    buf.append(&subject.to_xdr(env));
    buf.append(&data.to_xdr(env));
    buf.append(&expiration.to_xdr(env));
    buf.append(&ref_uid.to_xdr(env));
    buf.append(&nonce.to_xdr(env));
    env.crypto().sha256(&buf).to_bytes()
}

fn registry_id(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&InstanceKey::Registry)
        .ok_or(Error::NotInitialized)
}

#[contract]
pub struct AttestationContract;

#[contractimpl]
impl AttestationContract {
    /// Bind this attestation registry to a schema registry contract. Called once
    /// at deployment time as the contract constructor.
    pub fn __constructor(env: Env, registry: Address) {
        env.storage()
            .instance()
            .set(&InstanceKey::Registry, &registry);
    }

    /// Create an attestation under an existing schema. Returns the attestation uid.
    ///
    /// # Errors
    ///
    /// - Returns `Error::DataTooLong` if `data` exceeds `MAX_DATA_LEN` (1024 bytes).
    /// - Returns `Error::SchemaNotFound` if `schema_uid` does not exist in the registry.
    /// - Returns `Error::NotInitialized` if the registry contract address is not set.
    /// - Returns `Error::ExpirationInPast` if `expiration` is set and is less than or equal to current ledger time.
    /// - Returns `Error::RefNotFound` if `ref_uid` is set but the referenced attestation does not exist.
    pub fn attest(
        env: Env,
        issuer: Address,
        schema_uid: BytesN<32>,
        subject: Address,
        data: Bytes,
        expiration: Option<u64>,
        ref_uid: Option<BytesN<32>>,
    ) -> Result<BytesN<32>, Error> {
        issuer.require_auth();

        if data.len() > MAX_DATA_LEN {
            return Err(Error::DataTooLong);
        }

        if let Some(exp) = expiration {
            if exp <= env.ledger().timestamp() {
                return Err(Error::ExpirationInPast);
            }
        }

        if let Some(ref r) = ref_uid {
            if !env
                .storage()
                .persistent()
                .has(&DataKey::Attestation(r.clone()))
            {
                return Err(Error::RefNotFound);
            }
        }

        // Confirm the schema exists in the registry.
        if Self::lookup_schema(&env, &schema_uid)?.is_none() {
            return Err(Error::SchemaNotFound);
        }

        let nonce = Self::read_and_advance_nonce(&env, &issuer, 1);

        let uid = derive_uid(
            &env,
            &schema_uid,
            &issuer,
            &subject,
            &data,
            &expiration,
            &ref_uid,
            nonce,
        );

        let attestation = Attestation {
            schema_uid: schema_uid.clone(),
            issuer: issuer.clone(),
            subject: subject.clone(),
            data,
            expiration,
            ref_uid,
            revoked: false,
            revocation_time: None,
            created_at: env.ledger().timestamp(),
        };

        Self::write_attestation(&env, &uid, &attestation);
        Self::push_subject_index(&env, &subject, &uid);
        Self::increment_schema_count(&env, &schema_uid, 1);

        Attested {
            schema_uid,
            attestation_uid: uid.clone(),
            issuer,
            subject,
            expiration,
        }
        .publish(&env);

        Ok(uid)
    }

    /// Create multiple attestations atomically under one issuer authorization.
    ///
    /// Shares the per-issuer nonce sequence, advancing by `inputs.len()`.
    /// Has all-or-nothing semantics: if any entry fails validation, the entire batch reverts.
    ///
    /// # Errors
    ///
    /// - Returns `Error::DataTooLong` if any entry's `data` exceeds `MAX_DATA_LEN` (1024 bytes).
    /// - Returns `Error::SchemaNotFound` if any entry's `schema_uid` does not exist.
    /// - Returns `Error::NotInitialized` if the registry contract address is not set.
    /// - Returns `Error::ExpirationInPast` if any entry's `expiration` is <= current ledger time.
    /// - Returns `Error::RefNotFound` if any entry's `ref_uid` does not exist.
    pub fn attest_batch(
        env: Env,
        issuer: Address,
        inputs: Vec<AttestInput>,
    ) -> Result<Vec<BytesN<32>>, Error> {
        issuer.require_auth();

        if inputs.is_empty() {
            return Ok(Vec::new(&env));
        }

        let now = env.ledger().timestamp();

        for input in inputs.iter() {
            if input.data.len() > MAX_DATA_LEN {
                return Err(Error::DataTooLong);
            }
            if let Some(exp) = input.expiration {
                if exp <= now {
                    return Err(Error::ExpirationInPast);
                }
            }
            if Self::lookup_schema(&env, &input.schema_uid)?.is_none() {
                return Err(Error::SchemaNotFound);
            }
            if let Some(ref r) = input.ref_uid {
                if !env
                    .storage()
                    .persistent()
                    .has(&DataKey::Attestation(r.clone()))
                {
                    return Err(Error::RefNotFound);
                }
            }
        }

        let count = u64::from(inputs.len());
        let starting_nonce = Self::read_and_advance_nonce(&env, &issuer, count);
        let mut uids = Vec::new(&env);

        for (i, input) in inputs.iter().enumerate() {
            let nonce = starting_nonce + (i as u64);
            let uid = derive_uid(
                &env,
                &input.schema_uid,
                &issuer,
                &input.subject,
                &input.data,
                &input.expiration,
                &input.ref_uid,
                nonce,
            );

            let attestation = Attestation {
                schema_uid: input.schema_uid.clone(),
                issuer: issuer.clone(),
                subject: input.subject.clone(),
                data: input.data,
                expiration: input.expiration,
                ref_uid: input.ref_uid,
                revoked: false,
                revocation_time: None,
                created_at: now,
            };

            Self::write_attestation(&env, &uid, &attestation);
            Self::push_subject_index(&env, &input.subject, &uid);
            Self::increment_schema_count(&env, &input.schema_uid, 1);

            Attested {
                schema_uid: input.schema_uid,
                attestation_uid: uid.clone(),
                issuer: issuer.clone(),
                subject: input.subject,
                expiration: input.expiration,
            }
            .publish(&env);

            uids.push_back(uid);
        }

        Ok(uids)
    }

    /// Revoke an attestation. Only the original issuer may revoke, and only if the
    /// schema is revocable and the attestation is not already revoked.
    ///
    /// # Errors
    ///
    /// - Returns `Error::AttestationNotFound` if `attestation_uid` does not exist.
    /// - Returns `Error::NotAuthorizedToRevoke` if caller is not the attestation's issuer.
    /// - Returns `Error::AlreadyRevoked` if the attestation is already revoked.
    /// - Returns `Error::SchemaNotFound` if the referenced schema cannot be found.
    /// - Returns `Error::SchemaNotRevocable` if the schema does not permit revocation.
    /// - Returns `Error::NotInitialized` if the registry contract address is not set.
    pub fn revoke(env: Env, issuer: Address, attestation_uid: BytesN<32>) -> Result<(), Error> {
        issuer.require_auth();

        let att_key = DataKey::Attestation(attestation_uid.clone());
        let mut attestation: Attestation = env
            .storage()
            .persistent()
            .get(&att_key)
            .ok_or(Error::AttestationNotFound)?;

        if attestation.issuer != issuer {
            return Err(Error::NotAuthorizedToRevoke);
        }
        if attestation.revoked {
            return Err(Error::AlreadyRevoked);
        }

        let schema =
            Self::lookup_schema(&env, &attestation.schema_uid)?.ok_or(Error::SchemaNotFound)?;
        if !schema.revocable {
            return Err(Error::SchemaNotRevocable);
        }

        let now = env.ledger().timestamp();
        attestation.revoked = true;
        attestation.revocation_time = Some(now);
        Self::write_attestation(&env, &attestation_uid, &attestation);

        Revoked {
            schema_uid: attestation.schema_uid.clone(),
            attestation_uid,
            issuer,
            subject: attestation.subject.clone(),
            revocation_time: now,
        }
        .publish(&env);

        Ok(())
    }

    /// Read an attestation by uid. Extends its TTL on a hit.
    pub fn get_attestation(env: Env, attestation_uid: BytesN<32>) -> Option<Attestation> {
        let key = DataKey::Attestation(attestation_uid);
        let att = env.storage().persistent().get::<_, Attestation>(&key);
        if att.is_some() {
            env.storage()
                .persistent()
                .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
        }
        att
    }

    /// Return true if the attestation exists, is not revoked, and is not expired.
    pub fn is_valid(env: Env, attestation_uid: BytesN<32>) -> bool {
        let key = DataKey::Attestation(attestation_uid);
        let att: Option<Attestation> = env.storage().persistent().get(&key);
        match att {
            None => false,
            Some(a) => {
                env.storage()
                    .persistent()
                    .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
                if a.revoked {
                    false
                } else {
                    match a.expiration {
                        None => true,
                        Some(exp) => exp > env.ledger().timestamp(),
                    }
                }
            }
        }
    }

    /// Return the capped, most-recent index of attestation uids for a subject.
    pub fn attestations_for_subject(env: Env, subject: Address) -> Vec<BytesN<32>> {
        let key = DataKey::Subject(subject);
        let list: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(&env));
        if !list.is_empty() {
            env.storage()
                .persistent()
                .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
        }
        list
    }

    /// Number of attestations ever issued under a schema.
    pub fn schema_attestation_count(env: Env, schema_uid: BytesN<32>) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::SchemaCount(schema_uid))
            .unwrap_or(0)
    }

    /// The bound schema registry contract id.
    ///
    /// # Errors
    ///
    /// Returns `Error::NotInitialized` if the registry contract address is not set.
    pub fn registry(env: Env) -> Result<Address, Error> {
        registry_id(&env)
    }

    fn lookup_schema(env: &Env, schema_uid: &BytesN<32>) -> Result<Option<SchemaView>, Error> {
        let registry = registry_id(env)?;
        let args: Vec<Val> = vec![env, schema_uid.into_val(env)];
        let schema: Option<SchemaView> =
            env.invoke_contract(&registry, &Symbol::new(env, "get_schema"), args);
        Ok(schema)
    }

    fn write_attestation(env: &Env, uid: &BytesN<32>, attestation: &Attestation) {
        let att_key = DataKey::Attestation(uid.clone());
        env.storage().persistent().set(&att_key, attestation);
        env.storage()
            .persistent()
            .extend_ttl(&att_key, TTL_THRESHOLD, TTL_EXTEND);
    }

    fn read_and_advance_nonce(env: &Env, issuer: &Address, count: u64) -> u64 {
        let nonce_key = DataKey::Nonce(issuer.clone());
        let nonce: u64 = env.storage().persistent().get(&nonce_key).unwrap_or(0);
        env.storage().persistent().set(&nonce_key, &(nonce + count));
        nonce
    }

    fn increment_schema_count(env: &Env, schema_uid: &BytesN<32>, delta: u64) {
        let count_key = DataKey::SchemaCount(schema_uid.clone());
        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        env.storage().persistent().set(&count_key, &(count + delta));
    }

    fn push_subject_index(env: &Env, subject: &Address, uid: &BytesN<32>) {
        let key = DataKey::Subject(subject.clone());
        let mut list: Vec<BytesN<32>> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        list.push_back(uid.clone());
        while list.len() > MAX_SUBJECT_INDEX {
            list.remove(0);
        }
        env.storage().persistent().set(&key, &list);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
    }
}

mod test;

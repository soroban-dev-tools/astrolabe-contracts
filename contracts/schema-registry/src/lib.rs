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

//! Astrolabe schema registry.
//!
//! Registers immutable, named, typed schemas. Each schema is identified by a
//! `schema_uid` derived from its name, definition, and authority, so the same
//! name registered by two authorities never collides. See `docs/protocol.md`
//! for the normative specification.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, xdr::ToXdr, Address, Bytes,
    BytesN, Env, String, Symbol,
};

/// Maximum length of a schema definition string, in bytes.
const MAX_DEFINITION_LEN: u32 = 512;

/// TTL: extend a schema entry when its remaining TTL drops below this many ledgers.
const TTL_THRESHOLD: u32 = 518_400; // ~30 days at 5s ledgers
/// TTL: extend a schema entry to this many ledgers.
const TTL_EXTEND: u32 = 1_036_800; // ~60 days at 5s ledgers

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    SchemaAlreadyExists = 1,
    DefinitionTooLong = 2,
    SchemaNotFound = 3,
}

/// A registered schema. Immutable once written.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub authority: Address,
    pub name: Symbol,
    pub definition: String,
    pub revocable: bool,
    pub resolver: Option<Address>,
}

#[contracttype]
enum DataKey {
    Schema(BytesN<32>),
}

/// Emitted when a schema is registered.
#[contractevent(topics = ["schema", "register"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaRegistered {
    #[topic]
    pub schema_uid: BytesN<32>,
    pub authority: Address,
    pub name: Symbol,
    pub revocable: bool,
}

/// Derive the schema uid: `sha256(xdr(name) || xdr(definition) || xdr(authority))`.
fn derive_uid(env: &Env, name: &Symbol, definition: &String, authority: &Address) -> BytesN<32> {
    let mut buf = Bytes::new(env);
    buf.append(&name.clone().to_xdr(env));
    buf.append(&definition.clone().to_xdr(env));
    buf.append(&authority.clone().to_xdr(env));
    env.crypto().sha256(&buf).to_bytes()
}

#[contract]
pub struct SchemaRegistry;

#[contractimpl]
impl SchemaRegistry {
    /// Register an immutable schema. Returns the schema uid.
    ///
    /// Fails with `DefinitionTooLong` if the definition exceeds the byte limit,
    /// or `SchemaAlreadyExists` if the derived uid is already registered.
    pub fn register_schema(
        env: Env,
        authority: Address,
        name: Symbol,
        definition: String,
        revocable: bool,
        resolver: Option<Address>,
    ) -> Result<BytesN<32>, Error> {
        authority.require_auth();

        if definition.len() > MAX_DEFINITION_LEN {
            return Err(Error::DefinitionTooLong);
        }

        let uid = derive_uid(&env, &name, &definition, &authority);
        let key = DataKey::Schema(uid.clone());

        if env.storage().persistent().has(&key) {
            return Err(Error::SchemaAlreadyExists);
        }

        let schema = Schema {
            authority: authority.clone(),
            name: name.clone(),
            definition,
            revocable,
            resolver,
        };
        env.storage().persistent().set(&key, &schema);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);

        SchemaRegistered {
            schema_uid: uid.clone(),
            authority,
            name,
            revocable,
        }
        .publish(&env);

        Ok(uid)
    }

    /// Read a schema by uid. Extends the entry TTL on a hit.
    pub fn get_schema(env: Env, schema_uid: BytesN<32>) -> Option<Schema> {
        let key = DataKey::Schema(schema_uid);
        let schema = env.storage().persistent().get::<_, Schema>(&key);
        if schema.is_some() {
            env.storage()
                .persistent()
                .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
        }
        schema
    }
}

mod test;

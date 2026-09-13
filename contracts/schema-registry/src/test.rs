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

#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, String};

fn setup() -> (Env, SchemaRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(SchemaRegistry, ());
    let client = SchemaRegistryClient::new(&env, &contract_id);
    let authority = Address::generate(&env);
    (env, client, authority)
}

#[test]
fn register_and_read_back() {
    let (env, client, authority) = setup();
    let name = symbol_short!("merchant");
    let definition = String::from_str(&env, "string name,string country");

    let uid = client.register_schema(&authority, &name, &definition, &true, &None);

    let schema = client.get_schema(&uid).expect("schema should exist");
    assert_eq!(schema.authority, authority);
    assert_eq!(schema.name, name);
    assert_eq!(schema.definition, definition);
    assert!(schema.revocable);
    assert_eq!(schema.resolver, None);
}

#[test]
fn get_unknown_returns_none() {
    let (env, client, _authority) = setup();
    let unknown = BytesN::from_array(&env, &[9u8; 32]);
    assert_eq!(client.get_schema(&unknown), None);
}

#[test]
fn duplicate_rejected() {
    let (env, client, authority) = setup();
    let name = symbol_short!("kyc");
    let definition = String::from_str(&env, "u32 tier");

    client.register_schema(&authority, &name, &definition, &true, &None);

    let res = client.try_register_schema(&authority, &name, &definition, &true, &None);
    assert_eq!(res, Err(Ok(Error::SchemaAlreadyExists)));
}

#[test]
fn same_name_different_authority_does_not_collide() {
    let (env, client, authority_a) = setup();
    let authority_b = Address::generate(&env);
    let name = symbol_short!("badge");
    let definition = String::from_str(&env, "string label");

    let uid_a = client.register_schema(&authority_a, &name, &definition, &false, &None);
    let uid_b = client.register_schema(&authority_b, &name, &definition, &false, &None);

    assert_ne!(uid_a, uid_b);
}

#[test]
fn oversized_definition_rejected() {
    let (env, client, authority) = setup();
    let name = symbol_short!("big");
    let big = [b'a'; (MAX_DEFINITION_LEN + 1) as usize];
    let definition = String::from_bytes(&env, &big);

    let res = client.try_register_schema(&authority, &name, &definition, &true, &None);
    assert_eq!(res, Err(Ok(Error::DefinitionTooLong)));
}

#[test]
fn definition_at_limit_accepted() {
    let (env, client, authority) = setup();
    let name = symbol_short!("atlimit");
    let exact = [b'a'; MAX_DEFINITION_LEN as usize];
    let definition = String::from_bytes(&env, &exact);

    let uid = client.register_schema(&authority, &name, &definition, &true, &None);
    assert!(client.get_schema(&uid).is_some());
}

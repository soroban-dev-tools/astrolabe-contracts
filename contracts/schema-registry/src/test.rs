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
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    Address, Env, String,
};

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

#[test]
fn register_publishes_event() {
    let (env, client, authority) = setup();
    let name = symbol_short!("testevt");
    let definition = String::from_str(&env, "string k");

    let _uid = client.register_schema(&authority, &name, &definition, &true, &None);

    let events = env.events().all();
    assert_eq!(events.events().len(), 1);
    let event = &events.events()[0];
    let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
    assert_eq!(v0.topics.len(), 3);
}

#[test]
fn register_with_resolver_address() {
    let (env, client, authority) = setup();
    let resolver = Address::generate(&env);
    let name = symbol_short!("reslv");
    let definition = String::from_str(&env, "string id");

    let uid = client.register_schema(
        &authority,
        &name,
        &definition,
        &true,
        &Some(resolver.clone()),
    );
    let schema = client.get_schema(&uid).expect("schema exists");
    assert_eq!(schema.resolver, Some(resolver));
}

#[test]
fn schema_uid_determinism_identical_inputs() {
    let (env, _, authority) = setup();
    let name = symbol_short!("det_test");
    let definition = String::from_str(&env, "u32 value");

    let uid1 = derive_uid(&env, &name, &definition, &authority);
    let uid2 = derive_uid(&env, &name, &definition, &authority);

    assert_eq!(uid1, uid2);
}

#[test]
fn schema_uid_sensitivity_name_change() {
    let (env, _, authority) = setup();
    let name1 = symbol_short!("name_one");
    let name2 = symbol_short!("name_two");
    let definition = String::from_str(&env, "u32 value");

    let uid1 = derive_uid(&env, &name1, &definition, &authority);
    let uid2 = derive_uid(&env, &name2, &definition, &authority);

    assert_ne!(uid1, uid2);
}

#[test]
fn schema_uid_sensitivity_definition_change() {
    let (env, _, authority) = setup();
    let name = symbol_short!("sch_def");
    let def1 = String::from_str(&env, "string title");
    let def2 = String::from_str(&env, "string summary");

    let uid1 = derive_uid(&env, &name, &def1, &authority);
    let uid2 = derive_uid(&env, &name, &def2, &authority);

    assert_ne!(uid1, uid2);
}

#[test]
fn schema_uid_sensitivity_authority_change() {
    let (env, _, authority1) = setup();
    let authority2 = Address::generate(&env);
    let name = symbol_short!("sch_auth");
    let definition = String::from_str(&env, "bool verified");

    let uid1 = derive_uid(&env, &name, &definition, &authority1);
    let uid2 = derive_uid(&env, &name, &definition, &authority2);

    assert_ne!(uid1, uid2);
}

#[test]
fn schema_uid_regression_golden_value() {
    let env = Env::default();
    let authority = Address::from_str(
        &env,
        "CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC",
    );
    let name = symbol_short!("kyc_tier");
    let definition = String::from_str(&env, "u32 tier");

    let uid = derive_uid(&env, &name, &definition, &authority);
    let expected = BytesN::from_array(
        &env,
        &[
            138, 62, 209, 75, 102, 79, 172, 7, 113, 40, 66, 48, 77, 154, 195, 26, 231, 81, 238, 30,
            50, 197, 55, 238, 185, 10, 236, 21, 76, 10, 87, 3,
        ],
    );
    assert_eq!(uid, expected);
}

#[test]
#[should_panic]
fn register_schema_without_auth_panics() {
    let env = Env::default();
    let contract_id = env.register(SchemaRegistry, ());
    let client = SchemaRegistryClient::new(&env, &contract_id);
    let authority = Address::generate(&env);

    client.register_schema(
        &authority,
        &symbol_short!("no_auth"),
        &String::from_str(&env, "string note"),
        &true,
        &None,
    );
}

#[test]
fn register_schema_with_empty_definition_succeeds() {
    let (env, client, authority) = setup();
    let name = symbol_short!("empty");
    let empty_def = String::from_str(&env, "");

    let uid = client.register_schema(&authority, &name, &empty_def, &false, &None);
    let schema = client.get_schema(&uid).expect("schema exists");
    assert_eq!(schema.definition, empty_def);
    assert!(!schema.revocable);
}

#[test]
fn schema_registry_error_codes_align_with_specification() {
    assert_eq!(Error::SchemaAlreadyExists as u32, 1);
    assert_eq!(Error::DefinitionTooLong as u32, 2);
    assert_eq!(Error::SchemaNotFound as u32, 3);
}

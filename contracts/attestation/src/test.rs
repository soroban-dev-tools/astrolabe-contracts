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
use astrolabe_schema_registry::{SchemaRegistry, SchemaRegistryClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    Address, Bytes, Env, String,
};

struct Fixture<'a> {
    env: Env,
    att: AttestationContractClient<'a>,
    registry: SchemaRegistryClient<'a>,
    authority: Address,
    issuer: Address,
    subject: Address,
}

fn setup() -> Fixture<'static> {
    let env = Env::default();
    env.mock_all_auths();

    let registry_id = env.register(SchemaRegistry, ());
    let registry = SchemaRegistryClient::new(&env, &registry_id);

    let att_id = env.register(AttestationContract, (registry_id.clone(),));
    let att = AttestationContractClient::new(&env, &att_id);

    Fixture {
        authority: Address::generate(&env),
        issuer: Address::generate(&env),
        subject: Address::generate(&env),
        env,
        att,
        registry,
    }
}

fn register_schema(f: &Fixture, revocable: bool) -> BytesN<32> {
    f.registry.register_schema(
        &f.authority,
        &symbol_short!("merchant"),
        &String::from_str(&f.env, "string name"),
        &revocable,
        &None,
    )
}

fn data(env: &Env) -> Bytes {
    Bytes::from_array(env, &[1, 2, 3, 4])
}

#[test]
fn attest_and_read_back() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    let a = f.att.get_attestation(&uid).expect("exists");
    assert_eq!(a.issuer, f.issuer);
    assert_eq!(a.subject, f.subject);
    assert_eq!(a.schema_uid, schema_uid);
    assert!(!a.revoked);
    assert!(f.att.is_valid(&uid));
}

#[test]
fn attest_unknown_schema_fails() {
    let f = setup();
    let bogus = BytesN::from_array(&f.env, &[7u8; 32]);
    let res = f
        .att
        .try_attest(&f.issuer, &bogus, &f.subject, &data(&f.env), &None, &None);
    assert_eq!(res, Err(Ok(Error::SchemaNotFound)));
}

#[test]
fn data_too_long_fails() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    let big = Bytes::from_array(&f.env, &[0u8; (MAX_DATA_LEN + 1) as usize]);
    let res = f
        .att
        .try_attest(&f.issuer, &schema_uid, &f.subject, &big, &None, &None);
    assert_eq!(res, Err(Ok(Error::DataTooLong)));
}

#[test]
fn revoke_by_issuer_succeeds() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    f.att.revoke(&f.issuer, &uid);

    let a = f.att.get_attestation(&uid).expect("exists");
    assert!(a.revoked);
    assert!(a.revocation_time.is_some());
    assert!(!f.att.is_valid(&uid));
}

#[test]
fn revoke_by_stranger_fails() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    let stranger = Address::generate(&f.env);
    let res = f.att.try_revoke(&stranger, &uid);
    assert_eq!(res, Err(Ok(Error::NotAuthorizedToRevoke)));
}

#[test]
fn revoke_non_revocable_schema_fails() {
    let f = setup();
    let schema_uid = register_schema(&f, false);
    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    let res = f.att.try_revoke(&f.issuer, &uid);
    assert_eq!(res, Err(Ok(Error::SchemaNotRevocable)));
}

#[test]
fn double_revoke_fails() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    f.att.revoke(&f.issuer, &uid);
    let res = f.att.try_revoke(&f.issuer, &uid);
    assert_eq!(res, Err(Ok(Error::AlreadyRevoked)));
}

#[test]
fn revoke_unknown_fails() {
    let f = setup();
    let bogus = BytesN::from_array(&f.env, &[3u8; 32]);
    let res = f.att.try_revoke(&f.issuer, &bogus);
    assert_eq!(res, Err(Ok(Error::AttestationNotFound)));
}

#[test]
fn expired_attestation_is_invalid() {
    let f = setup();
    f.env.ledger().with_mut(|l| l.timestamp = 1_000);
    let schema_uid = register_schema(&f, true);

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(2_000),
        &None,
    );
    assert!(f.att.is_valid(&uid));

    f.env.ledger().with_mut(|l| l.timestamp = 2_001);
    assert!(!f.att.is_valid(&uid));
    // Still readable in storage; only validity flips.
    assert!(f.att.get_attestation(&uid).is_some());
}

#[test]
fn subject_index_updates_and_caps() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let total = MAX_SUBJECT_INDEX + 5;
    let mut first_uid: Option<BytesN<32>> = None;
    for i in 0..total {
        let byte_val = u8::try_from(i % 256).unwrap();
        let d = Bytes::from_array(&f.env, &[byte_val]);
        let uid = f
            .att
            .attest(&f.issuer, &schema_uid, &f.subject, &d, &None, &None);
        if i == 0 {
            first_uid = Some(uid);
        }
    }

    let index = f.att.attestations_for_subject(&f.subject);
    assert_eq!(index.len(), MAX_SUBJECT_INDEX);
    // The oldest uid was evicted from the index.
    assert!(!index.contains(first_uid.unwrap()));
    // But its record still exists in storage.
    assert_eq!(
        f.att.schema_attestation_count(&schema_uid),
        u64::from(total)
    );
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn attest_without_auth_panics() {
    // No mock_all_auths here: require_auth must reject.
    let env = Env::default();
    let registry_id = env.register(SchemaRegistry, ());
    let att_id = env.register(AttestationContract, (registry_id.clone(),));
    let att = AttestationContractClient::new(&env, &att_id);

    let issuer = Address::generate(&env);
    let subject = Address::generate(&env);
    let schema_uid = BytesN::from_array(&env, &[1u8; 32]);
    att.attest(&issuer, &schema_uid, &subject, &data(&env), &None, &None);
}

#[test]
fn attest_publishes_event() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let _uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(5_000),
        &None,
    );

    let events = f.env.events().all();
    let contract_events = events.filter_by_contract(&f.att.address);
    assert_eq!(contract_events.events().len(), 1);
    let event = &contract_events.events()[0];
    let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
    assert_eq!(v0.topics.len(), 3);
}

#[test]
fn revoke_publishes_event() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    f.att.revoke(&f.issuer, &uid);

    let events = f.env.events().all();
    let contract_events = events.filter_by_contract(&f.att.address);
    assert_eq!(contract_events.events().len(), 1);
    let revoke_event = &contract_events.events()[0];
    let soroban_sdk::xdr::ContractEventBody::V0(v0) = &revoke_event.body;
    assert_eq!(v0.topics.len(), 3);
}

#[test]
fn is_valid_returns_false_for_unknown_attestation() {
    let f = setup();
    let unknown = BytesN::from_array(&f.env, &[8u8; 32]);
    assert!(!f.att.is_valid(&unknown));
}

#[test]
fn attestations_for_subject_empty_for_new_address() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let list = f.att.attestations_for_subject(&stranger);
    assert_eq!(list.len(), 0);
}

#[test]
fn registry_endpoint_returns_bound_schema_registry() {
    let f = setup();
    assert_eq!(f.att.registry(), f.registry.address);
}

#[test]
fn schema_attestation_count_zero_for_unused_schema() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    assert_eq!(f.att.schema_attestation_count(&schema_uid), 0);

    f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert_eq!(f.att.schema_attestation_count(&schema_uid), 1);
}

#[test]
fn attest_with_reference_uid_succeeds() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let first_uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );

    let second_uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &Some(first_uid.clone()),
    );

    let a = f.att.get_attestation(&second_uid).expect("exists");
    assert_eq!(a.ref_uid, Some(first_uid));
}

#[test]
fn get_unknown_returns_none() {
    let f = setup();
    let unknown = BytesN::from_array(&f.env, &[4u8; 32]);
    assert_eq!(f.att.get_attestation(&unknown), None);
}

#[test]
fn revoke_expired_attestation_succeeds() {
    let f = setup();
    f.env.ledger().with_mut(|l| l.timestamp = 1_000);
    let schema_uid = register_schema(&f, true);

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(2_000),
        &None,
    );

    f.env.ledger().with_mut(|l| l.timestamp = 2_500);
    assert!(!f.att.is_valid(&uid));

    f.att.revoke(&f.issuer, &uid);
    let a = f.att.get_attestation(&uid).expect("exists");
    assert!(a.revoked);
    assert_eq!(a.revocation_time, Some(2_500));
}

#[test]
fn attestation_uid_determinism_identical_inputs() {
    let f = setup();
    let schema_uid = BytesN::from_array(&f.env, &[2u8; 32]);
    let d = data(&f.env);
    let exp = Some(10_000);
    let ref_uid = Some(BytesN::from_array(&f.env, &[3u8; 32]));

    let uid1 = derive_uid(&f.env, &schema_uid, &f.issuer, &f.subject, &d, &exp, &ref_uid, 0);
    let uid2 = derive_uid(&f.env, &schema_uid, &f.issuer, &f.subject, &d, &exp, &ref_uid, 0);

    assert_eq!(uid1, uid2);
}

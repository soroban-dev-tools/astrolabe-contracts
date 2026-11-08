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

    let uid1 = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &exp,
        &ref_uid,
        0,
    );
    let uid2 = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &exp,
        &ref_uid,
        0,
    );

    assert_eq!(uid1, uid2);
}

#[test]
fn attestation_uid_sensitivity_schema_and_subject() {
    let f = setup();
    let schema_uid1 = BytesN::from_array(&f.env, &[2u8; 32]);
    let schema_uid2 = BytesN::from_array(&f.env, &[7u8; 32]);
    let subject2 = Address::generate(&f.env);
    let d = data(&f.env);

    let base_uid = derive_uid(
        &f.env,
        &schema_uid1,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &None,
        0,
    );

    let diff_schema = derive_uid(
        &f.env,
        &schema_uid2,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &None,
        0,
    );
    assert_ne!(base_uid, diff_schema);

    let diff_subject = derive_uid(
        &f.env,
        &schema_uid1,
        &f.issuer,
        &subject2,
        &d,
        &None,
        &None,
        0,
    );
    assert_ne!(base_uid, diff_subject);
}

#[test]
fn attestation_uid_sensitivity_data_and_expiration() {
    let f = setup();
    let schema_uid = BytesN::from_array(&f.env, &[2u8; 32]);
    let d1 = Bytes::from_array(&f.env, &[1, 2, 3]);
    let d2 = Bytes::from_array(&f.env, &[1, 2, 4]);

    let base_uid = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d1,
        &Some(1_000),
        &None,
        0,
    );

    let diff_data = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d2,
        &Some(1_000),
        &None,
        0,
    );
    assert_ne!(base_uid, diff_data);

    let diff_exp = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d1,
        &Some(2_000),
        &None,
        0,
    );
    assert_ne!(base_uid, diff_exp);

    let none_exp = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d1,
        &None,
        &None,
        0,
    );
    assert_ne!(base_uid, none_exp);
}

#[test]
fn attestation_uid_sensitivity_ref_and_nonce() {
    let f = setup();
    let schema_uid = BytesN::from_array(&f.env, &[2u8; 32]);
    let ref1 = BytesN::from_array(&f.env, &[10u8; 32]);
    let ref2 = BytesN::from_array(&f.env, &[11u8; 32]);
    let d = data(&f.env);

    let base_uid = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &Some(ref1.clone()),
        0,
    );

    let diff_ref = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &Some(ref2),
        0,
    );
    assert_ne!(base_uid, diff_ref);

    let none_ref = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &None,
        0,
    );
    assert_ne!(base_uid, none_ref);

    let diff_nonce = derive_uid(
        &f.env,
        &schema_uid,
        &f.issuer,
        &f.subject,
        &d,
        &None,
        &Some(ref1),
        1,
    );
    assert_ne!(base_uid, diff_nonce);
}

#[test]
fn attestation_uid_regression_golden_value() {
    let env = Env::default();
    let schema_uid = BytesN::from_array(&env, &[1u8; 32]);
    let issuer = Address::from_str(
        &env,
        "CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC",
    );
    let subject = Address::from_str(
        &env,
        "CB2773G3PKQVSECFXTM2Q65W2XJLMWGQXC5UTFBQEPHYPAGUEXEYBMW4",
    );
    let d = Bytes::from_array(&env, &[10, 20, 30]);
    let exp = Some(1_700_000_000);

    let uid = derive_uid(&env, &schema_uid, &issuer, &subject, &d, &exp, &None, 0);
    let expected = BytesN::from_array(
        &env,
        &[
            134, 136, 219, 199, 4, 59, 0, 239, 30, 77, 178, 153, 219, 159, 74, 208, 226, 24, 240,
            21, 29, 72, 234, 167, 114, 247, 251, 175, 112, 126, 201, 122,
        ],
    );
    assert_eq!(uid, expected);
}

#[test]
fn characterisation_multi_subject_index_and_issuer_nonces() {
    let f = setup();
    let schema_uid = register_schema(&f, true);
    let subject_b = Address::generate(&f.env);
    let issuer_b = Address::generate(&f.env);

    let uid_a1 = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    let uid_a2 = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert_ne!(uid_a1, uid_a2);

    let uid_b1 = f.att.attest(
        &f.issuer,
        &schema_uid,
        &subject_b,
        &data(&f.env),
        &None,
        &None,
    );
    let uid_b2 = f.att.attest(
        &issuer_b,
        &schema_uid,
        &subject_b,
        &data(&f.env),
        &None,
        &None,
    );
    assert_ne!(uid_b1, uid_b2);

    let index_a = f.att.attestations_for_subject(&f.subject);
    let index_b = f.att.attestations_for_subject(&subject_b);
    assert_eq!(index_a.len(), 2);
    assert_eq!(index_b.len(), 2);
    assert!(index_a.contains(uid_a1));
    assert!(index_a.contains(uid_a2));
    assert!(index_b.contains(uid_b1));
    assert!(index_b.contains(uid_b2));
}

#[test]
fn attest_expiration_in_past_and_present_rejected() {
    let f = setup();
    f.env.ledger().with_mut(|l| l.timestamp = 2_000);
    let schema_uid = register_schema(&f, true);

    let res_past = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(1_999),
        &None,
    );
    assert_eq!(res_past, Err(Ok(Error::ExpirationInPast)));

    let res_now = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(2_000),
        &None,
    );
    assert_eq!(res_now, Err(Ok(Error::ExpirationInPast)));

    let res_future = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &Some(2_001),
        &None,
    );
    assert!(res_future.is_ok());
}

#[test]
fn attest_ref_uid_validation_scenarios() {
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

    let missing_ref = BytesN::from_array(&f.env, &[99u8; 32]);
    let res_missing = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &Some(missing_ref),
    );
    assert_eq!(res_missing, Err(Ok(Error::RefNotFound)));

    let res_valid = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &Some(first_uid.clone()),
    );
    assert!(res_valid.is_ok());
    let second_uid = res_valid.unwrap().unwrap();
    let att = f.att.get_attestation(&second_uid).unwrap();
    assert_eq!(att.ref_uid, Some(first_uid));
}

#[test]
fn attest_batch_valid_entries_advance_nonces() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let mut inputs = Vec::new(&f.env);
    for i in 0..5 {
        inputs.push_back(AttestInput {
            schema_uid: schema_uid.clone(),
            subject: f.subject.clone(),
            data: Bytes::from_array(&f.env, &[i as u8]),
            expiration: None,
            ref_uid: None,
        });
    }

    let uids = f.att.attest_batch(&f.issuer, &inputs);
    assert_eq!(uids.len(), 5);

    for i in 0..5 {
        for j in (i + 1)..5 {
            assert_ne!(uids.get(i).unwrap(), uids.get(j).unwrap());
        }
        let a = f.att.get_attestation(&uids.get(i).unwrap()).unwrap();
        assert_eq!(a.issuer, f.issuer);
        assert_eq!(a.subject, f.subject);
        assert!(!a.revoked);
    }

    let single_uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert!(!uids.contains(single_uid));
}

#[test]
fn attest_batch_mixed_invalid_reverts_entire_batch() {
    let f = setup();
    f.env.ledger().with_mut(|l| l.timestamp = 2_000);
    let schema_uid = register_schema(&f, true);

    let mut inputs = Vec::new(&f.env);
    inputs.push_back(AttestInput {
        schema_uid: schema_uid.clone(),
        subject: f.subject.clone(),
        data: Bytes::from_array(&f.env, &[1]),
        expiration: None,
        ref_uid: None,
    });
    inputs.push_back(AttestInput {
        schema_uid: schema_uid.clone(),
        subject: f.subject.clone(),
        data: Bytes::from_array(&f.env, &[2]),
        expiration: Some(1_999),
        ref_uid: None,
    });
    inputs.push_back(AttestInput {
        schema_uid: schema_uid.clone(),
        subject: f.subject.clone(),
        data: Bytes::from_array(&f.env, &[3]),
        expiration: None,
        ref_uid: None,
    });

    let res = f.att.try_attest_batch(&f.issuer, &inputs);
    assert_eq!(res, Err(Ok(Error::ExpirationInPast)));

    assert_eq!(f.att.schema_attestation_count(&schema_uid), 0);
    assert_eq!(f.att.attestations_for_subject(&f.subject).len(), 0);

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert_eq!(f.att.schema_attestation_count(&schema_uid), 1);
    assert!(f.att.is_valid(&uid));
}

#[test]
fn attest_batch_multi_schema_multi_subject_and_empty() {
    let f = setup();
    let schema_a = register_schema(&f, true);
    let schema_b = f.registry.register_schema(
        &f.authority,
        &symbol_short!("ngo"),
        &String::from_str(&f.env, "string reg_id"),
        &true,
        &None,
    );
    let subject_2 = Address::generate(&f.env);

    let empty_inputs = Vec::new(&f.env);
    let empty_res = f.att.attest_batch(&f.issuer, &empty_inputs);
    assert_eq!(empty_res.len(), 0);

    let mut inputs = Vec::new(&f.env);
    inputs.push_back(AttestInput {
        schema_uid: schema_a.clone(),
        subject: f.subject.clone(),
        data: Bytes::from_array(&f.env, &[1]),
        expiration: None,
        ref_uid: None,
    });
    inputs.push_back(AttestInput {
        schema_uid: schema_b.clone(),
        subject: f.subject.clone(),
        data: Bytes::from_array(&f.env, &[2]),
        expiration: None,
        ref_uid: None,
    });
    inputs.push_back(AttestInput {
        schema_uid: schema_a.clone(),
        subject: subject_2.clone(),
        data: Bytes::from_array(&f.env, &[3]),
        expiration: None,
        ref_uid: None,
    });

    let uids = f.att.attest_batch(&f.issuer, &inputs);
    assert_eq!(uids.len(), 3);

    assert_eq!(f.att.schema_attestation_count(&schema_a), 2);
    assert_eq!(f.att.schema_attestation_count(&schema_b), 1);

    let index_1 = f.att.attestations_for_subject(&f.subject);
    let index_2 = f.att.attestations_for_subject(&subject_2);
    assert_eq!(index_1.len(), 2);
    assert_eq!(index_2.len(), 1);
    assert!(index_1.contains(uids.get(0).unwrap()));
    assert!(index_1.contains(uids.get(1).unwrap()));
    assert!(index_2.contains(uids.get(2).unwrap()));
}

#[contract]
pub struct MockResolver;

#[contracttype]
pub enum MockResolverKey {
    ApproveAttest,
    ApproveRevoke,
}

#[contractimpl]
impl MockResolver {
    pub fn init(env: Env, approve_attest: bool, approve_revoke: bool) {
        env.storage()
            .instance()
            .set(&MockResolverKey::ApproveAttest, &approve_attest);
        env.storage()
            .instance()
            .set(&MockResolverKey::ApproveRevoke, &approve_revoke);
    }

    pub fn set_approve_attest(env: Env, approve: bool) {
        env.storage()
            .instance()
            .set(&MockResolverKey::ApproveAttest, &approve);
    }

    pub fn set_approve_revoke(env: Env, approve: bool) {
        env.storage()
            .instance()
            .set(&MockResolverKey::ApproveRevoke, &approve);
    }

    pub fn on_attest(env: Env, _attestation: Attestation) -> bool {
        env.storage()
            .instance()
            .get(&MockResolverKey::ApproveAttest)
            .unwrap_or(true)
    }

    pub fn on_revoke(env: Env, _attestation: Attestation) -> bool {
        env.storage()
            .instance()
            .get(&MockResolverKey::ApproveRevoke)
            .unwrap_or(true)
    }
}

#[test]
fn resolver_approval_allows_attest_and_revoke() {
    let f = setup();
    let resolver_id = f.env.register(MockResolver, ());
    let resolver_client = MockResolverClient::new(&f.env, &resolver_id);
    resolver_client.init(&true, &true);

    let schema_uid = f.registry.register_schema(
        &f.authority,
        &symbol_short!("resolved"),
        &String::from_str(&f.env, "string note"),
        &true,
        &Some(resolver_id),
    );

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert!(f.att.is_valid(&uid));

    f.att.revoke(&f.issuer, &uid);
    assert!(!f.att.is_valid(&uid));
    let rec = f.att.get_attestation(&uid).unwrap();
    assert!(rec.revoked);
}

#[test]
fn resolver_rejection_on_attest_fails() {
    let f = setup();
    let resolver_id = f.env.register(MockResolver, ());
    let resolver_client = MockResolverClient::new(&f.env, &resolver_id);
    resolver_client.init(&false, &true);

    let schema_uid = f.registry.register_schema(
        &f.authority,
        &symbol_short!("reject1"),
        &String::from_str(&f.env, "string note"),
        &true,
        &Some(resolver_id),
    );

    let res = f.att.try_attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert_eq!(res, Err(Ok(Error::ResolverRejected)));
}

#[test]
fn resolver_rejection_on_revoke_fails() {
    let f = setup();
    let resolver_id = f.env.register(MockResolver, ());
    let resolver_client = MockResolverClient::new(&f.env, &resolver_id);
    resolver_client.init(&true, &false);

    let schema_uid = f.registry.register_schema(
        &f.authority,
        &symbol_short!("reject2"),
        &String::from_str(&f.env, "string note"),
        &true,
        &Some(resolver_id),
    );

    let uid = f.att.attest(
        &f.issuer,
        &schema_uid,
        &f.subject,
        &data(&f.env),
        &None,
        &None,
    );
    assert!(f.att.is_valid(&uid));

    let res = f.att.try_revoke(&f.issuer, &uid);
    assert_eq!(res, Err(Ok(Error::ResolverRejected)));
    // Attestation must remain unrevoked and valid
    assert!(f.att.is_valid(&uid));
    let rec = f.att.get_attestation(&uid).unwrap();
    assert!(!rec.revoked);
}

#[test]
fn resolver_rejection_in_batch_reverts_entire_batch() {
    let f = setup();
    let resolver_id = f.env.register(MockResolver, ());
    let resolver_client = MockResolverClient::new(&f.env, &resolver_id);
    resolver_client.init(&false, &true);

    let resolved_schema = f.registry.register_schema(
        &f.authority,
        &symbol_short!("resolv"),
        &String::from_str(&f.env, "string note"),
        &true,
        &Some(resolver_id),
    );
    let open_schema = register_schema(&f, true);

    let mut inputs = Vec::new(&f.env);
    inputs.push_back(AttestInput {
        schema_uid: open_schema.clone(),
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    });
    inputs.push_back(AttestInput {
        schema_uid: resolved_schema.clone(),
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    });

    let res = f.att.try_attest_batch(&f.issuer, &inputs);
    assert_eq!(res, Err(Ok(Error::ResolverRejected)));

    // Ensure zero records were created and counts untouched
    assert_eq!(f.att.schema_attestation_count(&open_schema), 0);
    assert_eq!(f.att.schema_attestation_count(&resolved_schema), 0);
    assert_eq!(f.att.attestations_for_subject(&f.subject).len(), 0);
}

#[test]
fn authorized_signer_configuration_lifecycle() {
    let f = setup();

    assert_eq!(f.att.get_authorized_signer(&f.issuer), None);
    assert_eq!(f.att.get_nonce(&f.issuer), 0);

    let signer = BytesN::from_array(&f.env, &[77u8; 32]);
    f.att.set_authorized_signer(&f.issuer, &signer);

    // Check SignerSet event immediately after invocation
    let events = f.env.events().all();
    let contract_events = events.filter_by_contract(&f.att.address);
    assert_eq!(contract_events.events().len(), 1);

    assert_eq!(f.att.get_authorized_signer(&f.issuer), Some(signer));

    // Clear signer
    f.att.clear_authorized_signer(&f.issuer);

    // Check SignerCleared event immediately after invocation
    let events = f.env.events().all();
    let contract_events = events.filter_by_contract(&f.att.address);
    assert_eq!(contract_events.events().len(), 1);

    assert_eq!(f.att.get_authorized_signer(&f.issuer), None);
}

#[test]
fn attest_delegated_valid_signature_succeeds() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let f = setup();
    let schema_uid = register_schema(&f, true);

    let signing_key = SigningKey::from_bytes(&[15u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signer = BytesN::from_array(&f.env, &verifying_key.to_bytes());

    f.att.set_authorized_signer(&f.issuer, &signer);

    let input = AttestInput {
        schema_uid: schema_uid.clone(),
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    };

    let nonce = f.att.get_nonce(&f.issuer);
    let deadline = f.env.ledger().timestamp() + 3_600;

    let payload = f
        .att
        .delegated_attestation_payload(&f.issuer, &input, &nonce, &deadline);

    let mut payload_bytes = [0u8; 4096];
    let payload_len = payload.len() as usize;
    payload.copy_into_slice(&mut payload_bytes[..payload_len]);
    let signature_raw = signing_key.sign(&payload_bytes[..payload_len]);
    let signature = BytesN::from_array(&f.env, &signature_raw.to_bytes());

    let uid = f
        .att
        .attest_delegated(&f.issuer, &input, &deadline, &signature);

    assert!(f.att.is_valid(&uid));
    let rec = f.att.get_attestation(&uid).unwrap();
    assert_eq!(rec.issuer, f.issuer);
    assert_eq!(rec.subject, f.subject);
    assert_eq!(rec.schema_uid, schema_uid);

    // Nonce must have advanced from 0 to 1
    assert_eq!(f.att.get_nonce(&f.issuer), 1);
    assert_eq!(f.att.schema_attestation_count(&schema_uid), 1);
}

#[test]
fn attest_delegated_unconfigured_signer_rejected() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let input = AttestInput {
        schema_uid,
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    };

    let deadline = f.env.ledger().timestamp() + 1_000;
    let fake_sig = BytesN::from_array(&f.env, &[0u8; 64]);

    let res = f
        .att
        .try_attest_delegated(&f.issuer, &input, &deadline, &fake_sig);
    assert_eq!(res, Err(Ok(Error::SignerNotConfigured)));
}

#[test]
fn attest_delegated_expired_deadline_rejected() {
    let f = setup();
    let schema_uid = register_schema(&f, true);

    let input = AttestInput {
        schema_uid,
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    };

    f.env.ledger().set_timestamp(1_000);
    let expired_deadline = 999;
    let fake_sig = BytesN::from_array(&f.env, &[0u8; 64]);

    let res = f
        .att
        .try_attest_delegated(&f.issuer, &input, &expired_deadline, &fake_sig);
    assert_eq!(res, Err(Ok(Error::DeadlineExceeded)));
}

#[test]
#[should_panic]
fn attest_delegated_invalid_signature_panics() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let f = setup();
    let schema_uid = register_schema(&f, true);

    let signing_key = SigningKey::from_bytes(&[21u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signer = BytesN::from_array(&f.env, &verifying_key.to_bytes());
    f.att.set_authorized_signer(&f.issuer, &signer);

    let input = AttestInput {
        schema_uid,
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    };

    let deadline = f.env.ledger().timestamp() + 3_600;

    let wrong_key = SigningKey::from_bytes(&[99u8; 32]);
    let wrong_sig = wrong_key.sign(b"wrong payload");
    let signature = BytesN::from_array(&f.env, &wrong_sig.to_bytes());

    f.att
        .attest_delegated(&f.issuer, &input, &deadline, &signature);
}

#[test]
#[should_panic]
fn attest_delegated_replay_attempt_panics() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let f = setup();
    let schema_uid = register_schema(&f, true);

    let signing_key = SigningKey::from_bytes(&[33u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signer = BytesN::from_array(&f.env, &verifying_key.to_bytes());
    f.att.set_authorized_signer(&f.issuer, &signer);

    let input = AttestInput {
        schema_uid,
        subject: f.subject.clone(),
        data: data(&f.env),
        expiration: None,
        ref_uid: None,
    };

    let nonce = f.att.get_nonce(&f.issuer);
    let deadline = f.env.ledger().timestamp() + 3_600;

    let payload = f
        .att
        .delegated_attestation_payload(&f.issuer, &input, &nonce, &deadline);

    let mut payload_bytes = [0u8; 4096];
    let payload_len = payload.len() as usize;
    payload.copy_into_slice(&mut payload_bytes[..payload_len]);
    let signature_raw = signing_key.sign(&payload_bytes[..payload_len]);
    let signature = BytesN::from_array(&f.env, &signature_raw.to_bytes());

    // First submission succeeds
    let _uid = f
        .att
        .attest_delegated(&f.issuer, &input, &deadline, &signature);

    // Replay submission with same signature must panic as nonce advanced
    f.att
        .attest_delegated(&f.issuer, &input, &deadline, &signature);
}

#[test]
fn offchain_revocation_lifecycle() {
    let f = setup();
    let hash = BytesN::from_array(&f.env, &[88u8; 32]);

    assert!(!f.att.is_revoked_offchain(&hash));
    assert_eq!(f.att.get_offchain_revocation(&hash), None);

    f.env.ledger().set_timestamp(42_000);
    f.att.revoke_offchain(&f.issuer, &hash);

    // Event emitted immediately
    let events = f.env.events().all();
    let contract_events = events.filter_by_contract(&f.att.address);
    assert_eq!(contract_events.events().len(), 1);

    assert!(f.att.is_revoked_offchain(&hash));
    assert_eq!(f.att.get_offchain_revocation(&hash), Some(42_000));

    // Double revocation fails
    let res = f.att.try_revoke_offchain(&f.issuer, &hash);
    assert_eq!(res, Err(Ok(Error::AlreadyRevoked)));
}

#[test]
fn verify_offchain_valid_signature_and_revocation_states() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let f = setup();
    let hash = BytesN::from_array(&f.env, &[71u8; 32]);
    let fake_sig = BytesN::from_array(&f.env, &[0u8; 64]);

    // 1. Unconfigured signer returns SignerNotConfigured
    let res = f.att.try_verify_offchain(&f.issuer, &hash, &fake_sig);
    assert_eq!(res, Err(Ok(Error::SignerNotConfigured)));

    // 2. Configure signer
    let signing_key = SigningKey::from_bytes(&[55u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signer = BytesN::from_array(&f.env, &verifying_key.to_bytes());
    f.att.set_authorized_signer(&f.issuer, &signer);

    // 3. Sign the 32-byte payload hash
    let sig_raw = signing_key.sign(&hash.to_array());
    let sig = BytesN::from_array(&f.env, &sig_raw.to_bytes());

    // 4. Verify valid unrevoked signature returns Ok(true)
    let res = f.att.verify_offchain(&f.issuer, &hash, &sig);
    assert!(res);

    // 5. Revoke the hash on-chain
    f.att.revoke_offchain(&f.issuer, &hash);

    // 6. Verify revoked hash returns Ok(false)
    let res = f.att.verify_offchain(&f.issuer, &hash, &sig);
    assert!(!res);
}

#[test]
#[should_panic]
fn verify_offchain_invalid_signature_panics() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let f = setup();
    let hash = BytesN::from_array(&f.env, &[71u8; 32]);

    let signing_key = SigningKey::from_bytes(&[55u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let signer = BytesN::from_array(&f.env, &verifying_key.to_bytes());
    f.att.set_authorized_signer(&f.issuer, &signer);

    let wrong_key = SigningKey::from_bytes(&[66u8; 32]);
    let sig_raw = wrong_key.sign(&hash.to_array());
    let bad_sig = BytesN::from_array(&f.env, &sig_raw.to_bytes());

    let _ = f.att.verify_offchain(&f.issuer, &hash, &bad_sig);
}

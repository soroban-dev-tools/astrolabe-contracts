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

//! Resolver interface for Astrolabe schemas.
//!
//! Schemas may optionally designate a resolver contract address.
//! When configured, the attestation contract invokes:
//! - `on_attest` during attestation creation.
//! - `on_revoke` during revocation.
//!
//! If either callback returns `false`, the operation reverts with `Error::ResolverRejected`.

use soroban_sdk::{contractclient, Env};

use crate::Attestation;

#[contractclient(name = "ResolverClient")]
pub trait ResolverInterface {
    /// Hook executed on attestation issuance. Returns true to approve, false to reject.
    fn on_attest(env: Env, attestation: Attestation) -> bool;

    /// Hook executed on attestation revocation. Returns true to approve, false to reject.
    fn on_revoke(env: Env, attestation: Attestation) -> bool;
}

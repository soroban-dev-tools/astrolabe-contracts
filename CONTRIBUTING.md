# Contributing to astrolabe-contracts

Welcome. This repository holds the on-chain protocol for Astrolabe, an open
attestation registry on Stellar. You do not need to have used Stellar before to
contribute here. This guide takes you from a cold clone to a useful pull request in
one sitting.

## What this repository is, and the other two

Astrolabe lets anyone define a schema, issue a signed claim about an address, and
verify or revoke it on chain. It ships as three repositories:

- **astrolabe-contracts** (you are here) — the Rust/Soroban contracts, deploy and
  seed scripts, and the committed Testnet contract ids. This is the protocol and
  the deployment truth.
- **[astrolabe-sdk](https://github.com/soroban-dev-tools/astrolabe-sdk)** — the
  TypeScript client published to npm.
- **[astrolabe-explorer](https://github.com/soroban-dev-tools/astrolabe-explorer)**
  — the Next.js web app.

Dependencies point one way and never reverse:
**contracts → SDK → explorer**. This repository depends on nothing else in the
project; it builds and tests with no knowledge of the SDK or the app.

## The domain in two minutes

Four concepts are enough to start:

- **Schema** — a named, typed, immutable description of what an attestation's data
  means, for example `"string name,string country"`. Registered once, referenced
  forever by its `schema_uid`.
- **Attestation** — one claim made under a schema: an issuer, a subject, some data,
  an optional expiry, and whether it can be revoked. Identified by an
  `attestation_uid`.
- **Authority** — the address that registers a schema. The uid is derived from it,
  so schema names are namespaced per authority; there is no global name ownership.
- **Issuer** — the address that creates an attestation and the only one that may
  revoke it.

The normative specification is [`docs/protocol.md`](docs/protocol.md). It is
authoritative: when the code and the spec disagree, the spec wins and the code is
a bug. Read it before changing contract behaviour. The threat model is in
[`docs/threat-model.md`](docs/threat-model.md).

## Repository map

```
contracts/
  schema-registry/       schema registry contract
    src/lib.rs           contract code
    src/test.rs          unit tests
  attestation/           attestation registry contract
    src/lib.rs           contract code
    src/test.rs          unit tests
scripts/
  deploy_testnet.sh      build + deploy both contracts, write deployments file
  seed.sh                register the five seed schemas
deployments/
  testnet.json           committed Testnet contract ids (deployment truth)
docs/
  protocol.md            normative protocol specification
  threat-model.md        assets, threats, mitigations
  design-decisions.md    why the protocol is shaped the way it is
  multi-repo.md          the cross-repository contract
```

Nothing in this repository is generated; every file is edited by hand. The WASM
artifacts under `target/` are build output and are not committed.

## Getting set up

Prerequisites, with versions:

- **Rust** 1.84 or newer (`rustc --version`). Install from https://rustup.rs.
- The **wasm32v1-none** target: `rustup target add wasm32v1-none`.
- **Stellar CLI** 22 or newer (`stellar --version`). Install with
  `cargo install --locked stellar-cli` or see the Stellar docs.

Clone and prove the setup works. No deployment of your own is needed for this; the
tests run entirely in a local sandbox:

```bash
git clone https://github.com/soroban-dev-tools/astrolabe-contracts
cd astrolabe-contracts
rustup target add wasm32v1-none
cargo test --all
```

A passing test run means you are ready. To also build the on-chain artifacts:

```bash
cargo build --target wasm32v1-none --release
```

You only need a funded Testnet key if you want to deploy your own copy:

```bash
stellar keys generate astrolabe-deployer --network testnet --fund
scripts/deploy_testnet.sh                 # writes deployments/testnet.json
scripts/seed.sh                           # registers the five seed schemas
```

`--fund` calls the Testnet friendbot for you. Never commit a secret key; the
`stellar keys` store keeps them outside the repository.

## Where to start

Issues carry three difficulty labels: `good first issue`, `intermediate`, and
`advanced`, plus area labels. The unclaimed work below is ordered easiest to
hardest and mirrors [`ISSUES.md`](ISSUES.md), where each item has full acceptance
criteria.

1. **C1 — reject already-passed expirations** (`good first issue`). A small,
   self-contained validation with one new error code. Protocol section 3.4.
2. **C2 — validate `ref_uid` references** (`good first issue`). Confirm a linked
   attestation exists before storing the link.
3. **C3 — test and document `schema_attestation_count`** (`good first issue`).
   Fill a test and documentation gap.
4. **C4 — property tests for uid determinism** (`intermediate`). Pin the derivation
   in protocol sections 2.3 and 3.3 with tests.
5. **C5 — batch attest** (`intermediate`). All-or-nothing multi-attest sharing one
   authorisation.
6. **C6 — invoke the resolver on attest and revoke** (`intermediate`). Wire up the
   stored `resolver` extension point from PRD section 4.1.
7. **C7 — delegated attestation** (`advanced`). The single largest unclaimed piece:
   ed25519 over a canonical payload, with replay protection. PRD section 4.2.
8. **C8 — off-chain verification path** (`advanced`). PRD section 4.4; depends on
   C7's payload format.

To claim an issue, comment on it and a maintainer will assign it to you. For
anything labelled `advanced`, or any change to on-chain behaviour or storage
format, open a GitHub Discussion with a short proposal first and get one
maintainer sign-off before you start (see Community below).

## Rules that matter here

A reviewer will send a pull request back if it breaks any of these. Each is here
for a reason.

1. **Never break an existing on-chain format.** Storage keys, the `Schema` and
   `Attestation` layouts, uid derivation, event topics and fields, and error codes
   are a public contract. Deployed data and downstream indexers depend on them.
   Add new variants; do not renumber or repurpose existing ones.
2. **Every function that changes state needs an unauthorised-path test.** For each
   new entry point, prove that the wrong caller is rejected, not only that the
   right caller succeeds. `attest` and `revoke` already have these; keep the
   pattern.
3. **Nothing grows unbounded per address.** The per-subject index is capped at 100
   and per-schema state is a counter, not a list. Any new index must be bounded,
   and full history belongs in the off-chain indexer, not in contract state.
4. **Bound every caller-supplied input.** `definition` is capped at 512 bytes and
   `data` at 1024. New inputs that land in storage need a documented limit.
5. **No `unsafe`, and no new dependency without justification.** If a change needs a
   new crate, say why in the commit message. Keep the contracts small.

## Code style

- **Language:** Rust, edition 2021, `#![no_std]` contracts on `soroban-sdk`.
- **Formatter:** `rustfmt`. Run `cargo fmt --all` before committing.
- **Linter:** Clippy with warnings denied.
- **Tests:** the built-in test harness with `soroban-sdk` testutils.
- **Commits:** [Conventional Commits](https://www.conventionalcommits.org):
  `feat:`, `fix:`, `docs:`, `test:`, `chore:`. Keep the subject imperative and
  under about 70 characters.
- **Branches:** `type/short-description`, for example `feat/batch-attest`.

The exact commands CI runs, which you should run locally first:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all
cargo build --target wasm32v1-none --release
```

## Pull request checklist

- [ ] `cargo fmt --all --check` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` is clean.
- [ ] `cargo test --all` passes, including a new unauthorised-path test for any new
      state-changing function.
- [ ] `docs/protocol.md` updated if behaviour, storage, events, or error codes
      changed, and `docs/threat-model.md` updated if an authorisation check changed.
- [ ] No change to `deployments/testnet.json` in a feature pull request.
- [ ] Commits follow Conventional Commits.
- [ ] If this depends on or enables a change in `astrolabe-sdk` or
      `astrolabe-explorer`, link that pull request in the description and state the
      dependency direction.

## Releases

Maintainers cut releases. A release is a git tag such as `v0.1.0`. Tagging builds
the WASM, attaches each artifact and its sha256 to a GitHub release, and marks the
deployment recorded in `deployments/testnet.json`. Contributors must not edit
`deployments/testnet.json` or bump the version in a feature pull request; deploying
and versioning are a maintainer step, done in dependency order across the three
repositories as described in [`docs/multi-repo.md`](docs/multi-repo.md).

## Security

Report vulnerabilities privately as described in [`SECURITY.md`](SECURITY.md),
never in a public issue. The sensitive surfaces in this repository are the
authorisation checks in `attest` and `revoke`, schema immutability and uid
derivation, and the state-growth bounds. Astrolabe is unaudited and Testnet only.

## Community

Design discussion happens in this repository's GitHub Discussions. Protocol changes
need a written proposal there and one maintainer sign-off before implementation
begins. Two merged pull requests earn triage rights on request. Commit rights are
granted per repository; because this is the protocol repository, they are granted
slowly and only after sustained contract-review work. Small pull requests get
reviewed faster than large ones. Be kind, be specific, and assume good faith.

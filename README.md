# astrolabe-contracts

The on-chain protocol for **Astrolabe**, an open attestation registry on Stellar's
Soroban platform. An attestation is a signed claim: an issuer says some data about
a subject, under a named schema, optionally until an expiry, optionally revocable.
Applications read attestations instead of maintaining their own private allowlists.

This repository owns the **deployment truth**. It contains two Soroban contracts,
the scripts that deploy and seed them, and the committed Testnet contract ids.

> **Status:** unaudited, Testnet only. Do not deploy to Mainnet.

## The three repositories

Astrolabe ships as three repositories. Dependencies point one way and never
reverse:

```
astrolabe-contracts  ->  astrolabe-sdk  ->  astrolabe-explorer
```

- **astrolabe-contracts** (this repo) — Rust, Soroban. The protocol.
- **astrolabe-sdk** — TypeScript client, published to npm.
- **astrolabe-explorer** — Next.js web app.

## Contracts

- **schema-registry** — register an immutable, named, typed schema. Returns a
  `schema_uid` derived from the name, definition, and authority.
- **attestation** — `attest`, `revoke`, `get_attestation`, `is_valid`, and a capped
  per-subject index. Depends on the schema registry to confirm schemas exist.

The normative specification is [`docs/protocol.md`](docs/protocol.md). The threat
model is [`docs/threat-model.md`](docs/threat-model.md).

## Testnet deployment

Current Testnet deployment lives in [`deployments/testnet.json`](deployments/testnet.json):

| Contract        | Contract id |
|-----------------|-------------|
| schema_registry | `CAYUJL5VPIVDBAGXAQ3FTOUJ4LICWV6IFKIMTRPAJB7ZS5FMRRI3M5EC` |
| attestation     | `CB2773G3PKQVSECFXTM2Q65W2XJLMWGQXC5UTFBQEPHYPAGUEXEYBMW4` |

Five seed schemas are registered: `verified_merchant`, `verified_ngo`, `kyc_tier`,
`agent_job_completed`, `contributor_badge`. Their uids are in the deployment file.

## Build and test

```bash
rustup target add wasm32v1-none
cargo test --all                              # unit tests
cargo fmt --all --check                       # formatting
cargo clippy --all-targets -- -D warnings     # lints
cargo build --target wasm32v1-none --release  # WASM artifacts
```

The WASM artifacts land in `target/wasm32v1-none/release/`.

## Deploy your own

```bash
# One-time: a funded Testnet identity
stellar keys generate astrolabe-deployer --network testnet --fund

# Deploy both contracts and write deployments/testnet.json
scripts/deploy_testnet.sh

# Register the five seed schemas
scripts/seed.sh
```

Never commit a secret key. The `stellar keys` store keeps them outside the repo.

## Contributing

New here and never used Stellar? Start with [`CONTRIBUTING.md`](CONTRIBUTING.md) and
the open work in [`ISSUES.md`](ISSUES.md). Cross-repository rules are in
[`docs/multi-repo.md`](docs/multi-repo.md).

## License

Apache-2.0. See [`LICENSE`](LICENSE).

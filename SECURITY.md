# Security Policy

## Reporting a vulnerability

Report vulnerabilities privately by email to **security@soroban-dev-tools.org**.
Do not open a public issue for a security problem.

Include, as far as you can: the contract and function affected, a description of
the impact, and a way to reproduce it (a failing test or a transaction hash on
Testnet is ideal). We will acknowledge your report and keep you updated as we
investigate. Please give us reasonable time to ship a fix before any public
disclosure.

## Scope

This repository holds the on-chain protocol: the schema registry and attestation
contracts, their deploy and seed scripts, and the deployment records. Sensitive
surfaces here are:

- The authorisation checks in `attest` and `revoke` (see `docs/threat-model.md`,
  items T1 and T2).
- Schema immutability and uid derivation (T3, T4).
- The state-growth bounds on the per-subject index and input sizes (T5, T6).

Bugs in the TypeScript client belong in `astrolabe-sdk`; bugs in the web app
belong in `astrolabe-explorer`.

## Status

Astrolabe is **unaudited** and deployed to **Stellar Testnet only**. Do not rely
on it to protect anything of value, and do not deploy it to Mainnet, until the
README of this repository states that an audit has been completed.

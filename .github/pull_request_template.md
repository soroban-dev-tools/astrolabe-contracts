## Summary
<!-- Provide a clear, concise summary of the changes in this pull request -->

## Issue Reference
<!-- Reference the related issue (e.g. Fixes #12, Closes #4) -->

## Type of Change
- [ ] Bug fix (non-breaking change fixing an issue)
- [ ] New feature (non-breaking change adding functionality)
- [ ] Refactoring (code structure improvement without changing public behavior)
- [ ] Documentation update
- [ ] CI or tooling improvement

## Verification Checklist
- [ ] All workspace tests pass (`cargo test --all`).
- [ ] Code is formatted cleanly (`cargo fmt --all --check`).
- [ ] Clippy lints pass with zero warnings (`cargo clippy --all-targets -- -D warnings`).
- [ ] Release WASM bytecode compiles cleanly (`cargo build --target wasm32v1-none --release`).
- [ ] Documentation updated in `docs/protocol.md` and `docs/threat-model.md` if behavior changed.
- [ ] No breaking changes to existing on-chain storage layout or serialization format.

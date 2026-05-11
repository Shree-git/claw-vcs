## Summary

- 

## Scope

- [ ] Rust code
- [ ] docs
- [ ] tests
- [ ] release or packaging
- [ ] GitHub Actions

## Checks run

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] CI-only gates reviewed when relevant: rustdoc, dependency policy, CLI contracts, example smoke, deployment validation, SAST, release artifacts
- [ ] docs-only change, code checks not run

## Notes for reviewers

- Public interface changed: yes / no
- Migration needed: yes / no
- Deprecation policy applies: yes / no

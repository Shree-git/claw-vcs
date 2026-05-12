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

## Release, migration, and rollback

- Release note / changelog needed: yes / no
- Install, packaging, or artifact verification changed: yes / no
- Migration or compatibility impact: yes / no
- Rollback plan or operator recovery note:

## Security and supply chain

- Security-sensitive area touched: yes / no
- Dependency, license, SBOM, or cargo-vet impact: yes / no
- Public interface, policy, object format, or protocol changed: yes / no
- Required extra review evidence:

## Notes for reviewers

- Public interface changed: yes / no
- Migration needed: yes / no
- Deprecation policy applies: yes / no

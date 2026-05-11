# Unsafe Audit

As of v0.1 launch hardening, repository source under `crates/`, `tests/`, and `proto/` contains no direct Rust `unsafe` blocks.

Audit command:

```bash
rg -n '\bunsafe\b' crates tests proto Cargo.toml
```

If `unsafe` is introduced later, the PR must document:

- why safe Rust is insufficient
- which invariants callers must uphold
- focused tests around the unsafe boundary
- whether Miri, sanitizers, or `cargo geiger` should be added to CI

# Panic Audit

Production parsing and untrusted input paths should avoid `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.

Audit command:

```bash
rg -n 'unwrap\(|expect\(|panic!|todo!|unimplemented!' crates
cargo clippy --workspace --lib --bins --locked -- -D clippy::panic -D clippy::todo -D clippy::unimplemented
```

Guidance:

- tests may use `unwrap` and `expect`
- CLI setup code may use explicit panics only when failure is unrecoverable before user input
- library parsing, sync decoding, Git import, policy evaluation, and object loading should return typed errors
- production comments should describe current tracked limitations; avoid stale MVP/TODO wording
- crates that can support local public API docs should opt into `#![deny(missing_docs)]`
- CI and release quality gates run the production panic audit against library and binary targets
- each release should review new test-only panic sites before public announcement

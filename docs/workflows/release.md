# Release Workflow

Use this path before publishing a Claw release.

1. Update version and changelog.
2. Run `cargo fmt --all -- --check`.
3. Run `cargo clippy --workspace --all-targets --locked -- -D warnings`.
4. Run `cargo test --workspace --all-targets --locked`.
5. Run `cargo audit` and `cargo deny check`.
6. Compile fuzz targets with `cargo check --manifest-path fuzz/Cargo.toml --bins --locked`.
7. Verify release artifacts, signatures, attestations, and SBOM.
8. Run install smoke tests from a clean environment.
9. Publish release notes with known limitations and rollback instructions.

See `RELEASING.md` and `docs/reference/release-checklist.md`.

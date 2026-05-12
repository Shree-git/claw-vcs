# Releasing Claw VCS

When release infrastructure is configured, this repo uses `cargo-dist` to build and publish:

- GitHub Release artifacts (archives, checksums)
- `claw-installer.sh` + `claw-installer.ps1`
- Windows `.msi`
- A Homebrew formula published to a tap repository
- artifact signatures, attestations, and SBOMs when the release workflows are enabled

## One-time setup

### 1) Create the Homebrew tap repo

Create a GitHub repo:

- `shree-git/homebrew-tap`

Make sure it has a default branch (for example, commit a README) so Actions can push updates.

### 2) Add GitHub Actions secret for the tap

In the `shree-git/claw-vcs` repo, add a secret:

- `HOMEBREW_TAP_TOKEN`: a token with the minimum write access required for `shree-git/homebrew-tap`

## Cutting a release

1. Bump the version in `Cargo.toml` (`[workspace.package].version`).
2. Update `CHANGELOG.md`.
3. Run local gates:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo audit --deny warnings
cargo deny check
cargo vet
```

4. Compile and smoke-test fuzz targets:

```bash
cargo check --manifest-path fuzz/Cargo.toml --bins --locked
cargo run --manifest-path fuzz/Cargo.toml --bin object_id_parse --locked -- -runs=1
```

5. Run a release dry-run if supported by the local `cargo-dist` version:

```bash
cargo dist plan
```

6. Commit the version and changelog update.
7. Create and push a git tag in the form `vX.Y.Z` (example: `v0.1.0`).

Pushing the tag triggers `.github/workflows/release.yml` which builds and publishes artifacts.

## Release verification

Before announcing a release, verify from a clean machine or container:

```bash
claw --version
claw doctor
mkdir /tmp/claw-demo
cd /tmp/claw-demo
claw init
claw status
```

Verify release assets:

- checksums match
- Cosign signatures verify
- GitHub artifact attestations verify with `gh attestation verify`
- SBOM is present and readable
- Homebrew formula installs the tagged version
- Windows MSI installs and adds `claw` to `PATH`
- `cargo install --git https://github.com/shree-git/claw-vcs.git --tag vX.Y.Z --package claw --locked` installs the tagged version

See [docs/security/verifying-releases.md](docs/security/verifying-releases.md).

## Rollback

Keep the previous release artifact and a verified repository backup available before promoting a new release.

Rollback procedure:

1. Stop the daemon.
2. Install the previous verified version.
3. Restore a verified backup if a migration changed `.claw/` state.
4. Run `claw admin preflight`.
5. Restart the daemon.
6. Verify refs, object store health, and client sync.

See [docs/operations/upgrade-and-rollback.md](docs/operations/upgrade-and-rollback.md).

## WinGet (manual, first publish)

WinGet publishing is manual until the initial package is accepted into `microsoft/winget-pkgs`.

Suggested package identifier:

- `ShreeGit.ClawVCS`

High-level steps:

1. Ensure the GitHub Release includes the `.msi` asset.
2. Use `wingetcreate new` pointing at the MSI URL.
3. Submit the generated manifests as a PR to `microsoft/winget-pkgs`.

After the first acceptance, you can automate updates on each release tag.

# Release Checklist

Use this checklist for every tag release and hotfix.

## CI-Enforced Gates

- [ ] `release.yml` `quality` job passed (`cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`).
- [ ] `release.yml` `security-audit-gate` passed (`cargo audit`).
- [ ] `release.yml` `contract-tests-gate` passed (`claw`, `spec_tests`, `ops_artifacts_tests`).
- [ ] `release.yml` `compatibility-matrix-gate` passed on Linux, macOS, and Windows.
- [ ] `cross-version-runtime.yml` `cross-version-runtime-integration` job passed (`cross_version_runtime_tests`).
- [ ] `contract-diff.yml` passed and uploaded `contract-diff-summary.json` as `contract-diff-summary` artifact.

## Signed Artifact Flow

- [ ] `release.yml` `host` job signed each release file with `cosign sign-blob`.
- [ ] For each artifact, sidecars exist in the release bundle: `<artifact>.sig` and `<artifact>.pem`.
- [ ] `verify-artifacts.yml` successfully verified all artifact/signature/certificate pairs.

## Recommended Operator Checks (Not CI-Enforced)

- [ ] Rollback tested in staging with a known restore point.
- [ ] `soak-24h.yml` completed for a 24h run (scheduled or manual) and results reviewed.
- [ ] No unresolved Sev1/Sev2 incidents linked to the candidate.
- [ ] Latest `nightly-chaos.yml` run reviewed; follow-up recorded for any failures.
- [ ] Go/No-go recorded (Release Owner, Tech Lead, SRE), with release notes and rollback pointer published.

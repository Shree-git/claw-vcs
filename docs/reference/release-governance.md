# Release Governance

## Purpose
Define the current, implemented release gates and operator responsibilities for API/CLI/runtime contract safety.

## CI-Enforced

- **Contract diff gate (`contract-diff.yml`):**
  - Runs on PRs and pushes to `main`.
  - Publishes `contract-diff-summary.json` as artifact `contract-diff-summary`.
  - Fails when contract artifacts change without updates to release governance docs or `CHANGELOG.md`.
- **Release gates (`release.yml`, tag publish path):**
  - `quality` (fmt, clippy, workspace tests).
  - `security-audit-gate` (`cargo audit`).
  - `contract-tests-gate` (core and integration contract suites, including ops artifacts checks).
  - `compatibility-matrix-gate` on Linux, macOS, and Windows.
- **Cross-version runtime checks (`cross-version-runtime.yml`):**
  - Runs on PRs and pushes to `main`.
  - Runs `cross_version_runtime_tests` (`claw-integration-tests`).
- **Signed artifact flow (`release.yml` + `verify-artifacts.yml`):**
  - `release.yml` signs each release artifact with `cosign sign-blob`, producing `<artifact>.sig` and `<artifact>.pem`.
  - `verify-artifacts.yml` verifies signature and certificate sidecars with `cosign verify-blob` and fails on missing pairs.
- **Nightly drill (`nightly-chaos.yml`):**
  - Scheduled daily at `03:00 UTC`.
  - Runs deterministic failure drills via `chaos_tests`.
  - Runs an additional deterministic stress subset (`ops_artifacts_tests`) in serial mode.

## Recommended Operator Practice (Not CI-Enforced)

- Run/review `soak-24h.yml` for a full 24h soak before stable promotion.
- Run rollback drill in staging for each release candidate.
- Record go/no-go decision with Release Owner, Tech Lead, and SRE.
- Publish release notes with migration notes, known issues, and rollback reference.
- Review nightly chaos workflow results and track remediation tasks.

## No-Silent-Breakage Policy
- Any externally visible contract change must be explicit, reviewed, and documented before release.
- Silent behavior changes that break existing clients are prohibited.
- Breakage discovered after release triggers immediate rollback or mitigation and incident review.

## Contract-Change Rules
- **Contract surface:** Public API, CLI flags/options/output, config schema, event payloads, and integration protocols.
- **Required for any change:**
  - Machine-readable contract diff summary artifact (`contract-diff-summary.json`) produced by CI.
  - Compatibility impact classification: `compatible`, `conditionally compatible`, or `breaking`.
  - Migration guidance and versioning plan for `breaking` or `conditionally compatible` changes.
- **Breaking changes:**
  - Must be behind an announced deprecation window unless security or safety critical.
  - Require explicit sign-off from product owner and platform owner.
  - Must include rollback strategy validated in staging.

## Roles and Accountability
- **Release Owner:** Runs the release process, confirms checklist, records go/no-go.
- **Service Owners:** Validate service-level readiness and runbooks.
- **SRE/Security:** Confirm SLO posture, rollback readiness, and security scan status.

## Required Artifacts Per Release
- Completed release checklist.
- Contract diff summary artifact (`contract-diff-summary`).
- Compatibility matrix artifact present and valid (`docs/reference/compatibility-matrix.json`).
- Signed release artifacts (`<artifact>`, `<artifact>.sig`, `<artifact>.pem`) and verification record.
- Release notes with known issues and rollback point.

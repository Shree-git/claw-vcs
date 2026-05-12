# Public Launch Backlog Coverage

This file maps the public-launch hardening backlog to concrete artifacts in the repository or verified GitHub state. It is an audit aid, not a substitute for the live launch checklist.

Status key:

- **Implemented**: the repository contains code, docs, tests, workflows, or assets for the item.
- **Verified**: external repository state was checked and recorded.
- **External pending**: the remaining work needs repository-owner, package-registry, release, trademark, or clean-machine access after this branch lands.
- **Not applicable**: optional item intentionally not configured yet.

## Completion Summary

The in-repository P0/P1/P2 hardening work is implemented on the launch-hardening branch. The goal is not complete until the external pending items below are finished: package/name reservation where required, trademark/domain/social-handle review, GitHub social preview upload, PR review/merge, hardened public release publication, public artifact attestation verification, and clean-environment release-channel verification.

## P0

| # | Status | Evidence |
|---:|---|---|
| 1 | Verified | Repository remote is `Shree-git/claw-vcs`; README title is `Claw VCS`; CLI binary remains `claw`. |
| 2 | Implemented | README contains the v0.1 experimental status banner. |
| 3 | Implemented | `.gitignore` excludes OS/editor junk, local state, logs, databases, and secrets; `.DS_Store` is not tracked. |
| 4 | Verified | `docs/operations/public-launch-checklist.md` records full-history gitleaks and trufflehog passes plus GitHub secret scanning/push protection. |
| 5 | Implemented | `.github/workflows/ci.yml` runs formatting, clippy, tests, examples, docs, fuzz compile, `cargo-deny`, and `cargo-vet`. |
| 6 | Verified | `docs/operations/public-launch-checklist.md` records `main` branch protection settings verified by GitHub API. |
| 7 | Implemented | Workflows use SHA-pinned actions; `rg "uses: .+@(v[0-9]\|main\|master\|latest)" .github/workflows` returns no tag-only uses. |
| 8 | Implemented | Workflows default to `contents: read`; write scopes are job-local for release, SAST, Scorecard, SBOM, and attestations. |
| 9 | Implemented | `.github/workflows/release.yml`, `.github/workflows/ci.yml`, `.github/workflows/sbom.yml`, and `.github/workflows/verify-artifacts.yml` cover artifact/SBOM attestations and verification. |
| 10 | External pending | `docs/operations/package-registry-strategy.md` and `docs/operations/install-verification-log.md` separate live/planned channels; clean-environment verification remains pending for the next hardened release. |
| 11 | Implemented | `docs/security/threat-model.md`. |
| 12 | Implemented | Visibility semantics are documented in `docs/reference/known-limitations.md`; policy code supports `EncryptedMetadataRequired`, recipient envelopes, authorized/revoked recipients, and the legacy `restricted` alias. |
| 13 | Implemented | `crates/claw-sync/src/event_service.rs` implements an internal event bus; limitations document notes polling only as compatibility fallback. |
| 14 | Implemented | `crates/claw-git/tests/git_bridge_real_git.rs` and CLI Git bridge tests exercise export/import with real Git commands and roundtrip paths. |
| 15 | Implemented | `docs/reference/known-limitations.md`. |

## P1: Contributor And User Readiness

| # | Status | Evidence |
|---:|---|---|
| 16 | Implemented | `.github/ISSUE_TEMPLATE/*` and `.github/PULL_REQUEST_TEMPLATE.md`. |
| 17 | Implemented | `.github/CODEOWNERS`. |
| 18 | Implemented | `CHANGELOG.md`. |
| 19 | Implemented | `SUPPORT.md`. |
| 20 | Implemented | `ROADMAP.md`. |
| 21 | Implemented | ADRs in `docs/adr/` cover no staging area, BLAKE3 IDs, Protobuf+COF, gRPC sync, intent/change/revision, capsules, and policy objects. |
| 22 | Implemented | `docs/spec/object-format.md`. |
| 23 | Implemented | `tests/vectors/` includes COF, IDs, capsules, policies, patch, and fail-closed vectors. |
| 24 | Implemented | `fuzz/fuzz_targets/` covers COF decode, object IDs, patch apply/codecs, JSON tree merge, Git import parsing, sync chunks, capsules, policy checks, and store objects. |
| 25 | Implemented | Property tests live in `crates/*/tests/*props.rs`, `crates/claw-core/tests/serialization_props.rs`, and policy/crypto tamper tests. |
| 26 | Implemented | Git interop tests live in `crates/claw-git/tests/git_bridge_real_git.rs`, `tests/integration/spec_tests.rs`, and CLI Git workflow tests. |
| 27 | Implemented | Durability/crash coverage appears in `tests/integration/chaos_tests.rs`, `tests/integration/backlog_gap_tests.rs`, store corruption tests, and admin backup/rollback tests. |
| 28 | Implemented | `.github/dependabot.yml` and `.github/workflows/dependency-review.yml`; Dependabot security updates verified enabled. |
| 29 | Implemented | `deny.toml`; CI runs `cargo deny check`. |
| 30 | Implemented | `supply-chain/{audits.toml,config.toml,imports.lock}`; CI runs `cargo vet`. |
| 31 | Implemented | `.github/workflows/sbom.yml` and release/CI SBOM attestation jobs; public release SBOM verification remains part of release-channel verification. |
| 32 | Implemented | `.github/workflows/scorecard.yml`; Scorecard checks pass on PR. |
| 33 | Implemented | `.github/workflows/codeql.yml` and `.github/workflows/semgrep.yml`; code scanning uploads verified. |
| 34 | Implemented | `docs/security/verifying-releases.md`. |
| 35 | Implemented | `docs/operations/upgrade-and-rollback.md`, `docs/runbooks/emergency-rollback.md`, and `docs/runbooks/backup-and-restore.md`. |

## P1: Product And Concept Clarity

| # | Status | Evidence |
|---:|---|---|
| 36 | Implemented | README uses complementary Git positioning instead of replacement language. |
| 37 | Implemented | `docs/concepts/object-model.md`, `docs/concepts/intent-change-revision.md`, `docs/concepts/capsules-and-evidence.md`, `docs/concepts/policies.md`, and `docs/cli/*.md`. |
| 38 | Implemented | README contains `What Claw VCS is not`. |
| 39 | Implemented | README and `docs/security/threat-model.md` define signed-claim limits and trust assumptions. |
| 40 | Implemented | Mermaid trust/adversary diagrams in `docs/security/threat-model.md` and `docs/concepts/claw-vs-attestations.md`. |
| 41 | Implemented | `docs/concepts/claw-vs-attestations.md`. |
| 42 | Implemented | `scripts/demo.sh` delegates to `examples/basic-demo/scripts/demo.sh`; CI runs the demo. |
| 43 | Implemented | `examples/demo-media/basic-demo.cast`, `status-screenshot.svg`, `ship-capsule-screenshot.svg`, and `command-gallery.svg`. |
| 44 | Implemented | `examples/demo-media/command-gallery.svg` covers status, log, diff, show, policy, failed integrate, successful ship, and Git export. |
| 45 | Implemented | `docs/migration/from-git.md`. |
| 46 | Implemented | `docs/workflows/solo.md`, `human-agent-pair.md`, `multiple-agents.md`, `policy-gated-integration.md`, `sensitive-paths.md`, and `release.md`. |
| 47 | Implemented | `docs/agents/integration-guide.md`, `agent-registration.md`, and `change-workflow.md`. |
| 48 | Implemented | `docs/agents/evidence-schema.md`. |
| 49 | Implemented | `docs/agents/evidence-freshness.md` plus freshness policy implementation/tests. |
| 50 | Implemented | `docs/agents/key-rotation-and-revocation.md` plus `claw agent keygen/register/rotate/revoke` support. |

## P1: CLI And Developer Experience

| # | Status | Evidence |
|---:|---|---|
| 51 | Implemented | `claw completions bash|zsh|fish`; smoke checked generated output. |
| 52 | Implemented | `claw doctor` and `claw doctor --json`; docs in `docs/cli/doctor.md`. |
| 53 | Implemented | `claw version --json`; docs in `docs/cli/version.md`. |
| 54 | Implemented | `status`, `log`, `show`, `policy eval`, `diff`, and `doctor` support JSON output; docs under `docs/cli/`. |
| 55 | Implemented | JSON/human diagnostics include remediation; `docs/cli/exit-codes.md` and `docs/reference/public-interface-manifest.md` define the contract. |
| 56 | Implemented | Aliases are limited and documented (`serve`, create/new forms, compatibility aliases where needed). |
| 57 | Implemented | Dry-run support for integrate, sync push, policy apply, git-import, and git-export; help/docs verified. |
| 58 | Implemented | `docs/cli/exit-codes.md` and `crates/claw/src/error.rs`. |
| 59 | Implemented | `claw init` prints next steps. |
| 60 | Implemented | Command docs exist in `docs/cli/` for init, intent, change, snapshot, ship, integrate, sync, git-export, and git-import with examples, JSON output, exit codes, and common errors. |

## P1: Daemon, Sync, And Security

| # | Status | Evidence |
|---:|---|---|
| 61 | Implemented | README and `docs/reference/production-profile-defaults.md` document production auth/TLS defaults. |
| 62 | Implemented | Auth/token redaction is covered by daemon/security tests and telemetry/support-bundle guidance. |
| 63 | Implemented | Sync request-size, stream, concurrency, retry, and rate-limit behavior is implemented in daemon/sync code and documented in security/production docs. |
| 64 | Implemented | `crates/claw-sync/src/security.rs` models roles/scopes; daemon docs describe authorization. |
| 65 | Implemented | Audit logging for sync/security/admin paths is implemented and documented in observability/security docs. |
| 66 | Implemented | mTLS flags/docs/tests exist for daemon/sync. |
| 67 | Implemented | Replay protection uses revision binding, nonce metadata, signer context, and tests in sync/capsule paths. |
| 68 | Implemented | `docs/reference/compatibility.md`, `compatibility-matrix.json`, and sync negotiation code. |
| 69 | Implemented | Remote compatibility/integration tests cover push/pull, partial clone, interruption, auth, TLS, stale token, and protocol mismatch. |
| 70 | Implemented | Recipient model for encrypted capsule fields is implemented in crypto/policy/CLI and documented in agent/security docs. |

## P1: Release And Packaging

| # | Status | Evidence |
|---:|---|---|
| 71 | Implemented | `docs/operations/release-reproducibility.md` and release workflow metadata/signing/attestation/SBOM gates. |
| 72 | Implemented | `RELEASING.md` includes fmt, clippy, test, audit, deny, vet, fuzz smoke, dry-run, install verification, signatures, attestations, SBOM, Homebrew, Windows, and rollback. |
| 73 | Implemented | `docs/operations/package-registry-strategy.md`. |
| 74 | External pending | Registry availability checks are recorded; actual reservation, domain/social handles, and trademark clearance require maintainer/account action. |
| 75 | Implemented | Install docs end with `claw --version`, `claw doctor`, and smoke test commands. |
| 76 | Implemented | README and release verification docs provide manual download and verification alternatives to pipe installers. |
| 77 | Implemented | `docs/operations/uninstall.md`. |

## P1: Code Quality And Maintainability

| # | Status | Evidence |
|---:|---|---|
| 78 | Implemented | Crate-level `//!` docs exist across workspace crates. |
| 79 | Implemented | Rustdoc examples exist for core, store, crypto, policy, and patch crates. |
| 80 | Implemented | `#![deny(missing_docs)]` is enabled on public library crates including core, crypto, policy, and patch. |
| 81 | Implemented | MVP/TODO audit recorded in `docs/reference/panic-audit.md`; stale production markers were removed or documented. |
| 82 | Implemented | Criterion benchmark scaffolds and docs in `crates/*/benches/` and `docs/reference/benchmarks.md`. |
| 83 | Implemented | Large-repo scenarios are covered in `tests/integration/backlog_gap_tests.rs` and benchmark fixtures. |
| 84 | Implemented | Windows path/release-channel coverage is in compatibility matrix CI and path safety tests; broader real-world validation remains called out before rollout. |
| 85 | Implemented | `docs/reference/unsafe-audit.md` documents unsafe audit status. |
| 86 | Implemented | Panic audit guidance and CI clippy panic/todo/unimplemented checks. |
| 87 | Implemented | Concurrency and load/soak tests cover multi-agent daemon writes, reads, pushes, refs, and overload behavior. |
| 88 | Implemented | COF/object corruption tests in `crates/claw-core/tests/cof_corruption.rs`, `crates/claw-store/tests/store_props.rs`, and integration chaos tests. |
| 89 | Implemented | Object-format migration framework and config compatibility checks are documented/tested. |
| 90 | Implemented | `docs/reference/compatibility.md` and `docs/reference/compatibility-matrix.json`. |

## P2: Polish, Community, And Adoption

| # | Status | Evidence |
|---:|---|---|
| 91 | Implemented | `docs/index.html` and `docs/landing-page.md`. |
| 92 | Implemented | README includes restrained CI/license/security badges. |
| 93 | Implemented | `docs/assets/social-preview.png` upload-ready asset and source SVG. |
| 94 | External pending | Logo overinvestment intentionally deferred until name/trademark clearance. |
| 95 | Verified | Repository topics verified with `gh repo view` and recorded in `public-launch-checklist.md`. |
| 96 | Implemented | `examples/basic-human-workflow`, `agent-capsule`, `policy-gated-integration`, `git-roundtrip`, and `sensitive-path`. |
| 97 | Implemented | Persona docs under `docs/persona/`. |
| 98 | Implemented | `docs/maintainers/guide.md` and related maintainer docs. |
| 99 | Implemented | `docs/maintainers/governance.md`. |
| 100 | Not applicable | Funding file is optional; no funding channel is configured unless maintainers choose one. |
| 101 | Implemented | `.github/labels.yml`; live GitHub labels verified with `gh label list`. |
| 102 | Implemented | `docs/reference/stability.md`. |
| 103 | Implemented | `docs/reference/deprecation-policy.md` and `docs/maintainers/deprecations.md`. |
| 104 | Implemented | `docs/reference/telemetry.md` and `docs/maintainers/telemetry.md`. |
| 105 | Implemented | ClawLab/hosted remote references are marked planned in README, CLI docs, compatibility docs, known limitations, and telemetry policy. |
| 106 | Implemented | `docs/reference/data-layout.md`. |
| 107 | Implemented | `examples/backup-restore/` and backup/restore runbook. |
| 108 | Implemented | Disaster recovery and backup/rollback tests in `tests/integration/backlog_gap_tests.rs` plus CI example smoke. |
| 109 | Implemented | `examples/policy-gated-integration/scripts/failure-cases.sh` and workflow docs show missing evidence, sensitive paths, stale evidence, trust, and signer failures. |
| 110 | Implemented | `docs/concepts/agent-honesty-is-not-enough.md`. |

## Current External Blockers

- PR #4 requires review approval before merge.
- Package/name reservation, trademark review, domain/social-handle checks, and GitHub social preview upload require maintainer/account access.
- The next hardened public release must be cut before public artifact attestations, SBOMs, signatures, installers, Homebrew, MSI, and clean-environment channel checks can be verified.
- GitHub reports low Dependabot findings on the default branch until this branch's dependency updates land on `main`.

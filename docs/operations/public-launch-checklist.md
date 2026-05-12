# Public Launch Checklist

Some items require repository owner access or external account access and cannot be completed by editing files in this repository alone.

Backlog item-by-item coverage is tracked in [backlog-coverage.md](backlog-coverage.md).
Owner-only launch blockers are tracked in
[GitHub issue #5](https://github.com/Shree-git/claw-vcs/issues/5).

Status as of 2026-05-12:

- GitHub repository: `Shree-git/claw-vcs`.
- Secret scanning, push protection, and Dependabot security updates are enabled.
  Open Dependabot alerts are launch-gated by `scripts/public-launch-preflight.sh`.
- Full-history local secret scans passed on 2026-05-12 for the
  `codex/public-launch-hardening` branch history:
  `gitleaks detect --source . --no-git=false --redact --no-banner` reported no
  leaks, and `trufflehog git file://$PWD --json --no-update` reported 0
  verified and 0 unverified secrets. Re-run both commands after the final
  launch commit before announcement.
- Code scanning uploads are accepted for PR #4 on 2026-05-12:
  CodeQL, Semgrep OSS, and Scorecard analyses exist for `refs/pull/4/merge`.
- PR #4 has passing CI, release planning, security, SAST,
  dependency-review, compatibility, contract, and deploy-validation checks.
  GitHub still reports `REVIEW_REQUIRED`, so merge is blocked only by
  branch-protection review.
- `main` branch protection is enabled with required reviews, code-owner review, stale approval dismissal, required checks, conversation resolution, signed commits, no force pushes, and no deletions.
- `main` branch protection was verified with the GitHub API on 2026-05-12.
- Repository topics were verified with `gh repo view` on 2026-05-12:
  `ai-agents`, `cli`, `developer-tools`, `provenance`, `rust`,
  `version-control`, `git`, `sigstore`, `slsa`, `supply-chain-security`, and
  `vcs`.
- Package-name checks on 2026-05-12:
  `claw-vcs` and the `claw-vcs-*` internal package names were not published on
  crates.io, `claw`, `claw-core`, `claw-crypto`, and `claw-sync` were occupied
  by unrelated crates, the planned WinGet manifest path
  `ShreeGit.ClawVCS` was absent from `microsoft/winget-pkgs`, and
  `Formula/claw.rb` exists in `Shree-git/homebrew-tap`.
- `scripts/publish-cratesio.sh --dry-run` was run on 2026-05-12.
  `claw-vcs-core` packaged and verified successfully. The remaining internal
  crates were intentionally skipped because their `claw-vcs-*` registry
  dependencies cannot resolve until the first real publish sequence begins.
- Maintainer preflight on 2026-05-12 passed repository identity, visibility,
  topics, security settings, branch protection, signed commits, Homebrew tap
  presence, and social preview asset checks. It still blocks launch on open
  Dependabot alerts reported against `main` until this PR's patched lockfile
  lands, and on unreserved crates.io identities for the `claw-vcs` package set.
  It warns that WinGet, GitHub social preview upload, optional GitHub Pages
  publication, trademark, domain, and social-handle review require maintainer
  action.
- Suggested repository labels are tracked in `.github/labels.yml`.
- PR #4 review conversations have been replied to and resolved. The PR remains
  blocked by the required independent approval.
- Remaining external checks: package/name reservation where required, trademark/domain/social-handle review, social preview upload, optional GitHub Pages publication, launch-hardening release publication, and clean-environment verification for each release channel after the hardened changes are published.
  These are tracked in [issue #5](https://github.com/Shree-git/claw-vcs/issues/5).

Before announcement, run the maintainer preflight from an authenticated local checkout:

```bash
scripts/public-launch-preflight.sh
CLAW_PREFLIGHT_STRICT=1 scripts/public-launch-preflight.sh
```

The normal preflight reports launch blockers that are still pending. Strict
mode is the broad-announcement gate: it fails until there are no open
Dependabot alerts, the `claw-vcs` crates.io package set is reserved or
published, the GitHub social preview is uploaded, and completed
name/domain/social/package evidence is recorded.

## Owner-Only Launch Handoff

These steps require repository owner, package registry, release, or account
access; they cannot be completed by editing this repository alone.

1. Review and merge PR #4 after the required approval is recorded.
   Confirm the low `rand` Dependabot alerts close after the patched lockfile is
   on `main`.
2. Reserve or publish the `claw-vcs` crates.io package set before documenting a
   crates.io install path. Publish internal packages in the order documented in
   `docs/operations/package-registry-strategy.md`, using
   `CLAW_CRATESIO_EXPECTED_OWNER=<owner> CLAW_CRATESIO_PUBLISH=1 scripts/publish-cratesio.sh --publish`
   from the exact release tag once credentials are configured.
3. Complete trademark, domain, and social-handle checks before treating the name
   and permanent visual identity as launch-ready. Use
   [name-clearance.md](name-clearance.md) and
   [name-clearance-evidence.template.md](name-clearance-evidence.template.md)
   to record evidence in `docs/operations/name-clearance-evidence.md`, or set
   `CLAW_PREFLIGHT_NAME_EVIDENCE` to the evidence file used by strict
   preflight.
4. Upload `docs/assets/social-preview.png` as the GitHub social preview.
5. If the launch includes a public website, enable GitHub Pages or another docs
   host for `docs/index.html`. After this PR lands on `main`, the manual
   `Pages` workflow can publish the committed `docs/` site.
6. Cut the launch-hardening release tag, then verify the published release
   artifacts:

```bash
scripts/public-launch-preflight.sh
CLAW_RELEASE_VERIFY_REPORT=release-verification/<launch-tag>.json scripts/verify-release-channel.sh <launch-tag>
```

7. Record clean-environment verification results for every live install channel
   in [install-verification-log.md](install-verification-log.md). Prefer the
   JSON reports uploaded by `release-channel-smoke.yml` over pasted terminal
   summaries.

## Repository Identity

- [x] Rename the GitHub repository to `claw-vcs`.
- [x] Keep the binary and command name as `claw`.
- [ ] Reserve or verify `claw-vcs` where package registries need an unambiguous project name.
- [ ] Complete trademark/name clearance before investing in a permanent logo.
      Record evidence in [name-clearance.md](name-clearance.md).

## GitHub Repository Rules

For `main`, require:

- [x] pull request before merging
- [x] at least one approving review
- [x] stale approval dismissal
- [x] required status checks
- [x] conversation resolution
- [x] signed commits
- [x] no force pushes
- [x] no branch deletions
- [x] no bypassing except a documented emergency maintainer exception

## GitHub Security Settings

- [x] Enable GitHub secret scanning.
- [x] Run full-history `gitleaks detect --source . --no-git=false --redact`.
- [x] Run full-history `trufflehog git file://$PWD --json --no-update`.
- [x] Enable Dependabot security updates.
- [x] Confirm code scanning uploads are accepted for CodeQL, Semgrep, and Scorecard workflows.
- [x] Configure release artifact provenance attestations in `release.yml`.
- [ ] Confirm the next public release artifacts verify with `gh attestation verify --repo Shree-git/claw-vcs`.

Release/install verification evidence is tracked in [install-verification-log.md](install-verification-log.md).

## Release Channel Verification

On a clean Unix host, use the helper script for the archive, checksum,
signatures, provenance/SBOM attestations, SBOM readability, release metadata, shell installer, and tagged cargo
install path:

```bash
scripts/verify-release-channel.sh <launch-tag>
```

Before announcement, test each live channel from a clean environment:

- [ ] GitHub release archive, checksum, signatures, attestations, SBOM, and release metadata
- [ ] shell installer
- [ ] PowerShell installer
- [ ] Homebrew formula
- [ ] Windows MSI
- [ ] `cargo install --git`
- [ ] Docker/OCI image, if promoted from planned to live

Mark unavailable channels as planned or unsupported in release notes and docs.

## Repository Metadata

Suggested topics:

```text
version-control
vcs
provenance
ai-agents
supply-chain-security
cli
rust
git
sigstore
slsa
developer-tools
```

Suggested labels are tracked in [`.github/labels.yml`](../../.github/labels.yml).

## Landing Page

- [x] Add a static landing page artifact in `docs/index.html`.
- [x] Add a manual, SHA-pinned GitHub Pages deployment workflow for the
  committed `docs/` site.
- [ ] If the launch should include a public website, enable GitHub Pages or
  another docs host, run the manual deployment, and verify the rendered page.

## Social Preview

Suggested card copy:

```text
Claw VCS
Intent. Evidence. Provenance.
Version control for human + AI code.
```

Upload-ready asset:

- `docs/assets/social-preview.png` (1280x640 PNG, under 1 MB)

Source asset:

- `docs/assets/social-preview.svg`

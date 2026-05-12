# Public Launch Checklist

Some items require repository owner access or external account access and cannot be completed by editing files in this repository alone.

Backlog item-by-item coverage is tracked in [backlog-coverage.md](backlog-coverage.md).

Status as of 2026-05-12:

- GitHub repository: `Shree-git/claw-vcs`.
- Secret scanning, push protection, and Dependabot security updates are enabled.
- Full-history local secret scans passed on 2026-05-12:
  `gitleaks 8.30.0` scanned 93 commits and 2.19 MB with no leaks found, and
  `trufflehog 3.95.2` scanned 2,424 chunks and 2.27 MB with 0 verified and 0
  unverified secrets.
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
- Maintainer preflight on 2026-05-12 passed repository identity, visibility,
  topics, security settings, branch protection, signed commits, Homebrew tap
  presence, and social preview asset checks. It still blocks launch on
  unreserved crates.io identities for the `claw-vcs` package set and warns
  that WinGet, GitHub social preview upload, optional GitHub Pages publication,
  trademark, domain, and social-handle review require maintainer action.
- Suggested repository labels are tracked in `.github/labels.yml`.
- Remaining external checks: package/name reservation where required, trademark/domain/social-handle review, social preview upload, optional GitHub Pages publication, launch-hardening release publication, and clean-environment verification for each release channel after the hardened changes are published.

Before announcement, run the maintainer preflight from an authenticated local checkout:

```bash
scripts/public-launch-preflight.sh
```

## Owner-Only Launch Handoff

These steps require repository owner, package registry, release, or account
access; they cannot be completed by editing this repository alone.

1. Review and merge PR #4 after the required approval is recorded.
2. Reserve or publish the `claw-vcs` crates.io package set before documenting a
   crates.io install path. Publish internal packages in the order documented in
   `docs/operations/package-registry-strategy.md`, using
   `scripts/publish-cratesio.sh --publish` once credentials are configured.
3. Complete trademark, domain, and social-handle checks before treating the name
   and permanent visual identity as launch-ready.
4. Upload `docs/assets/social-preview.png` as the GitHub social preview.
5. If the launch includes a public website, enable GitHub Pages or another docs
   host for `docs/index.html` and verify the rendered page.
6. Cut the launch-hardening release tag, then verify the published release
   artifacts:

```bash
scripts/public-launch-preflight.sh
scripts/verify-release-channel.sh <launch-tag>
```

7. Record clean-environment verification results for every live install channel
   in [install-verification-log.md](install-verification-log.md).

## Repository Identity

- [x] Rename the GitHub repository to `claw-vcs`.
- [x] Keep the binary and command name as `claw`.
- [ ] Reserve or verify `claw-vcs` where package registries need an unambiguous project name.
- [ ] Complete trademark/name clearance before investing in a permanent logo.

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
signatures, attestations, SBOM readability, shell installer, and tagged cargo
install path:

```bash
scripts/verify-release-channel.sh <launch-tag>
```

Before announcement, test each live channel from a clean environment:

- [ ] GitHub release archive, checksum, signatures, attestations, and SBOM
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
rust
git
sigstore
slsa
developer-tools
```

Suggested labels are tracked in [`.github/labels.yml`](../../.github/labels.yml).

## Landing Page

- [x] Add a static landing page artifact in `docs/index.html`.
- [ ] If the launch should include a public website, enable and verify GitHub
  Pages or another docs host for the committed landing page.

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

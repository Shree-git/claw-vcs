# Public Launch Checklist

Some items require repository owner access or external account access and cannot be completed by editing files in this repository alone.

Backlog item-by-item coverage is tracked in [backlog-coverage.md](backlog-coverage.md).

Status as of 2026-05-11:

- GitHub repository: `Shree-git/claw-vcs`.
- Secret scanning, push protection, and Dependabot security updates are enabled.
- Full-history local secret scans passed on 2026-05-11:
  `gitleaks 8.30.0` scanned 51 commits with no leaks found, and
  `trufflehog 3.95.2` reported 0 verified and 0 unverified secrets.
- Code scanning uploads are accepted for PR #4 on 2026-05-11:
  CodeQL, Semgrep OSS, and Scorecard analyses exist for `refs/pull/4/merge`.
- `main` branch protection is enabled with required reviews, code-owner review, stale approval dismissal, required checks, conversation resolution, signed commits, no force pushes, and no deletions.
- `main` branch protection was verified with the GitHub API on 2026-05-11.
- Repository topics were verified with `gh repo view` on 2026-05-11:
  `ai-agents`, `cli`, `developer-tools`, `provenance`, `rust`,
  `version-control`, `git`, `sigstore`, `slsa`, `supply-chain-security`, and
  `vcs`.
- Package-name checks on 2026-05-11:
  `claw-vcs` was not published on crates.io, `claw` was occupied by an
  unrelated crates.io crate, the planned WinGet manifest path
  `ShreeGit.ClawVCS` was absent from `microsoft/winget-pkgs`, and
  `Formula/claw.rb` exists in `Shree-git/homebrew-tap`.
- Suggested repository labels are tracked in `.github/labels.yml`.
- Remaining external checks: package/name reservation where required, trademark/domain/social-handle review, social preview upload, public release attestations, and clean-environment verification for each release channel after the hardened changes are published.

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

Before announcement, test each live channel from a clean environment:

- [ ] GitHub release archive
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

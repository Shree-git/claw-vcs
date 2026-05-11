# Public Launch Checklist

Some items require repository owner access or external account access and cannot be completed by editing files in this repository alone.

Status as of 2026-05-11:

- GitHub repository: `Shree-git/claw-vcs`.
- Secret scanning, push protection, and Dependabot security updates are enabled.
- Full-history local secret scans passed on 2026-05-11:
  `gitleaks 8.30.0` scanned 51 commits with no leaks found, and
  `trufflehog 3.95.2` reported 0 verified and 0 unverified secrets.
- Code scanning uploads are accepted for PR #4 on 2026-05-11:
  CodeQL, Semgrep OSS, and Scorecard analyses exist for `refs/pull/4/merge`.
- `main` branch protection is enabled with required reviews, code-owner review, stale approval dismissal, required checks, conversation resolution, signed commits, no force pushes, and no deletions.
- Suggested repository topics and labels are configured.
- Remaining external checks: package/name reservation, trademark/domain/social-handle review, social preview upload, public release attestations, and clean-environment verification for each release channel after the hardened changes are published.

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
- [ ] Confirm artifact attestations are enabled for public release artifacts.

Release/install verification evidence is tracked in [install-verification-log.md](install-verification-log.md).

## Release Channel Verification

Before announcement, test each live channel from a clean environment:

- [ ] GitHub release archive
- [ ] shell installer
- [ ] PowerShell installer
- [ ] Homebrew formula
- [ ] Windows MSI
- [ ] `cargo install --git`

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

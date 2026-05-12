# Package Registry Strategy

Claw VCS uses `claw` as the CLI name and `claw-vcs` as the public repository/product package identity where a longer name is needed.

| Channel | Status | Notes |
|---|---|---|
| GitHub Releases | live | `v0.1.0` exists with archives, checksums, signatures, installers, and MSI. A new launch-hardening release still needs clean-environment verification. |
| Homebrew | live | Formula exists as `Formula/claw.rb` in `shree-git/homebrew-tap`; Homebrew exposes that repository as tap `shree-git/tap`, so install with `brew install shree-git/tap/claw`. Verify formula after it points at the launch-hardening release. Homebrew core did not have a `claw` formula in the local `brew info claw` check on 2026-05-11. |
| crates.io | planned | Checked on 2026-05-11: `claw-vcs` returned 404 from the crates.io crate API, while `claw` is occupied by an unrelated crate. Reserve `claw-vcs` before documenting a crates.io install path. |
| WinGet | planned | Planned package id: `ShreeGit.ClawVCS`; checked on 2026-05-11 and no manifest path exists in `microsoft/winget-pkgs`. First publish requires manual PR to `microsoft/winget-pkgs`. |
| Windows MSI | live | `v0.1.0` MSI exists; verify on Windows for the launch-hardening release. |
| Shell installer | live | `v0.1.0` shell installer exists; keep non-pipe manual download path documented and verify the launch-hardening release. |
| PowerShell installer | live | `v0.1.0` PowerShell installer exists; keep non-pipe manual download path documented and verify on Windows. |
| Scoop | unsupported | Revisit after first stable Windows release. |
| Nix | unsupported | Prefer source build or manual archive install for now. |
| AUR | unsupported | Revisit after Linux adoption demand. |
| Docker image | planned | Docker, Helm, Terraform, and systemd deployment assets exist and are validated by CI, but no public OCI image is a launch channel until it is built, signed, attested, pushed, and clean-environment verified. |

Before broad announcement, verify every live channel from a clean machine or container and record expected command output in release notes.

Latest preflight result on 2026-05-12: repository, branch-protection,
security-setting, Homebrew tap, and social-preview asset checks passed;
`claw-vcs` was still unreserved on crates.io, and WinGet remained planned.

Maintainer preflight:

```bash
scripts/public-launch-preflight.sh
```

The preflight checks package-name signals, repository settings, branch protection,
and the local social preview asset. It is expected to fail until launch-gating
external actions, such as reserving or publishing `claw-vcs` on crates.io, are
complete.

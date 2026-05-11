# Package Registry Strategy

Claw VCS uses `claw` as the CLI name and `claw-vcs` as the public repository/product package identity where a longer name is needed.

| Channel | Status | Notes |
|---|---|---|
| GitHub Releases | live | `v0.1.0` exists with archives, checksums, signatures, installers, and MSI. A new launch-hardening release still needs clean-environment verification. |
| Homebrew | live | Formula exists in `shree-git/homebrew-tap`; verify formula after it points at the launch-hardening release. |
| crates.io | planned | Reserve `claw-vcs` if available; avoid implying `cargo install claw` until published. |
| WinGet | planned | Planned package id: `ShreeGit.ClawVCS`; first publish requires manual PR to `microsoft/winget-pkgs`. |
| Windows MSI | live | `v0.1.0` MSI exists; verify on Windows for the launch-hardening release. |
| Shell installer | live | `v0.1.0` shell installer exists; keep non-pipe manual download path documented and verify the launch-hardening release. |
| PowerShell installer | live | `v0.1.0` PowerShell installer exists; keep non-pipe manual download path documented and verify on Windows. |
| Scoop | unsupported | Revisit after first stable Windows release. |
| Nix | unsupported | Prefer source build or manual archive install for now. |
| AUR | unsupported | Revisit after Linux adoption demand. |
| Docker image | planned | Docker, Helm, Terraform, and systemd deployment assets exist and are validated by CI, but no public OCI image is a launch channel until it is built, signed, attested, pushed, and clean-environment verified. |

Before broad announcement, verify every live channel from a clean machine or container and record expected command output in release notes.

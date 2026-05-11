# Uninstall

These paths are defaults. Confirm your installation and environment before deleting local data.

## Homebrew

```bash
brew uninstall claw
brew untap shree-git/homebrew-tap
```

## Windows MSI

Use Windows Settings, Control Panel, or:

```powershell
winget uninstall ShreeGit.ClawVCS
```

The exact package name depends on the accepted installer manifest.

## Manual Binary

Remove the binary from the directory where it was installed:

```bash
rm -f ~/.local/bin/claw
```

If installed elsewhere:

```bash
command -v claw
```

Then remove that path if it belongs to Claw VCS.

## Shell or PowerShell Installer

Installer-managed binaries are commonly placed under:

- `$CLAW_HOME/bin`
- `~/.claw/bin`
- `~/.local/bin`
- `%USERPROFILE%\.claw\bin`

Remove the binary and update `PATH` if needed.

## Configuration and Auth

Claw repository state lives in each repository under `.claw/`. Auth profiles and user-level configuration may live under your platform config directory. Inspect before deleting:

```bash
find "$HOME" -maxdepth 3 -iname '*claw*' -print
```

Delete auth profiles only after confirming no active daemon or remote depends on them.

## Repository Data

To remove Claw state from a specific checkout:

```bash
rm -rf .claw
```

This deletes Claw objects, refs, capsules, policies, backups, and local metadata for that checkout. Keep a verified backup if any data matters.

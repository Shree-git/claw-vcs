# Migration docs

Use these pages when bringing an existing project or team process into Claw.

## Pages

- [From Git to Claw](from-git.md)
- [Upgrade and config migration checklist](upgrade-checklist.md)

## Migration rules

- Keep the existing Git remote until Claw rollback has been tested.
- Pin Claw CLI and daemon versions during the migration window.
- Run `claw git-roundtrip` before relying on Git interop.
- Back up `.claw/` before config migration or rollback testing.
- Record the exact release artifact used for each environment.

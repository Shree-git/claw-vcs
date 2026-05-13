# Platform operator

Use this path when running Claw for a team.

## Start

- [Production install](../operations/production-install.md)
- [Production readiness checklist](../reference/production-readiness-checklist.md)
- [Upgrade and rollback](../operations/upgrade-and-rollback.md)
- [Runbooks](../runbooks/README.md)

## Day one checks

- daemon runs behind your network boundary
- bearer auth is enabled
- TLS is configured for non-localhost access
- `.claw/` backup and restore have been tested
- health and metrics endpoints are scraped
- support bundle command works
- CLI and daemon versions are pinned and compatibility-checked before remote
  operations
- `.claw/` and `~/.claw/auth.toml` handling is documented in your backup and
  credential process

## During incidents

Start with:

```bash
claw --version
claw admin preflight
claw admin support-bundle
```

Then follow the matching runbook.

## Do not assume

- hosted ClawLab remotes are live unless release notes say so
- Git bridge behavior is stable without version pinning
- policy objects apply globally without being referenced by intents
- support bundles are safe to share without review

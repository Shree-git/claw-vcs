# Claw VCS Operator Documentation

This tree is for teams evaluating or running self-hosted Claw deployments. Treat the guidance here as the supported operator baseline for `v0.1.x` controlled production rollouts, with a deliberately narrow support boundary rather than a claim of broad platform maturity.

## Start here

- [Quickstart](getting-started/quickstart.md)
- [Landing page](landing-page.md)
- [Concepts](concepts/index.md)
- [Workflows](workflows/index.md)
- [Agent docs](agents/index.md)
- [Migration docs](migration/index.md)
- [Persona index](persona/index.md)
- [Production install](operations/production-install.md)
- [Public launch checklist](operations/public-launch-checklist.md)
- [Release verification](security/verifying-releases.md)
- [Threat model](security/threat-model.md)
- [Upgrade and rollback](operations/upgrade-and-rollback.md)
- [Disaster recovery](operations/disaster-recovery.md)
- [Troubleshooting](operations/troubleshooting.md)

## Runbooks

- [Runbook index](runbooks/README.md)
- [Daemon start, stop, and health](runbooks/daemon-start-stop-health.md)
- [Backup and restore](runbooks/backup-and-restore.md)
- [Token rotation](runbooks/token-rotation.md)
- [Emergency rollback](runbooks/emergency-rollback.md)

## Reference

- [Compatibility](reference/compatibility.md)
- [Stability reference](reference/stability.md)
- [Data layout](reference/data-layout.md)
- [Known limitations](reference/known-limitations.md)
- [Object format spec](spec/object-format.md)
- [Benchmarks](reference/benchmarks.md)
- [Unsafe audit](reference/unsafe-audit.md)
- [Panic audit](reference/panic-audit.md)
- [Production readiness checklist](reference/production-readiness-checklist.md)
- [Production profile defaults](reference/production-profile-defaults.md)
- [Telemetry policy](reference/telemetry.md)

## Maintainers

- [Governance](maintainers/governance.md)
- [Maintainer guide](maintainers/guide.md)
- [Telemetry maintainer guide](maintainers/telemetry.md)
- [Deprecation maintainer guide](maintainers/deprecations.md)
- [Dependency policy](maintainers/dependency-policy.md)

## Self-hosted-first baseline

Use this baseline unless you have stricter internal controls:

- Run `claw daemon` behind your own network perimeter.
- Require bearer auth (`--auth-token` or `--auth-profile`).
- Terminate TLS at an ingress/proxy or configure daemon TLS via `.claw/config.toml`.
- Validate environment with `claw admin preflight` before first start and after major changes.
- Create and verify metadata backups with `claw admin backup create` and `claw admin backup verify`.
- Run the backup/restore demo in `examples/backup-restore/`.

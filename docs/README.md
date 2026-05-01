# Claw VCS Operator Documentation

This tree is for teams evaluating or running experimental self-hosted Claw deployments. Treat the guidance here as an operator baseline for `v0.1.x`, not as a claim of production maturity across every subsystem.

## Start here

- [Quickstart](getting-started/quickstart.md)
- [Production install](operations/production-install.md)
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
- [Production readiness checklist](reference/production-readiness-checklist.md)
- [Production profile defaults](reference/production-profile-defaults.md)

## Self-hosted-first baseline

Use this baseline unless you have stricter internal controls:

- Run `claw daemon` behind your own network perimeter.
- Require bearer auth (`--auth-token` or `--auth-profile`).
- Terminate TLS at an ingress/proxy or configure daemon TLS via `.claw/config.toml`.
- Validate environment with `claw admin preflight` before first start and after major changes.
- Create and verify metadata backups with `claw admin backup create` and `claw admin backup verify`.

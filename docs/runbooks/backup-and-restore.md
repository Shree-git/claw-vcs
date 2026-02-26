# Runbook: Backup and Restore

## Purpose

Create, verify, and restore repository metadata backups (`.claw/`).

## Create backup

```bash
claw admin backup create
```

Record `backup_id` from output.

## Verify backup

Verify latest:

```bash
claw admin backup verify
```

Verify specific backup:

```bash
claw admin backup verify --backup-id <backup-id>
```

## Restore from backup

1. Stop daemon.
2. Validate restore plan:

```bash
claw admin rollback plan --backup-id <backup-id>
```

3. Execute restore:

```bash
claw admin rollback execute --backup-id <backup-id>
```

4. Post-restore checks:

```bash
claw admin backup verify --backup-id <backup-id>
claw admin preflight
```

5. Restart daemon and run client sync smoke test.

## Storage layout

- Backups path: `.claw/backups/<backup-id>/`
- Backup snapshot: `.claw/backups/<backup-id>/snapshot/`
- Manifest: `.claw/backups/<backup-id>/manifest.json`

## Operational notes

- Backups exclude nested historical backup content to avoid recursion.
- Use off-host replication for `.claw/backups/`.
- Run periodic restore drills in non-production environments.

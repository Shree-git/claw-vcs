# Disaster Recovery

This guide covers recovery of Claw repository metadata for self-hosted deployments.

## What is recoverable

- Built-in backup/rollback commands snapshot `.claw/` metadata.
- Backups are stored under `.claw/backups/<backup-id>/`.
- Each backup includes `manifest.json` and a `snapshot/` tree with checksums.

## Recovery objectives

- Define RPO based on backup cadence.
- Define RTO based on host rebuild + backup restore time.

## DR preparation checklist

- Run scheduled `claw admin backup create`.
- Periodically run `claw admin backup verify`.
- Replicate backup storage off-host (copy `.claw/backups/`).
- Keep tested procedure to redeploy known-good `claw` binary.

## Host loss recovery procedure

1. Rebuild host and restore repository working directory.
2. Restore `.claw/backups/` from off-host storage.
3. Install known-good `claw` binary.
4. Identify latest valid backup id and verify it:

```bash
claw admin backup verify --backup-id <backup-id>
```

5. Restore metadata snapshot:

```bash
claw admin rollback execute --backup-id <backup-id>
```

6. Validate environment and start daemon:

```bash
claw admin preflight
claw daemon --listen 0.0.0.0:50051 --auth-profile default
```

7. Re-run client smoke tests (`sync pull` and `sync push`).

## Corruption recovery

If `.claw/` exists but appears corrupted:

1. Stop daemon immediately.
2. Capture diagnostics:

```bash
claw admin support-bundle
```

3. Restore from last verified backup using rollback.
4. Re-verify backup and preflight before restart.

## Post-recovery validation

- `claw admin preflight` reports `PASS`.
- Expected refs are visible via normal sync workflows.
- New backup can be created and verified successfully.

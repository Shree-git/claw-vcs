# Upgrade and Rollback

Use this procedure for low-risk production upgrades.

## Before upgrade

1. Record current binary version:

```bash
claw --version
```

2. Create and verify backup:

```bash
claw admin backup create
claw admin backup verify
```

3. Save current backup id from command output.

## Upgrade steps

1. Stop daemon (service manager preferred).
2. Install new `claw` binary.
3. Preview config migration if needed:

```bash
claw admin migrate plan
```

4. Apply migration:

```bash
claw admin migrate apply
```

5. Run preflight and restart daemon:

```bash
claw admin preflight
claw daemon --listen 0.0.0.0:50051 --auth-profile default
```

6. Run smoke checks from a client repo:

```bash
claw sync pull --remote origin --ref-name heads/main
claw sync push --remote origin --ref-name heads/main
```

## Rollback trigger examples

- Daemon fails to start after upgrade.
- Client sync fails due to protocol/runtime regression.
- Unexpected integrity or ref update errors.

## Rollback steps

1. Stop daemon.
2. Reinstall previous binary version.
3. Validate rollback plan:

```bash
claw admin rollback plan --backup-id <backup-id>
```

4. Execute rollback:

```bash
claw admin rollback execute --backup-id <backup-id>
```

5. Verify and restart:

```bash
claw admin backup verify --backup-id <backup-id>
claw admin preflight
```

6. Restart daemon and re-run smoke checks.

## Notes

- Backups cover repository metadata under `.claw/`.
- Keep at least one known-good binary artifact and one recent verified backup per environment.
- Every `migrate apply`, `backup create`, and `rollback execute` writes an entry to `.claw/migrations/ledger.jsonl`.

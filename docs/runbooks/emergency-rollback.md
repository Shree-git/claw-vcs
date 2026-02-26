# Runbook: Emergency Rollback

## Trigger conditions

- Production outage after upgrade or migration.
- Repeated daemon startup failures.
- Critical sync regressions affecting push/pull.

## Immediate actions

1. Stop daemon to prevent further state changes.
2. Notify incident channel and assign incident owner.
3. Capture diagnostics:

```bash
claw admin support-bundle
```

## Execute rollback

1. Reinstall previous known-good `claw` binary.
2. Identify latest known-good `backup_id`.
3. Validate plan and backup integrity:

```bash
claw admin rollback plan --backup-id <backup-id>
claw admin backup verify --backup-id <backup-id>
```

4. Restore metadata:

```bash
claw admin rollback execute --backup-id <backup-id>
```

5. Validate and restart:

```bash
claw admin preflight
claw daemon --listen 0.0.0.0:50051 --auth-profile default
```

6. Run client smoke checks (`sync pull`, `sync push`).

## Exit criteria

- Daemon healthy and stable.
- Critical client operations restored.
- Incident timeline and used `backup_id` recorded.

## Follow-up

- Preserve support bundle and logs.
- Review `.claw/migrations/ledger.jsonl` for change history.
- Schedule root-cause analysis before next upgrade attempt.

# Runbook: Storage Pressure

## Symptoms

- Host disk usage approaches/exceeds operational threshold (for example, >85-90%).
- Write-heavy operations fail or slow significantly.
- Backup creation/verification starts failing.
- `claw_storage_operation_duration_seconds` and queue age/depth increase.

## Immediate triage

1. Declare incident and assign incident owner.
2. Capture diagnostics:

```bash
claw admin support-bundle
claw admin preflight
```

3. Confirm filesystem pressure on daemon host:

```bash
df -h
du -sh .claw/*
```

4. Identify dominant growth source (`.claw/backups/`, logs, temp files, or data path).

## Mitigation steps

1. Pause non-critical write traffic (bulk sync/CI) to reduce growth rate.
2. Offload large artifacts (especially older backups/logs) to approved external storage.
3. Remove only data eligible by retention policy (start with oldest already-verified backups).
4. If storage remains constrained, expand volume/capacity and restart affected daemon instances.

## Validation checks

- Free space returns above operational threshold and remains stable.
- Backup integrity check succeeds:

```bash
claw admin backup verify
```

- Representative read/write sync operations succeed.
- Storage latency and queue backlog trend back toward baseline.

## Escalation

- Primary: Claw platform on-call.
- Secondary: storage/infrastructure on-call.
- Escalate immediately if free space continues dropping after first mitigation pass.

## Post-incident follow-ups

- Record pressure source, cleanup actions, and recovered capacity.
- Fix retention gaps (backup/log pruning, schedule, ownership).
- Add capacity alert thresholds tied to growth rate, not just absolute usage.

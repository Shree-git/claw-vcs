# Runbook: Degraded Git Backend

## Symptoms

- `sync pull`/`sync push` latency and failure rate increase.
- `claw_daemon_queue_depth` trends upward while worker capacity stays flat in
  `claw_daemon_worker_pool_size`.
- Logs show repeated sync or Git bridge timeout/error messages for the same
  remotes/refs.

## Immediate triage

1. Declare incident and assign incident owner.
2. Capture diagnostics:

```bash
claw admin support-bundle
claw admin preflight
```

3. Run quick protocol check from a client repo:

```bash
claw sync pull --remote origin --ref-name heads/main
```

4. Identify scope: all remotes vs one remote/ref, read-only vs read-write impact.

## Mitigation steps

1. Reduce write pressure (pause bulk sync/CI jobs) while recovery is in progress.
2. Restart affected daemon instances via service manager to clear stuck bridge workers.
3. If issue started after a release, execute [Emergency rollback](emergency-rollback.md).
4. If one remote is failing, temporarily route critical traffic to a healthy remote/mirror.

## Validation checks

- `claw sync pull` and a controlled `claw sync push` succeed for representative repos.
- `claw_daemon_queue_depth` falls and remains stable for at least 15 minutes.
- Error logs no longer show repeating bridge timeout/failure patterns.

## Escalation

- Primary: Claw platform on-call.
- Secondary: storage owner (I/O saturation) and release owner (recent deploy correlation).
- Escalate immediately if user-facing SLO burn remains active after first mitigation pass.

## Post-incident follow-ups

- Record trigger, first failing signal, and mitigation timeline.
- Attach support bundle, key log excerpts, and affected remote/ref list.
- Add a prevention action (capacity, retry tuning, release guardrail, or alert threshold adjustment).

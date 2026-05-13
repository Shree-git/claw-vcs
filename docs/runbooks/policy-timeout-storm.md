# Runbook: Policy Timeout Storm

## Symptoms

- Sudden rise in policy-related request failures and retries.
- `claw_daemon_policy_eval_duration_seconds` rises above baseline when policy
  evaluation is served through the daemon metrics surface.
- Audit logs or `sync_audit_event` records show a sharp policy deny/timeout
  increase for mutating operations.
- User operations fail even when daemon and storage look otherwise healthy.

## Immediate triage

1. Declare incident and assign incident owner.
2. Capture diagnostics:

```bash
claw admin support-bundle
claw admin preflight
```

3. Confirm impact with a representative operation:

```bash
claw sync pull --remote origin --ref-name heads/main
```

4. Check timeline for recent policy/config deployments and token rotations.

## Mitigation steps

1. Revert the most recent policy change in your policy control plane.
2. If policy service is overloaded, scale it up and restore normal response latency.
3. Restart affected daemon instances after policy rollback/scale-up to clear hot retry loops.
4. If failures continue, temporarily reduce non-critical traffic until policy latency stabilizes.

## Validation checks

- `claw_daemon_policy_eval_duration_seconds` returns near baseline when that
  daemon metric is present.
- Policy failure/deny spikes clear in audit logs or `sync_audit_event` records.
- Representative `claw sync pull` and `claw sync push` operations succeed.
- Incident channel confirms user-facing error rate is back within SLO.

## Escalation

- Primary: Claw platform on-call.
- Secondary: security/policy on-call.
- Escalate to release owner if correlated with a recent deployment.

## Post-incident follow-ups

- Document the policy version/config that triggered the storm.
- Add or tune alerting for policy latency before timeout thresholds are crossed.
- Capture rollback/playbook improvements and assign owners with due dates.

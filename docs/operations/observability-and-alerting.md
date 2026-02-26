# Observability and Alerting

Use this as the minimum operator baseline for production Claw deployments.

## Enforcement Boundary

- **CI-enforced:** release gates, contract-diff artifact generation, release artifact signing, and the scheduled `nightly-chaos.yml` run.
- **Operator practice:** dashboard/alert tuning, canary assessment, incident response, and promotion decisions.

## Telemetry baseline

### Logs

- Emit structured logs (JSON) from CLI, daemon, storage, and git bridge.
- Include correlation fields on every line: `trace_id`, `span_id`, `request_id`, `component`, `operation`, `repo`, `remote`, `result`, `duration_ms`.
- Propagate IDs end-to-end:
  - CLI generates or forwards `request_id`.
  - Daemon preserves `request_id` and attaches tracing context.
  - Storage and git bridge log the same IDs for joinability.

### Metrics (Prometheus)

Expose metrics from daemon and bridge with low-cardinality labels (`component`, `operation`, `result`, `remote`).

- **Latency**
  - `claw_request_duration_seconds` (histogram)
  - `claw_storage_operation_duration_seconds` (histogram)
  - `claw_git_bridge_operation_duration_seconds` (histogram)
- **Queue depth / backlog**
  - `claw_sync_queue_depth` (gauge)
  - `claw_sync_oldest_job_age_seconds` (gauge)
- **Auth failures**
  - `claw_auth_failures_total` (counter; label reason: `missing_token|invalid_token|expired_token`)
- **Retries**
  - `claw_retries_total` (counter; labels `operation`, `reason`)
  - `claw_retry_backoff_seconds` (histogram)
- **Policy evaluation**
  - `claw_policy_eval_total` (counter; labels `result=allow|deny`)
  - `claw_policy_eval_duration_seconds` (histogram)

### Tracing boundaries

Create a single distributed trace per user operation with these span boundaries:

1. `cli.command`
2. `daemon.rpc`
3. `storage.transaction`
4. `git_bridge.operation`

Required attributes: `request_id`, `repo`, `remote`, `ref_name`, `operation`, `retry_count`, `policy_result`.

## Starter dashboard

Build one dashboard named **Claw Operations** with these panels:

1. **SLO and Error Budget**
   - Success rate (`1 - error_ratio`) for daemon RPCs.
   - Burn rate (5m and 1h windows).
2. **Latency**
   - P50/P95/P99 for `claw_request_duration_seconds` by operation.
3. **Backlog Health**
   - `claw_sync_queue_depth` and `claw_sync_oldest_job_age_seconds` over time.
4. **Policy Deny Rate**
   - `rate(claw_policy_eval_total{result="deny"}[5m])` and deny percentage.
5. **Auth Anomalies**
   - `rate(claw_auth_failures_total[5m])` split by reason.
6. **Retry Pressure**
   - Retry rate and backoff distribution by operation.

## Starter alerts

Tune thresholds after 2-4 weeks of baseline traffic.

- **Error budget burn (page)**
  - Condition: fast burn `>14x` over 5m and slow burn `>2x` over 1h.
  - Action: page primary on-call; create incident ticket.
- **Backlog growth (page)**
  - Condition: `claw_sync_queue_depth` increasing for 15m and `claw_sync_oldest_job_age_seconds > 300`.
  - Action: page primary; check daemon saturation and storage latency.
- **Policy deny spike (ticket/page)**
  - Condition: deny ratio >3x 7-day baseline for 10m.
  - Action: ticket by default; page if user-facing failures exceed SLO.
- **Auth anomaly (page)**
  - Condition: `invalid_token` or `expired_token` failures >5x baseline for 5m.
  - Action: page primary; investigate token rollout, clock skew, or abuse.

## Runbook hooks and ownership

- **Primary owner:** Claw platform on-call (daemon + sync path).
- **Secondary owners:** security on-call (auth/policy), storage owner (I/O bottlenecks), release owner (recent deploy correlation).
- **CI signals to review during release operations:**
  - `contract-diff.yml` artifact `contract-diff-summary` when contract files change.
  - `release.yml` signing output (`<artifact>.sig` and `<artifact>.pem`) and `verify-artifacts.yml` verification result.
  - Latest `nightly-chaos.yml` run (deterministic subset with `CHAOS_MODE=off`).
- Every alert should link to:
  - this doc,
  - [Troubleshooting](troubleshooting.md),
  - support-bundle command: `claw admin support-bundle`.
- First 10 minutes checklist:
  1. Confirm active burn/backlog/auth panel impact.
  2. Correlate `request_id` across logs and traces.
  3. Identify boundary with dominant latency/error (`daemon`, `storage`, or `git_bridge`).
  4. Capture mitigation decision and owner in incident timeline.
- Post-incident:
  - Record which signal fired first.
  - Adjust thresholds only after confirming true positive/false positive pattern.

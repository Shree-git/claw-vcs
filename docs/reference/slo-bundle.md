# SLO Bundle

Implementation targets for release gating and ongoing operations.

## Core SLOs
- **Availability:** >= 99.9% monthly for control-plane and command execution endpoints.
- **Command success rate:** >= 99.5% rolling 7-day for non-user-cancelled commands.
- **Latency (successful commands):** p95 <= 2.0s, p99 <= 5.0s rolling 24h.
- **Queue freshness:** stale queue items (>5 min old) <= 50 for >= 99% of 5-minute windows.

## Release Gates
- **CI-enforced checks:** `release.yml` gates and `cross-version-runtime.yml` must be green.
- **Operator-recommended checks:** `soak-24h.yml` 24h run is reviewed before stable promotion.
- Canary and stable promotion require all core SLOs to be in-policy.
- Any sustained breach (>30 minutes) blocks promotion until mitigated.
- If a release introduces a new sustained breach, rollback or mitigation is mandatory.

## Error Budget Model
- **Availability error budget:** 0.1% monthly downtime allowance.
- **Success-rate error budget:** 0.5% monthly failed-command allowance.
- **Burn-rate policy:**
  - Fast burn: >10% of monthly budget consumed in 24h -> freeze non-critical releases.
  - Slow burn: >25% consumed in 7d -> require reliability review before next train.

## Measurement Rules
- Measure from production telemetry only; staging data excluded.
- Exclude planned maintenance windows only when pre-announced and tagged.
- Percentiles and rates are computed per 5-minute window, then rolled up.

## Operational Actions
- Alert when p99 latency or queue staleness breaches for 3 consecutive windows.
- Open incident for any SLO breach lasting >= 15 minutes.
- Link every breach to a corrective action item in the next release train.

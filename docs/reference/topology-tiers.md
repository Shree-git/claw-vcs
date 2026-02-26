# Self-Hosted Topology Tiers and Support Boundary

This page defines supported self-hosted deployment tiers, what is in-bounds for support, and what operators must implement to stay in a supported posture.

## Tier 1: Single-node prod

- **Support boundary:** One production daemon node running all roles (control path and data path) with local or attached persistent storage, no automatic failover.
- **Required controls:** Host hardening, persistent volume backups, health checks, process supervision, capacity headroom for peak write bursts, and documented restore procedure with periodic restore validation.
- **Known limits:** Planned and unplanned downtime are service-impacting; maintenance requires an outage window; vertical scaling ceiling is host-bound.
- **Overload behavior:** Under saturation, prioritize control path RPCs (admin, auth, coordination, heartbeats) ahead of sync/data traffic.
- **Load-shedding guidance:** Apply admission limits and queue caps on sync/data operations first; return explicit retryable overload responses for data operations before allowing control path starvation.

## Tier 2: Active/passive HA

- **Support boundary:** Two nodes with one active primary and one passive standby, coordinated failover, and replicated durable state (or shared durable backend) with a single writable active node.
- **Required controls:** Automated health detection and failover runbook, split-brain prevention (fencing/lease), replication lag monitoring, regular failover drills, and defined RPO/RTO objectives.
- **Known limits:** Brief disruption during failover is expected; replication lag can increase recovery point risk; no horizontal read scale from the passive node.
- **Overload behavior:** Keep control path on the active node responsive during failover and overload windows; avoid promoting a lagging passive node without policy checks.
- **Load-shedding guidance:** Shed non-critical sync/data requests on the active node before control path degradation; throttle client retries during failover to avoid thundering herd effects.

## Tier 3: Multi-replica read scale

- **Support boundary:** One writable primary plus multiple read replicas for read-heavy workloads, with explicit read/write routing and replica lag awareness.
- **Required controls:** Deterministic routing policy, replica health and lag SLOs, backpressure limits per replica, capacity planning for primary write amplification, and tested replica rejoin/rebuild procedures.
- **Known limits:** Read-after-write consistency is not guaranteed on replicas during lag; primary remains a single write bottleneck; replica fan-out increases operational complexity.
- **Overload behavior:** Preserve primary control path availability first, then primary write path, then replica reads.
- **Load-shedding guidance:** Shed replica-bound read traffic first (especially expensive or low-priority queries), then non-essential data sync operations; reserve fixed control path concurrency on the primary.

## Cross-tier overload policy (required)

All supported tiers must enforce control-path-first behavior under stress:

1. Reserve concurrency and CPU/memory budget for control path handlers.
2. Bound work queues and per-tenant/per-client concurrency for data path traffic.
3. Prefer fail-fast overload responses with retry hints over unbounded queueing.
4. Degrade optional/expensive features before core control operations.

If a deployment cannot preserve control path responsiveness during sustained overload, it is outside the supported operating boundary until corrected.

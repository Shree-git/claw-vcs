# Telemetry maintainer guide

This page is for maintainers changing logs, metrics, traces, or support bundle
content. Operator use is covered in
[Observability and alerting](../operations/observability-and-alerting.md).

## Rules

- Keep metric labels low-cardinality.
- Keep request correlation fields stable when operators depend on them.
- Treat support bundles as sensitive by default.
- Do not add secrets, tokens, private URLs, or customer data to public logs.
- Document metric additions or removals in release notes.

## Review checklist

- New metric has name, type, unit, and label list.
- Logs include `component`, `operation`, `result`, and request correlation data
  when available.
- Trace spans do not expose secret values.
- Support bundle changes are named in release notes.
- Dashboards and alerts still match metric names.

## Current surfaces

- Daemon health endpoints: `/v1/health/live`, `/v1/health/ready`, and
  `/v1/health/deps`.
- Metrics endpoint: `/v1/metrics` in Prometheus text format.
- Runtime error envelopes: `claw --error-format json <command>`.
- Support bundles: `claw admin support-bundle`, written under `.claw/support/`
  unless `--out` is supplied.

## Redaction expectations

- Bearer tokens, auth profile tokens, private agent keys, TLS private keys, and
  private capsule payloads must not be emitted.
- Paths may be sensitive. Include repository-relative paths only when the
  diagnostic requires them.
- Keep request IDs and correlation IDs stable enough for operator debugging, but
  do not use user-supplied IDs as metric labels without normalization.

## Release-note trigger

Add release-note text when any of these change:

- metric name, type, unit, or label set
- health endpoint status semantics
- support bundle schema
- JSON error envelope fields or exit-code mapping

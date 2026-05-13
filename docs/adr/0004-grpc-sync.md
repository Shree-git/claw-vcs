# ADR 0004: gRPC Sync Protocol

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `proto/claw/sync.proto`, `docs/reference/compatibility.md`, `crates/claw-sync/src/server.rs`, `crates/claw-sync/src/client.rs`

## Context

Agents need programmatic access to refs, objects, intents, changes, capsules, workstreams, and events. The protocol needs streaming object transfer and typed service contracts.

## Decision

Use gRPC over HTTP/2 for daemon sync and agent-facing services. Keep protocol version and feature negotiation explicit.

## Alternatives Considered

- Git protocol extension: leverages existing infrastructure, but does not model intent, capsules, and policy as first-class sync resources.
- REST-only API: simpler for browser clients, but weaker for object streaming and typed bidirectional service contracts.
- Local filesystem sharing only: easy for demos, but insufficient for remote agents and daemon deployments.

## Consequences

- Object fetch and push can stream.
- Generated client/server code reduces schema drift.
- Browser-native clients may need an HTTP adapter or gateway.

## Verification Links

- `tests/integration/cli_sync_e2e_tests.rs`
- `tests/integration/cross_version_runtime_tests.rs`
- `docs/reference/compatibility-matrix.json`

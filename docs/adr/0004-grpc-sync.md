# ADR 0004: gRPC Sync Protocol

## Status

Accepted for v0.1.

## Context

Agents need programmatic access to refs, objects, intents, changes, capsules, workstreams, and events. The protocol needs streaming object transfer and typed service contracts.

## Decision

Use gRPC over HTTP/2 for daemon sync and agent-facing services. Keep protocol version and feature negotiation explicit.

## Consequences

- Object fetch and push can stream.
- Generated client/server code reduces schema drift.
- Browser-native clients may need an HTTP adapter or gateway.

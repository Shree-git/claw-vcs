# Architecture Decision Records

This directory records durable architecture choices for Claw VCS. ADRs are
append-only unless a later ADR explicitly supersedes one.

## ADR Template

Each ADR should include:

- status
- date
- owner
- decision
- alternatives considered
- consequences
- implementation and verification links
- supersession state

## Index

| ADR | Status | Date | Owner | Decision | Key Links |
|---|---|---|---|---|---|
| [0001](0001-no-staging-area.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Use atomic snapshots instead of a staging index. | `docs/concepts/object-model.md`, `docs/cli/snapshot.md` |
| [0002](0002-object-id-hashing.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Use BLAKE3 domain-separated object IDs. | `crates/claw-core/src/hash.rs`, `tests/vectors/ids/` |
| [0003](0003-protobuf-and-cof.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Encode payloads with Protobuf and wrap with COF. | `docs/spec/object-format.md`, `crates/claw-core/src/cof.rs`, `tests/vectors/cof/` |
| [0004](0004-grpc-sync.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Use gRPC/HTTP2 for daemon sync and agent services. | `proto/claw/sync.proto`, `crates/claw-sync/src/server.rs` |
| [0005](0005-intent-change-revision.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Model work as intent, change, and revision objects. | `docs/concepts/intent-change-revision.md`, `crates/claw-core/src/types/` |
| [0006](0006-capsules-as-repo-objects.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Store capsules as first-class repository objects. | `docs/concepts/capsules-and-evidence.md`, `crates/claw-crypto/src/capsule.rs` |
| [0007](0007-policy-objects-in-repo.md) | Accepted for v0.1 | 2026-05-12 | @Shree-git | Store integration policy as versioned repo objects. | `docs/concepts/policies.md`, `crates/claw-policy/src/` |

## Supersession Policy

When a decision changes, create a new ADR and update the older record's
`Superseded by` metadata. Do not rewrite historical rationale unless it was
factually wrong.

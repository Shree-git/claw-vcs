# ADR 0005: Intent, Change, Revision

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `docs/concepts/intent-change-revision.md`, `crates/claw-core/src/types/intent.rs`, `crates/claw-core/src/types/change.rs`, `crates/claw-core/src/types/revision.rs`

## Context

Commit history captures what changed but only loosely captures why it changed. Agent-authored work needs a structured link between the requested goal, implementation attempt, and recorded repository state.

## Decision

Model work as:

- `Intent`: the goal, constraints, acceptance tests, policy refs, and status.
- `Change`: one implementation attempt linked to an intent.
- `Revision`: a recorded repository state linked to parents, tree, patches, and optional capsule.

## Alternatives Considered

- Commit-only history: familiar, but too weak for structured goal and evidence queries.
- Issue tracker as intent source: useful integration point, but not offline-verifiable repository state.
- Single task object: simpler, but does not separate requested goal from competing implementation attempts and recorded revisions.

## Consequences

- Multiple changes can compete or cooperate on one intent.
- Evidence can bind to the exact revision it evaluated.
- Querying by goal or acceptance test becomes a repository operation.

## Verification Links

- `docs/concepts/intent-change-revision.md`
- `docs/cli/intent.md`
- `docs/cli/change.md`
- `tests/integration/cli_core_workflow_tests.rs`

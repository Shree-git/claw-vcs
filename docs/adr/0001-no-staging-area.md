# ADR 0001: No Staging Area

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `docs/concepts/object-model.md`, `docs/cli/snapshot.md`, `crates/claw/src/commands/snapshot.rs`

## Context

Git's staging area is powerful for human patch curation, but it adds ambiguity for autonomous agents: the workspace, staged tree, and intended change can diverge. Claw VCS centers intent-linked snapshots and signed evidence, so a partial staging layer would complicate provenance.

## Decision

`claw snapshot` records the working tree atomically. Partial selection should be represented by separate worktrees, path-scoped tools, or smaller changes rather than a staging index.

## Alternatives Considered

- Git-style staging index: familiar, but adds ambiguity between workspace, staged tree, and agent intent.
- Path-only snapshot flags: useful later, but too easy to mistake for provenance-preserving partial staging.
- Separate worktrees or scoped changes: keeps the repository state and intent boundary explicit.

## Consequences

- Agent workflows are simpler and easier to audit.
- Users who depend on partial staging should keep using Git for that workflow until a Claw-native alternative is designed.
- Snapshot commands must make ignored files and local state rules clear.

## Verification Links

- `docs/workflows/solo.md`
- `docs/workflows/human-agent-pair.md`
- `tests/integration/cli_core_workflow_tests.rs`

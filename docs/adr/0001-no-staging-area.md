# ADR 0001: No Staging Area

## Status

Accepted for v0.1.

## Context

Git's staging area is powerful for human patch curation, but it adds ambiguity for autonomous agents: the workspace, staged tree, and intended change can diverge. Claw VCS centers intent-linked snapshots and signed evidence, so a partial staging layer would complicate provenance.

## Decision

`claw snapshot` records the working tree atomically. Partial selection should be represented by separate worktrees, path-scoped tools, or smaller changes rather than a staging index.

## Consequences

- Agent workflows are simpler and easier to audit.
- Users who depend on partial staging should keep using Git for that workflow until a Claw-native alternative is designed.
- Snapshot commands must make ignored files and local state rules clear.

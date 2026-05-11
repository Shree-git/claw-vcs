# ADR 0005: Intent, Change, Revision

## Status

Accepted for v0.1.

## Context

Commit history captures what changed but only loosely captures why it changed. Agent-authored work needs a structured link between the requested goal, implementation attempt, and recorded repository state.

## Decision

Model work as:

- `Intent`: the goal, constraints, acceptance tests, policy refs, and status.
- `Change`: one implementation attempt linked to an intent.
- `Revision`: a recorded repository state linked to parents, tree, patches, and optional capsule.

## Consequences

- Multiple changes can compete or cooperate on one intent.
- Evidence can bind to the exact revision it evaluated.
- Querying by goal or acceptance test becomes a repository operation.

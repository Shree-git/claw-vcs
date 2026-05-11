# ADR 0007: Policy Objects Live in the Repository

## Status

Accepted for v0.1.

## Context

Branch protection and CI rules often live outside the repository. Forking or migrating a project can silently drop those controls.

## Decision

Represent integration policy as versioned repository objects. Policies define required checks, reviewers, sensitive paths, quarantine behavior, trust thresholds, and visibility semantics.

## Consequences

- Policy travels with the repository.
- Policy changes can be reviewed and audited like source changes.
- Hosted branch protection is still useful, but it is no longer the only policy record.

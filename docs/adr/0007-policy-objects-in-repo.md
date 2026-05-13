# ADR 0007: Policy Objects Live in the Repository

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `docs/concepts/policies.md`, `docs/reference/known-limitations.md`, `crates/claw-policy/src/`, `crates/claw-core/src/types/policy.rs`

## Context

Branch protection and CI rules often live outside the repository. Forking or migrating a project can silently drop those controls.

## Decision

Represent integration policy as versioned repository objects. Policies define required checks, reviewers, sensitive paths, quarantine behavior, trust thresholds, and visibility semantics.

## Alternatives Considered

- Hosted-only branch protection: important defense in depth, but not portable with the repository.
- Local config file only: easy to edit, but weaker as versioned, content-addressed repository evidence.
- CI workflow rules only: useful execution mechanism, but too coupled to one automation environment.

## Consequences

- Policy travels with the repository.
- Policy changes can be reviewed and audited like source changes.
- Hosted branch protection is still useful, but it is no longer the only policy record.

## Verification Links

- `docs/concepts/policies.md`
- `docs/workflows/policy-gated-integration.md`
- `crates/claw-policy/src/checks.rs`
- `tests/vectors/policies/basic_required_checks.json`

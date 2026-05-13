# Maintainer governance

This page records maintainer rules for public launch work.

## Review rules

- Public interface changes need docs and release-note text.
- Stability changes must update `docs/reference/stability.md` and the public
  interface manifest.
- Operator behavior changes must update the relevant operation doc or runbook.
- Security-sensitive changes must follow `SECURITY.md`.

## Ownership

`CODEOWNERS` is the routing layer for reviews. Today every protected area routes
to the single maintainer account. Release, security, and operator owner groups
are future governance roles; until those teams exist, area-specific review means
explicit maintainer review against the relevant checklist or runbook.

Claw VCS is currently maintained by Shree. Governance will evolve as contributor
volume grows. Shree is the final decision-maker for roadmap, release, and
security triage until a broader maintainer group exists.

## Funding

Claw VCS does not currently publish GitHub Sponsors or other funding links. Add
`.github/FUNDING.yml` only if inbound sponsorship is intentionally opened.

## Release readiness

Before a release, maintainers check:

- changelog entry
- compatibility matrix
- release checklist
- artifact verification
- rollback point
- known issues and migration notes

## Community triage

- Apply `security` only to public hardening work. Vulnerability reports should
  stay in the private security advisory flow.
- Apply `known-limitation` when the issue documents an accepted v0.1 boundary,
  not when it is simply an untriaged bug.
- Apply `protocol` for object format, sync, compatibility, or daemon API
  changes.
- Apply `git-interop` for import, export, notes, or roundtrip behavior.
- Move vague feature requests to design discussion before marking them
  `help wanted`.

## Docs ownership

Docs that describe commands must be checked against the current CLI source or
`claw --help`. If a command does not exist, say so directly and document the
current workaround instead of writing aspirational examples.

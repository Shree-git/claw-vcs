# Maintainer Guide

## Review Standards

- Require focused PRs with tests or docs evidence matching the behavior changed.
- Treat crypto, policy, sync, release, and workflow files as security-sensitive.
- Ask for explicit migration notes when public interfaces change.

## Release Process

- Follow `RELEASING.md`.
- Verify changelog, release checklist, signatures, attestations, SBOM, and install smoke tests.
- Keep rollback instructions linked from release notes.
- For public launch naming work, record trademark, domain, social-handle, and
  package-name evidence in `docs/operations/name-clearance.md`.

## Security Triage

- Use GitHub Security Advisories for private reports.
- Preserve evidence, affected versions, reproduction steps, and key/runner exposure details.
- Coordinate rotation guidance before public disclosure when keys or release artifacts are affected.

## Labels And Roadmap

- Keep `.github/labels.yml` aligned with the public launch checklist.
- Use `known-limitation` for documented v0.1 constraints.
- Update `ROADMAP.md` when a milestone changes materially.

## Emergency Fixes

- Prefer normal PR review.
- If bypass is unavoidable, record why, run the same release gates afterward, and open a follow-up issue for missed controls.

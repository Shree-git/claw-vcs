# Deprecation maintainer guide

The public deprecation policy is
[Deprecation policy](../reference/deprecation-policy.md). This page is the
maintainer checklist for applying it.

## Before deprecating

- Identify the public surface.
- Name the replacement.
- Add docs with migration steps.
- Add release-note text.
- Add a warning or error path that users can understand.

## During the window

- Track the lifecycle state: `N`, `N+1`, or `N+2`.
- Keep compatibility matrix entries current.
- Test warnings and soft-fail behavior.
- Keep rollback steps available for operators.

## Removal

Removal is allowed only after the policy window is complete, unless the change
is required for security or data safety. The release notes must name the removed
surface, replacement, and final migration step.

## Pre-1.0 wording

Even before v1.0, do not describe a removal as harmless if it affects documented
commands, object fields, config keys, policy semantics, daemon endpoints, or Git
bridge behavior. Name the break, the affected versions, and the validation step
operators should run after upgrading.

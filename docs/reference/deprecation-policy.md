# Deprecation Policy

This policy defines how public interfaces are changed and removed without disrupting operators.

## Lifecycle: N / N+1 / N+2

- **N (warn)**: Feature or interface is marked deprecated. Runtime/CLI warnings are mandatory, and docs/release notes include exact replacement and migration steps.
- **N+1 (soft-fail opt-in)**: Deprecated item continues to work by default, but operators can enable soft-fail behavior with explicit opt-in controls.
- **N+2 (remove)**: Deprecated item can be removed. Removal must be called out in release notes with a final migration reminder.

## No Silent Breakage Rule

Public interfaces must not break without an explicit operator-visible signal.

- If behavior changes, the release must include release-note callouts and migration guidance.
- If a deprecated feature is used, tooling must emit a clear warning (`deprecated`, replacement, and removal target release).
- If support is dropped, failures must be explicit (clear error message and remediation link/path), never ambiguous behavior changes.

## Scope

This policy applies to public interfaces listed in `docs/reference/public-interface-manifest.md`:

- CLI surface
- Daemon HTTP API
- Policy schema
- Git interop contract

## Operator Guarantees

- Stable interfaces get at least two release cycles of overlap before removal (`N` to `N+2`).
- Migration paths are documented before removal is allowed.
- Compatibility-state changes are tracked in `docs/reference/compatibility-matrix.json`.

## Enforcement Checklist (Release Gate)

Before shipping any deprecation-related release:

- Deprecation state is set correctly (`N`, `N+1`, or `N+2`).
- Warnings and error messages are tested and human-readable.
- Compatibility matrix is updated.
- Release notes include migration command examples or config diffs.

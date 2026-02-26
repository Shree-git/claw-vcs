# Public Interface Manifest

This document defines which interfaces are public and what stability guarantees operators can rely on.

## Stability Levels

- **Stable**: Backward-compatible within a major version. Breaking changes require a major version change and a deprecation window.
- **Beta**: Intended for production trials. Minor-version breaking changes are allowed with release-note callouts.
- **Experimental**: No compatibility guarantee. Can change or be removed at any release.

## Public Surfaces

| Surface | Scope | Stability | Contract for Operators |
|---|---|---|---|
| CLI command surface | User-facing commands, flags, positional arguments, exit codes, and non-debug stdout/stderr formats | Stable | Existing command paths and flag meanings are preserved across patch/minor updates in the same major line. |
| Daemon HTTP API | Versioned HTTP endpoints, request/response JSON/text shape, status code semantics, auth headers | Beta | Canonical schema artifact is `docs/reference/daemon-http-openapi-v1.json`; current v1 surface includes `/v1/health/live`, `/v1/health/ready`, `/v1/health/deps`, and `/v1/metrics` (Prometheus text). Endpoint behavior may change between minor releases, with release-note callouts and migration instructions. |
| Policy schema | Policy document keys, value types, validation rules, and schema version negotiation | Stable | Existing valid policies remain valid for N and N+1. Validation changes are additive unless a deprecation cycle completes. |
| Git interop contract | Mapping between Claw and Git refs/objects, clone/fetch/push interoperability rules, conflict behavior for bridge operations | Experimental | Behavior can change as interoperability matures; pin CLI + daemon versions for automation using this surface. |

## Explicitly Non-Public

The following are implementation details and may change without notice:

- Internal crate/module APIs in `crates/*`
- On-disk temporary files and cache layouts not marked as storage format contracts
- Debug log field names and tracing spans

## Change Control Expectations

- Every public-interface change must include release-note text.
- Any change that can break existing automation must include an upgrade path.
- Stable-surface removals require deprecation policy completion (`N` -> `N+1` -> `N+2`).

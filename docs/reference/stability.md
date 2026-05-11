# Stability reference

This page summarizes what can change in the `v0.1.x` line. The source of truth
for public surfaces is [Public Interface Manifest](public-interface-manifest.md).

## Stability levels

| Level | Meaning | Change rule |
|---|---|---|
| Stable | Intended for production use inside the current major line. | Breaking changes need a deprecation window. |
| Beta | Ready for controlled trials. | Minor releases can change behavior with release-note callouts. |
| Experimental | Still being shaped. | Can change or be removed in any release. |

## Current surfaces

| Surface | Level |
|---|---|
| CLI command surface | Stable |
| Policy schema | Stable |
| Daemon HTTP health and metrics v1 | Beta |
| gRPC sync protocol | Beta |
| Git interop contract | Experimental |
| Internal Rust crate APIs | Not public |
| Temporary files and caches | Not public |

## Release rules

- Stable removals follow the deprecation policy.
- Beta breaking changes must be named in release notes.
- Experimental changes should still include migration notes when users are likely
  to depend on the behavior.
- Debug logs and tracing span names are not public contracts unless a doc says so.

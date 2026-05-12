# ADR 0002: BLAKE3 Object IDs

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `crates/claw-core/src/hash.rs`, `docs/spec/object-format.md`, `tests/vectors/ids/`

## Context

Claw objects need fast content addressing across local storage, sync, and Git bridge workflows. The hash must distinguish object type domains and avoid legacy SHA-1 assumptions.

## Decision

Use BLAKE3 over a domain-separated byte sequence:

```text
"claw\0" || type_tag || cof_version || canonical_payload
```

Display IDs as `clw_` plus lowercase Base32.

## Alternatives Considered

- SHA-1: familiar from Git, but not appropriate for a new provenance format.
- SHA-256: conservative and widely known, but slower than BLAKE3 for large local repositories.
- Raw BLAKE3 without domain separation: fast, but would allow identical payload bytes in different object domains to share IDs.

## Consequences

- Hashing is fast for large repositories.
- Different object types cannot collide solely because their payload bytes match.
- Git interop must maintain a mapping between Git object IDs and Claw object IDs.

## Verification Links

- `crates/claw-core/src/hash.rs`
- `crates/claw-core/tests/serialization_props.rs`
- `tests/integration/vector_manifest_tests.rs`

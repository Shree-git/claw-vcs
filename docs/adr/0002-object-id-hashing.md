# ADR 0002: BLAKE3 Object IDs

## Status

Accepted for v0.1.

## Context

Claw objects need fast content addressing across local storage, sync, and Git bridge workflows. The hash must distinguish object type domains and avoid legacy SHA-1 assumptions.

## Decision

Use BLAKE3 over a domain-separated byte sequence:

```text
"claw\0" || type_tag || cof_version || canonical_payload
```

Display IDs as `clw_` plus lowercase Base32.

## Consequences

- Hashing is fast for large repositories.
- Different object types cannot collide solely because their payload bytes match.
- Git interop must maintain a mapping between Git object IDs and Claw object IDs.

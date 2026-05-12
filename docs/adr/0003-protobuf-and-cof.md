# ADR 0003: Protocol Buffers Plus COF

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `docs/spec/object-format.md`, `proto/claw/objects.proto`, `crates/claw-core/src/cof.rs`, `crates/claw-core/src/proto_conv.rs`

## Context

The object model needs typed structured payloads, language-neutral schema evolution, corruption checks, and explicit object metadata.

## Decision

Encode object payloads with Protocol Buffers and wrap them in Claw Object Format (COF). COF carries magic bytes, format version, object type tag, flags, compression marker, uncompressed length, payload bytes, and CRC32.

## Alternatives Considered

- JSON-only objects: easy to inspect, but weak for canonical encoding and binary payloads.
- Raw Protobuf only: compact, but lacks storage-level magic/version/type/CRC checks before payload decode.
- Custom binary schema only: maximal control, but higher implementation and tooling cost than Protobuf plus a thin envelope.

## Consequences

- Protobuf handles structured object fields and generated API types.
- COF provides storage-level validation before protobuf decoding.
- Future format versions need an explicit migration and compatibility policy.

## Verification Links

- `docs/spec/object-format.md`
- `tests/vectors/cof/`
- `tests/vectors/objects/core_object_types.json`
- `tests/integration/vector_manifest_tests.rs`

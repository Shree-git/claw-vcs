# ADR 0003: Protocol Buffers Plus COF

## Status

Accepted for v0.1.

## Context

The object model needs typed structured payloads, language-neutral schema evolution, corruption checks, and explicit object metadata.

## Decision

Encode object payloads with Protocol Buffers and wrap them in Claw Object Format (COF). COF carries magic bytes, format version, object type tag, flags, compression marker, uncompressed length, payload bytes, and CRC32.

## Consequences

- Protobuf handles structured object fields and generated API types.
- COF provides storage-level validation before protobuf decoding.
- Future format versions need an explicit migration and compatibility policy.

# Object Format

Claw Object Format (COF) wraps each protobuf payload with a small binary header and trailer so object storage can validate type, version, compression, length, and corruption independently of the content hash.

## Binary Layout

| Field | Encoding | Notes |
|---|---|---|
| Magic | 4 bytes | ASCII `CLW1`. |
| Version | `u8` | Current version is `0x01`. |
| Type tag | `u8` | Object kind. See the type table below. |
| Flags | `u8` | Bitfield. Unknown flags are rejected. |
| Compression | `u8` | `0x00` none, `0x01` zstd. |
| Uncompressed length | unsigned varint | Length of the protobuf payload after decompression. |
| Payload | bytes | Raw or compressed protobuf payload. |
| CRC32 | `u32` little-endian | CRC over the uncompressed protobuf payload bytes. |

## Type Tags

| Object | Tag |
|---|---:|
| Blob | `0x01` |
| Tree | `0x02` |
| Patch | `0x03` |
| Revision | `0x04` |
| Snapshot | `0x05` |
| Intent | `0x06` |
| Change | `0x07` |
| Conflict | `0x08` |
| Capsule | `0x09` |
| Policy | `0x0a` |
| Workstream | `0x0b` |
| RefLog | `0x0c` |

## Canonical Payloads

- Payloads are Protocol Buffer encodings of the corresponding object message.
- Producers should write deterministic field values and stable ordering for repeated fields that represent sets.
- Decoders must reject unknown COF versions, invalid type tags, invalid compression markers, unknown flags, truncated varints, mismatched uncompressed lengths, decompression failures, and CRC mismatches.
- COF v1 validates payload integrity after decompression. Header, length, type, version, flags, and compression fields are validated structurally rather than included in the CRC domain.

## Object IDs

Object IDs are BLAKE3 hashes with domain separation:

```text
"claw\0" || type_tag || cof_version || canonical_payload
```

IDs are displayed as:

```text
clw_ + lowercase_base32(hash_bytes)
```

Type and version are included in the hash domain so two object kinds with identical protobuf bytes cannot collide into the same object identity.

## Compatibility Rules

- v0.1 writes COF v1.
- v0.1 rejects future COF versions.
- Readers may support older versions after a migration framework exists.
- Writers should default to the newest stable object format for the running release.
- Migration tools must preserve the original object ID mapping in their migration ledger.

## Test Vectors

Test vectors live under `tests/vectors/`:

- `core_cof_vectors.json`
- `crypto_capsule_vector.json`
- `patch_vectors.json`
- `policy_fail_closed_vectors.json`

Each vector records input shape, expected encoding or canonical representation, expected verification result, and object identity where applicable.

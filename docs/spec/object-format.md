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

Minimum valid encoded size is 12 bytes: 4-byte magic, 1-byte version, 1-byte
type tag, 1-byte flags, 1-byte compression marker, at least 1 byte of length,
and a 4-byte CRC trailer.

## Header Fields

### Magic Bytes

The first four bytes are ASCII `CLW1`:

```text
43 4c 57 31
```

Decoders must reject any other prefix before reading the remaining header.

### Version Byte

COF v1 uses `0x01`. The v0.1 reader classifies version bytes as:

- `0x01`: native read/write.
- greater than `0x01`: unsupported future version.
- less than the readable floor: unsupported past version.

### Flags

Known flag bits are:

| Bit | Mask | Meaning |
|---:|---:|---|
| 0 | `0x01` | Payload bytes are compressed. |
| 1 | `0x02` | Payload bytes are encrypted by a higher layer. |

The current encoder sets the compressed bit when zstd is used and does not set
the encrypted bit. Decoders must reject any flag bits outside `0x03`.

### Compression

The compression marker must agree with the payload encoding:

| Marker | Meaning |
|---:|---|
| `0x00` | Payload is raw protobuf bytes. |
| `0x01` | Payload is zstd-compressed protobuf bytes. |

The current encoder uses zstd level 3 when the uncompressed payload is larger
than 64 bytes. Decoders must reject unknown markers and decompression failures.

### Length Encoding

`Uncompressed length` is an unsigned base-128 varint. Each byte stores seven
payload bits; the high bit indicates continuation. Decoders must reject
truncated varints and varints that overflow 64 bits.

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
- Object payload bytes are hashed before COF wrapping. Compression changes the
  stored COF bytes but not the object ID.
- Empty protobuf payloads are valid for messages whose default value has no
  encoded fields; `tests/vectors/cof/blob_empty.json` records that case.
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

The display form uses RFC 4648 base32 without padding. Parsers require the
`clw_` prefix and accept either lowercase or uppercase base32 payload text.
Hex form is accepted by low-level tooling for fixtures and diagnostics, but
user-facing docs should prefer the `clw_` display form.

Type and version are included in the hash domain so two object kinds with identical protobuf bytes cannot collide into the same object identity.

## Compatibility Rules

- v0.1 writes COF v1.
- v0.1 rejects future COF versions.
- Readers may support older versions after a migration framework exists.
- Writers should default to the newest stable object format for the running release.
- Migration tools must preserve the original object ID mapping in their migration ledger.

## Test Vectors

Test vectors live under `tests/vectors/`:

- `cof/blob_empty.json`
- `cof/tree_single_file.json`
- `ids/revision_basic.json`
- `capsules/signed_basic.json`
- `policies/basic_required_checks.json`
- `core_cof_vectors.json`
- `crypto_capsule_vector.json`
- `patch_vectors.json`
- `policy_fail_closed_vectors.json`

Each vector records input shape, expected encoding or canonical representation, expected verification result, and object identity where applicable.

Required fields for new object vectors:

- `name`
- `input_object`
- `canonical_payload_hex` or a canonical JSON/protobuf representation
- `cof_hex` when the vector exercises COF bytes
- `expected_object_id_hex`
- `expected_object_id`
- `expected_signature_hex` when the vector exercises signatures
- `expected_verification_result`

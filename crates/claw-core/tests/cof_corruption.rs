use claw_core::cof::{cof_decode, cof_encode, cof_peek_type_tag};
use claw_core::id::ObjectId;
use claw_core::object::TypeTag;
use claw_core::CoreError;

fn valid_blob_cof(payload: &[u8]) -> Vec<u8> {
    cof_encode(TypeTag::Blob, payload).expect("encode test COF")
}

fn assert_deserialization_contains(result: Result<(TypeTag, Vec<u8>), CoreError>, needle: &str) {
    match result {
        Err(CoreError::Deserialization(message)) => {
            assert!(
                message.contains(needle),
                "deserialization message {message:?} should contain {needle:?}"
            );
        }
        other => panic!("expected deserialization error containing {needle:?}, got {other:?}"),
    }
}

#[test]
fn cof_rejects_corrupt_header_fields() {
    let data = valid_blob_cof(b"header field coverage");

    let mut bad_version = data.clone();
    bad_version[4] = 0x7f;
    assert!(matches!(
        cof_decode(&bad_version),
        Err(CoreError::UnsupportedVersion(0x7f))
    ));

    let mut bad_type = data.clone();
    bad_type[5] = 0xff;
    assert!(matches!(
        cof_decode(&bad_type),
        Err(CoreError::UnknownTypeTag(0xff))
    ));

    let mut bad_compression = data;
    bad_compression[7] = 0xff;
    assert_deserialization_contains(cof_decode(&bad_compression), "unknown compression");
}

#[test]
fn cof_rejects_truncated_markers_and_varints() {
    assert_deserialization_contains(cof_decode(b"CLW1\x01\x01\x00\x00"), "too short");
    assert!(matches!(cof_decode(&[0; 12]), Err(CoreError::InvalidMagic)));

    let mut unterminated_len = b"CLW1".to_vec();
    unterminated_len.extend_from_slice(&[0x01, TypeTag::Blob as u8, 0x00, 0x00]);
    unterminated_len.extend_from_slice(&[0x80, 0x80, 0x80, 0x80]);
    assert_deserialization_contains(cof_decode(&unterminated_len), "unexpected end of uvarint");

    let mut overflowing_len = b"CLW1".to_vec();
    overflowing_len.extend_from_slice(&[0x01, TypeTag::Blob as u8, 0x00, 0x00]);
    overflowing_len.extend_from_slice(&[0x80; 10]);
    overflowing_len.extend_from_slice(&[0x00; 4]);
    assert_deserialization_contains(cof_decode(&overflowing_len), "uvarint overflow");
}

#[test]
fn cof_rejects_crc_and_payload_field_corruption() {
    let mut truncated_crc = b"CLW1".to_vec();
    truncated_crc.extend_from_slice(&[0x01, TypeTag::Blob as u8, 0x00, 0x00, 0x00]);
    truncated_crc.extend_from_slice(&[0x00, 0x00, 0x00]);
    assert_deserialization_contains(cof_decode(&truncated_crc), "too short for CRC32");

    let mut bad_crc = valid_blob_cof(b"crc target");
    let last = bad_crc.len() - 1;
    bad_crc[last] ^= 0x01;
    assert!(matches!(
        cof_decode(&bad_crc),
        Err(CoreError::Crc32Mismatch { .. })
    ));

    let mut bad_compressed_len = valid_blob_cof(&[b'a'; 100]);
    bad_compressed_len[8] = 10;
    assert_deserialization_contains(cof_decode(&bad_compressed_len), "length mismatch");
}

#[test]
fn cof_peek_rejects_bad_markers_without_decoding_payload() {
    let mut data = valid_blob_cof(b"peek target");
    assert_eq!(cof_peek_type_tag(&data).expect("peek type"), TypeTag::Blob);

    data[0] = b'X';
    assert!(matches!(
        cof_peek_type_tag(&data),
        Err(CoreError::InvalidMagic)
    ));

    assert_deserialization_peek_contains(cof_peek_type_tag(&[0; 7]), "too short");
}

#[test]
fn object_id_parsing_rejects_malformed_display_and_hex_forms() {
    for display in [
        "",
        "clw_",
        "clw_not-base32",
        "sha256:0000",
        "clw_mzxw6ytboi",
    ] {
        assert!(
            matches!(
                ObjectId::from_display(display),
                Err(CoreError::InvalidObjectId(_))
            ),
            "display form should be rejected: {display}"
        );
    }

    for hex in ["", "00", "gg", "012345"] {
        assert!(
            matches!(ObjectId::from_hex(hex), Err(CoreError::InvalidObjectId(_))),
            "hex form should be rejected: {hex}"
        );
    }

    let id = ObjectId::from_bytes([0x42; 32]);
    assert_eq!(ObjectId::from_display(&id.to_string()).unwrap(), id);
    assert_eq!(ObjectId::from_hex(&id.to_hex()).unwrap(), id);
}

fn assert_deserialization_peek_contains(result: Result<TypeTag, CoreError>, needle: &str) {
    match result {
        Err(CoreError::Deserialization(message)) => {
            assert!(
                message.contains(needle),
                "deserialization message {message:?} should contain {needle:?}"
            );
        }
        other => panic!("expected deserialization error containing {needle:?}, got {other:?}"),
    }
}

use claw_core::cof::cof_decode;
use claw_core::hash::content_hash;
use claw_core::id::ObjectId;
use claw_core::object::TypeTag;
use serde_json::Value;

#[test]
fn standalone_launch_vectors_match_expected_hashes_and_cof() {
    let vectors = [
        include_str!("../../tests/vectors/cof/blob_empty.json"),
        include_str!("../../tests/vectors/cof/tree_single_file.json"),
        include_str!("../../tests/vectors/ids/revision_basic.json"),
        include_str!("../../tests/vectors/capsules/signed_basic.json"),
        include_str!("../../tests/vectors/policies/basic_required_checks.json"),
    ];

    for raw in vectors {
        let vector: Value = serde_json::from_str(raw).expect("vector is valid JSON");
        let name = vector["name"].as_str().expect("vector has name");
        let input_type = vector["input_object"]["type"]
            .as_str()
            .expect("vector has input object type");
        let tag = type_tag_from_vector(input_type);
        let payload = decode_hex(
            vector["canonical_payload_hex"]
                .as_str()
                .expect("vector has canonical payload"),
        );

        let expected_id_hex = vector["expected_object_id_hex"]
            .as_str()
            .expect("vector has expected object ID hex");
        let expected_id = ObjectId::from_hex(expected_id_hex).expect("expected ID hex is valid");
        assert_eq!(
            content_hash(tag, &payload),
            expected_id,
            "object ID vector changed: {name}"
        );
        assert_eq!(
            expected_id.to_string(),
            vector["expected_object_id"]
                .as_str()
                .expect("vector has display object ID"),
            "display object ID vector changed: {name}"
        );

        if let Some(cof_hex) = vector.get("cof_hex").and_then(Value::as_str) {
            let encoded = decode_hex(cof_hex);
            let (decoded_tag, decoded_payload) = cof_decode(&encoded).expect("COF vector decodes");
            assert_eq!(decoded_tag, tag, "COF tag vector changed: {name}");
            assert_eq!(
                decoded_payload, payload,
                "COF payload vector changed: {name}"
            );
        }

        if let Some(signature) = vector.get("expected_signature_hex").and_then(Value::as_str) {
            assert_eq!(
                signature.len(),
                128,
                "Ed25519 signature vector must be 64 bytes: {name}"
            );
        }
    }
}

fn type_tag_from_vector(input_type: &str) -> TypeTag {
    match input_type {
        "Blob" => TypeTag::Blob,
        "Tree" => TypeTag::Tree,
        "Revision" => TypeTag::Revision,
        "Capsule" => TypeTag::Capsule,
        "Policy" => TypeTag::Policy,
        other => panic!("unsupported vector object type: {other}"),
    }
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert!(
        input.len().is_multiple_of(2),
        "hex input must have even length"
    );
    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&input[i..i + 2], 16).expect("hex byte is valid"))
        .collect()
}

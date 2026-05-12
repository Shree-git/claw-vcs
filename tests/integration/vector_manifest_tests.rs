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

#[test]
fn documented_launch_vector_files_are_parseable_and_checked() {
    let core_cof: Value =
        serde_json::from_str(include_str!("../../tests/vectors/core_cof_vectors.json"))
            .expect("core COF vectors are valid JSON");
    let core_cof = core_cof.as_array().expect("core COF vectors are an array");
    assert!(
        !core_cof.is_empty(),
        "core COF vectors must include at least one case"
    );
    for vector in core_cof {
        let name = vector["name"].as_str().expect("COF vector has name");
        let tag = type_tag_from_vector(vector["type_tag"].as_str().expect("COF vector has tag"));
        let payload = decode_hex(
            vector["payload_hex"]
                .as_str()
                .expect("COF vector has payload"),
        );
        let encoded = decode_hex(vector["cof_hex"].as_str().expect("COF vector has bytes"));
        let (decoded_tag, decoded_payload) = cof_decode(&encoded).expect("COF vector decodes");
        assert_eq!(decoded_tag, tag, "COF tag changed: {name}");
        assert_eq!(decoded_payload, payload, "COF payload changed: {name}");
    }

    let crypto_capsule: Value = serde_json::from_str(include_str!(
        "../../tests/vectors/crypto_capsule_vector.json"
    ))
    .expect("crypto capsule vector is valid JSON");
    for key in [
        "signing_seed_hex",
        "encryption_key_hex",
        "revision_payload_hex",
        "private_hex",
        "agent_id",
    ] {
        assert!(
            crypto_capsule.get(key).is_some(),
            "crypto capsule vector must include {key}"
        );
    }

    let patch_vectors: Value =
        serde_json::from_str(include_str!("../../tests/vectors/patch_vectors.json"))
            .expect("patch vectors are valid JSON");
    assert!(
        patch_vectors["text_line_cases"]
            .as_array()
            .is_some_and(|cases| !cases.is_empty()),
        "patch vectors must include text line cases"
    );
    assert!(
        patch_vectors["json_tree_cases"]
            .as_array()
            .is_some_and(|cases| !cases.is_empty()),
        "patch vectors must include JSON tree cases"
    );

    let policy_vectors: Value = serde_json::from_str(include_str!(
        "../../tests/vectors/policy_fail_closed_vectors.json"
    ))
    .expect("policy fail-closed vectors are valid JSON");
    let policy_vectors = policy_vectors
        .as_array()
        .expect("policy vectors are an array");
    assert!(
        policy_vectors
            .iter()
            .any(|vector| vector["expected_error"] == "missing required check"),
        "policy vectors must include fail-closed missing-check coverage"
    );
}

fn type_tag_from_vector(input_type: &str) -> TypeTag {
    match input_type {
        "Blob" | "blob" => TypeTag::Blob,
        "Tree" | "tree" => TypeTag::Tree,
        "Patch" | "patch" => TypeTag::Patch,
        "Revision" | "revision" => TypeTag::Revision,
        "Capsule" | "capsule" => TypeTag::Capsule,
        "Policy" | "policy" => TypeTag::Policy,
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

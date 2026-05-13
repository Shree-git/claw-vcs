use claw_core::cof::{cof_decode, cof_version};
use claw_core::hash::content_hash;
use claw_core::id::ObjectId;
use claw_core::object::{Object, TypeTag};
use serde_json::Value;
use std::collections::HashSet;

#[test]
fn standalone_launch_vectors_match_expected_hashes_and_cof() {
    let vectors = [
        include_str!("../../tests/vectors/cof/blob_empty.json"),
        include_str!("../../tests/vectors/cof/tree_single_file.json"),
        include_str!("../../tests/vectors/ids/revision_basic.json"),
        include_str!("../../tests/vectors/capsules/signed_basic.json"),
        include_str!("../../tests/vectors/policies/basic_required_checks.json"),
    ];

    let mut covered_types = HashSet::new();

    for raw in vectors {
        let vector: Value = serde_json::from_str(raw).expect("vector is valid JSON");
        covered_types.insert(assert_object_vector(&vector));
    }

    let extended: Value = serde_json::from_str(include_str!(
        "../../tests/vectors/objects/core_object_types.json"
    ))
    .expect("extended core object vectors are valid JSON");
    let extended = extended
        .as_array()
        .expect("extended core object vectors are an array");
    for vector in extended {
        covered_types.insert(assert_object_vector(vector));
    }

    for tag in [
        TypeTag::Blob,
        TypeTag::Tree,
        TypeTag::Patch,
        TypeTag::Revision,
        TypeTag::Snapshot,
        TypeTag::Intent,
        TypeTag::Change,
        TypeTag::Conflict,
        TypeTag::Capsule,
        TypeTag::Policy,
        TypeTag::Workstream,
        TypeTag::RefLog,
    ] {
        assert!(
            covered_types.contains(&tag),
            "standalone launch vectors must cover object type: {}",
            tag.name()
        );
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

    let invalid_cof: Value =
        serde_json::from_str(include_str!("../../tests/vectors/cof/invalid_cases.json"))
            .expect("invalid COF vectors are valid JSON");
    let invalid_cof = invalid_cof
        .as_array()
        .expect("invalid COF vectors are an array");
    assert!(
        invalid_cof.len() >= 9,
        "invalid COF vectors must cover magic, version, type tag, flags, compression, uvarint, length, decompression, and CRC failures"
    );
    let mut invalid_case_names = HashSet::new();
    for vector in invalid_cof {
        let name = vector["name"]
            .as_str()
            .expect("invalid COF vector has name");
        invalid_case_names.insert(name);
        let encoded = decode_hex(vector["cof_hex"].as_str().expect("invalid COF has bytes"));
        let err = cof_decode(&encoded).expect_err("invalid COF vector must fail to decode");
        let rendered = err.to_string().to_lowercase();
        let expected = vector["expected_error"]
            .as_str()
            .expect("invalid COF vector has expected error");
        assert!(
            rendered.contains(expected),
            "invalid COF vector {name} should mention {expected}, got {rendered}"
        );
        if let Some(expected_version) = vector.get("expected_version").and_then(Value::as_u64) {
            assert_eq!(
                cof_version(&encoded).expect("future-version vector still has valid COF magic"),
                expected_version as u8,
                "invalid COF vector {name} version byte changed"
            );
        }
    }
    for required_name in [
        "invalid_magic",
        "future_version",
        "unknown_type_tag",
        "unknown_flags",
        "unknown_compression",
        "unterminated_uvarint",
        "length_mismatch",
        "bad_zstd_payload",
        "crc_mismatch",
    ] {
        assert!(
            invalid_case_names.contains(required_name),
            "invalid COF vector set must include {required_name}"
        );
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

    let core_object_types: Value = serde_json::from_str(include_str!(
        "../../tests/vectors/objects/core_object_types.json"
    ))
    .expect("core object type vectors are valid JSON");
    let core_object_types = core_object_types
        .as_array()
        .expect("core object type vectors are an array");
    assert_eq!(
        core_object_types.len(),
        7,
        "extended core object vectors must cover patch plus six object types not in the starter set"
    );
}

fn assert_object_vector(vector: &Value) -> TypeTag {
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
    let decoded =
        Object::deserialize_payload(tag, &payload).expect("canonical payload deserializes");
    assert_eq!(
        decoded.type_tag(),
        tag,
        "decoded vector type changed: {name}"
    );
    decoded
        .serialize_payload()
        .expect("decoded vector reserializes after default materialization");
    assert_input_object_matches(name, &decoded, &vector["input_object"]);
    assert!(
        vector["expected_verification_result"]
            .as_str()
            .expect("vector has expected verification result")
            .starts_with("valid"),
        "launch vectors should describe valid object fixtures: {name}"
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

    tag
}

fn assert_input_object_matches(name: &str, object: &Object, input: &Value) {
    match object {
        Object::Blob(blob) => {
            assert_eq!(
                blob.data,
                decode_hex(
                    input["data_hex"]
                        .as_str()
                        .expect("blob vector has data_hex")
                ),
                "blob data changed: {name}"
            );
            assert_eq!(
                blob.media_type.as_deref(),
                input.get("media_type").and_then(Value::as_str),
                "blob media type changed: {name}"
            );
        }
        Object::Tree(tree) => {
            let entries = input["entries"]
                .as_array()
                .expect("tree vector has entries");
            assert_eq!(
                tree.entries.len(),
                entries.len(),
                "tree entry count changed: {name}"
            );
            for (actual, expected) in tree.entries.iter().zip(entries) {
                assert_eq!(
                    actual.name,
                    expected["name"].as_str().expect("tree entry has name")
                );
                assert_eq!(
                    format!("{:?}", actual.mode).to_lowercase(),
                    expected["mode"]
                        .as_str()
                        .expect("tree entry has mode")
                        .to_lowercase()
                );
                assert_object_id_value(
                    &actual.object_id,
                    &expected["object_id"],
                    "tree entry object_id",
                    name,
                );
            }
        }
        Object::Patch(patch) => {
            assert_eq!(
                patch.target_path,
                input["target_path"].as_str().expect("patch target")
            );
            assert_eq!(
                patch.codec_id,
                input["codec_id"].as_str().expect("patch codec")
            );
            assert_optional_object_id(
                patch.base_object.as_ref(),
                input.get("base_object"),
                "patch base_object",
                name,
            );
            assert_optional_object_id(
                patch.result_object.as_ref(),
                input.get("result_object"),
                "patch result_object",
                name,
            );
            let ops = input["ops"].as_array().expect("patch vector has ops");
            assert_eq!(patch.ops.len(), ops.len(), "patch op count changed: {name}");
            for (actual, expected) in patch.ops.iter().zip(ops) {
                assert_eq!(
                    actual.address,
                    expected["address"].as_str().expect("op address")
                );
                assert_eq!(
                    actual.op_type,
                    expected["op_type"].as_str().expect("op type")
                );
                assert_eq!(
                    actual.old_data.as_deref(),
                    expected_bytes(expected, "old_data_utf8").as_deref(),
                    "patch old_data changed: {name}"
                );
                assert_eq!(
                    actual.new_data.as_deref(),
                    expected_bytes(expected, "new_data_utf8").as_deref(),
                    "patch new_data changed: {name}"
                );
                assert_eq!(
                    actual.context_hash,
                    expected.get("context_hash").and_then(Value::as_u64),
                    "patch context hash changed: {name}"
                );
            }
        }
        Object::Revision(revision) => {
            if let Some(change_id) = &revision.change_id {
                assert_eq!(
                    change_id.as_bytes().as_slice(),
                    decode_hex(input["change_id"].as_str().expect("revision change_id")).as_slice(),
                    "revision change_id changed: {name}"
                );
            }
            assert_object_id_list(
                &revision.parents,
                &input["parents"],
                "revision parents",
                name,
            );
            assert_optional_object_id(
                revision.tree.as_ref(),
                input.get("tree"),
                "revision tree",
                name,
            );
            assert_eq!(
                revision.author,
                input["author"].as_str().expect("revision author")
            );
            assert_eq!(
                revision.created_at_ms,
                input["created_at_ms"].as_u64().expect("revision timestamp")
            );
            assert_eq!(
                revision.summary,
                input["summary"].as_str().expect("revision summary")
            );
            assert_string_list(
                &revision.policy_evidence,
                &input["policy_evidence"],
                "revision policy evidence",
                name,
            );
        }
        Object::Snapshot(snapshot) => {
            assert_object_id_value(
                &snapshot.tree_root,
                &input["tree_root"],
                "snapshot tree_root",
                name,
            );
            assert_object_id_value(
                &snapshot.revision_id,
                &input["revision_id"],
                "snapshot revision_id",
                name,
            );
            assert_eq!(
                snapshot.created_at_ms,
                input["created_at_ms"].as_u64().expect("snapshot timestamp")
            );
        }
        Object::Intent(intent) => {
            assert_eq!(
                intent.id.as_bytes().as_slice(),
                decode_hex(input["id_hex"].as_str().expect("intent id_hex")).as_slice(),
                "intent id changed: {name}"
            );
            assert_eq!(intent.title, input["title"].as_str().expect("intent title"));
            assert_eq!(intent.goal, input["goal"].as_str().expect("intent goal"));
            assert_string_list(
                &intent.constraints,
                &input["constraints"],
                "intent constraints",
                name,
            );
            assert_string_list(
                &intent.acceptance_tests,
                &input["acceptance_tests"],
                "intent acceptance tests",
                name,
            );
            assert_string_list(&intent.links, &input["links"], "intent links", name);
            assert_string_list(
                &intent.policy_refs,
                &input["policy_refs"],
                "intent policies",
                name,
            );
            assert_string_list(&intent.agents, &input["agents"], "intent agents", name);
            assert_string_list(
                &intent.change_ids,
                &input["change_ids"],
                "intent changes",
                name,
            );
            assert_eq!(
                format!("{:?}", intent.status).to_lowercase(),
                input["status"].as_str().expect("intent status")
            );
        }
        Object::Change(change) => {
            assert_eq!(
                change.id.as_bytes().as_slice(),
                decode_hex(input["id_hex"].as_str().expect("change id_hex")).as_slice(),
                "change id changed: {name}"
            );
            assert_eq!(
                change.intent_id.as_bytes().as_slice(),
                decode_hex(
                    input["intent_id_hex"]
                        .as_str()
                        .expect("change intent_id_hex")
                )
                .as_slice(),
                "change intent id changed: {name}"
            );
            assert_optional_object_id(
                change.head_revision.as_ref(),
                input.get("head_revision"),
                "change head_revision",
                name,
            );
            assert_eq!(
                change.workstream_id.as_deref(),
                input.get("workstream_id").and_then(Value::as_str),
                "change workstream changed: {name}"
            );
            assert_eq!(
                format!("{:?}", change.status).to_lowercase(),
                input["status"].as_str().expect("change status")
            );
        }
        Object::Conflict(conflict) => {
            assert_optional_object_id(
                conflict.base_revision.as_ref(),
                input.get("base_revision"),
                "conflict base_revision",
                name,
            );
            assert_object_id_value(
                &conflict.left_revision,
                &input["left_revision"],
                "conflict left_revision",
                name,
            );
            assert_object_id_value(
                &conflict.right_revision,
                &input["right_revision"],
                "conflict right_revision",
                name,
            );
            assert_eq!(
                conflict.file_path,
                input["file_path"].as_str().expect("conflict path")
            );
            assert_eq!(
                conflict.codec_id,
                input["codec_id"].as_str().expect("conflict codec")
            );
            assert_object_id_list(
                &conflict.left_patch_ids,
                &input["left_patch_ids"],
                "conflict left patches",
                name,
            );
            assert_object_id_list(
                &conflict.right_patch_ids,
                &input["right_patch_ids"],
                "conflict right patches",
                name,
            );
            assert_eq!(
                format!("{:?}", conflict.status).to_lowercase(),
                input["status"].as_str().expect("conflict status")
            );
        }
        Object::Capsule(capsule) => {
            assert_object_id_value(
                &capsule.revision_id,
                &input["revision_id"],
                "capsule revision",
                name,
            );
            assert_eq!(
                capsule.public_fields.agent_id,
                input["agent_id"].as_str().expect("capsule agent")
            );
            let evidence = input["evidence"].as_array().expect("capsule evidence");
            assert_eq!(
                capsule.public_fields.evidence.len(),
                evidence.len(),
                "capsule evidence count changed: {name}"
            );
            for (actual, expected) in capsule.public_fields.evidence.iter().zip(evidence) {
                assert_eq!(
                    actual.name,
                    expected["name"].as_str().expect("evidence name")
                );
                assert_eq!(
                    actual.status,
                    expected["status"].as_str().expect("evidence status")
                );
                assert_eq!(
                    actual.duration_ms,
                    expected["duration_ms"].as_u64().expect("evidence duration")
                );
            }
        }
        Object::Policy(policy) => {
            assert_eq!(
                policy.policy_id,
                input["policy_id"].as_str().expect("policy id")
            );
            assert_string_list(
                &policy.required_checks,
                &input["required_checks"],
                "policy required checks",
                name,
            );
            assert_eq!(
                format!("{:?}", policy.visibility).to_lowercase(),
                input["visibility"].as_str().expect("policy visibility")
            );
        }
        Object::Workstream(workstream) => {
            assert_eq!(
                workstream.workstream_id,
                input["workstream_id"].as_str().expect("workstream id")
            );
            let actual: Vec<String> = workstream
                .change_stack
                .iter()
                .map(ToString::to_string)
                .collect();
            assert_string_list(&actual, &input["change_stack"], "workstream changes", name);
        }
        Object::RefLog(reflog) => {
            assert_eq!(
                reflog.ref_name,
                input["ref_name"].as_str().expect("reflog ref")
            );
            let entries = input["entries"].as_array().expect("reflog entries");
            assert_eq!(
                reflog.entries.len(),
                entries.len(),
                "reflog entry count changed: {name}"
            );
            for (actual, expected) in reflog.entries.iter().zip(entries) {
                assert_optional_object_id(
                    actual.old_target.as_ref(),
                    expected.get("old_target"),
                    "reflog old_target",
                    name,
                );
                assert_object_id_value(
                    &actual.new_target,
                    &expected["new_target"],
                    "reflog new_target",
                    name,
                );
                assert_eq!(
                    actual.author,
                    expected["author"].as_str().expect("reflog author")
                );
                assert_eq!(
                    actual.message,
                    expected["message"].as_str().expect("reflog message")
                );
                assert_eq!(
                    actual.timestamp,
                    expected["timestamp"].as_u64().expect("reflog timestamp")
                );
            }
        }
    }
}

fn assert_object_id_value(actual: &ObjectId, expected: &Value, field: &str, name: &str) {
    let expected = expected
        .as_str()
        .unwrap_or_else(|| panic!("{field} should be a string for {name}"));
    assert_eq!(
        actual.to_string(),
        expected,
        "{field} display value changed: {name}"
    );
}

fn assert_optional_object_id(
    actual: Option<&ObjectId>,
    expected: Option<&Value>,
    field: &str,
    name: &str,
) {
    match (actual, expected.and_then(Value::as_str)) {
        (Some(actual), Some(expected)) => assert_eq!(
            actual.to_string(),
            expected,
            "{field} display value changed: {name}"
        ),
        (None, None) => {}
        (left, right) => panic!("{field} optionality changed for {name}: {left:?} vs {right:?}"),
    }
}

fn assert_object_id_list(actual: &[ObjectId], expected: &Value, field: &str, name: &str) {
    let expected = expected
        .as_array()
        .unwrap_or_else(|| panic!("{field} should be an array for {name}"));
    assert_eq!(
        actual.len(),
        expected.len(),
        "{field} length changed: {name}"
    );
    for (actual, expected) in actual.iter().zip(expected) {
        assert_object_id_value(actual, expected, field, name);
    }
}

fn assert_string_list(actual: &[String], expected: &Value, field: &str, name: &str) {
    let expected: Vec<&str> = expected
        .as_array()
        .unwrap_or_else(|| panic!("{field} should be an array for {name}"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{field} item should be a string"))
        })
        .collect();
    assert_eq!(
        actual.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "{field} changed: {name}"
    );
}

fn expected_bytes(input: &Value, key: &str) -> Option<Vec<u8>> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.as_bytes().to_vec())
}

fn type_tag_from_vector(input_type: &str) -> TypeTag {
    match input_type {
        "Blob" | "blob" => TypeTag::Blob,
        "Tree" | "tree" => TypeTag::Tree,
        "Patch" | "patch" => TypeTag::Patch,
        "Revision" | "revision" => TypeTag::Revision,
        "Snapshot" | "snapshot" => TypeTag::Snapshot,
        "Intent" | "intent" => TypeTag::Intent,
        "Change" | "change" => TypeTag::Change,
        "Conflict" | "conflict" => TypeTag::Conflict,
        "Capsule" | "capsule" => TypeTag::Capsule,
        "Policy" | "policy" => TypeTag::Policy,
        "Workstream" | "workstream" => TypeTag::Workstream,
        "RefLog" | "reflog" => TypeTag::RefLog,
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

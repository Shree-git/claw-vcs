use claw_core::cof::{cof_decode, cof_encode};
use claw_core::hash::content_hash;
use claw_core::id::{ChangeId, IntentId, ObjectId};
use claw_core::object::{Object, TypeTag};
use claw_core::types::{
    validate_tree_entry_name, Blob, Capsule, CapsulePublic, CapsuleSignature, Change, ChangeStatus,
    Conflict, ConflictStatus, Evidence, EvidencePolicy, FileMode, Intent, IntentStatus, Patch,
    PatchOp, Policy, RefLog, RefLogEntry, Revision, Snapshot, Tree, TreeEntry, Visibility,
    Workstream,
};
use proptest::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CofVector {
    name: String,
    type_tag: String,
    payload_hex: String,
    cof_hex: String,
}

#[test]
fn cof_vectors_are_stable() {
    let vectors: Vec<CofVector> =
        serde_json::from_str(include_str!("../../../tests/vectors/core_cof_vectors.json")).unwrap();

    for vector in vectors {
        let tag = type_tag_from_name(&vector.type_tag);
        let payload = decode_hex(&vector.payload_hex);
        let expected = decode_hex(&vector.cof_hex);
        let encoded = cof_encode(tag, &payload).unwrap();

        assert_eq!(encoded, expected, "COF vector changed: {}", vector.name);
        let (decoded_tag, decoded_payload) = cof_decode(&expected).unwrap();
        assert_eq!(decoded_tag, tag, "COF vector tag changed: {}", vector.name);
        assert_eq!(
            decoded_payload, payload,
            "COF vector payload changed: {}",
            vector.name
        );
    }
}

proptest! {
    #[test]
    fn cof_roundtrips_arbitrary_payloads(tag_value in 1u8..=12, payload in prop::collection::vec(any::<u8>(), 0..2048)) {
        let tag = TypeTag::from_u8(tag_value).unwrap();
        let encoded = cof_encode(tag, &payload).unwrap();
        let (decoded_tag, decoded_payload) = cof_decode(&encoded).unwrap();

        prop_assert_eq!(decoded_tag, tag);
        prop_assert_eq!(decoded_payload, payload);
    }

    #[test]
    fn object_id_display_roundtrips(bytes in any::<[u8; 32]>()) {
        let id = ObjectId::from_bytes(bytes);
        let display = id.to_string();
        let decoded = ObjectId::from_display(&display).unwrap();

        prop_assert_eq!(decoded, id);
        prop_assert_eq!(ObjectId::from_hex(&id.to_hex()).unwrap(), id);
    }

    #[test]
    fn object_payload_roundtrip_keeps_canonical_encoding(
        selector in 0u8..12,
        data in prop::collection::vec(any::<u8>(), 0..128),
        raw_name in "[A-Za-z0-9._-]{1,16}"
    ) {
        let object = object_for(selector, &data, &safe_tree_name(&raw_name));
        let payload = object.serialize_payload().unwrap();
        let decoded = Object::deserialize_payload(object.type_tag(), &payload).unwrap();
        let recoded = decoded.serialize_payload().unwrap();

        prop_assert_eq!(recoded, payload);
    }

    #[test]
    fn object_dependencies_are_sorted_unique_and_survive_roundtrip(
        selector in 0u8..12,
        data in prop::collection::vec(any::<u8>(), 0..128),
        raw_name in "[A-Za-z0-9._-]{1,16}"
    ) {
        let object = object_for(selector, &data, &safe_tree_name(&raw_name));
        let payload = object.serialize_payload().unwrap();
        let decoded = Object::deserialize_payload(object.type_tag(), &payload).unwrap();
        let dependencies = decoded.dependencies();
        let mut expected = dependencies.clone();
        expected.sort_by_key(|id| id.to_hex());
        expected.dedup();

        prop_assert_eq!(dependencies, expected);
        let dependencies = decoded.dependencies();
        prop_assert_eq!(dependencies, object.dependencies());
    }
}

fn object_for(selector: u8, data: &[u8], name: &str) -> Object {
    let id_a = content_hash(TypeTag::Blob, data);
    let id_b = content_hash(TypeTag::Tree, &non_empty(data, 0x42));
    let intent_id = IntentId::from_bytes([selector; 16]);
    let change_id = ChangeId::from_bytes([selector.wrapping_add(1); 16]);
    let policy_visibility = match selector % 3 {
        0 => Visibility::Public,
        1 => Visibility::Private,
        _ => Visibility::EncryptedMetadataRequired,
    };

    match selector % 12 {
        0 => Object::Blob(Blob {
            data: data.to_vec(),
            media_type: Some("application/octet-stream".to_string()),
        }),
        1 => Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: name.to_string(),
                mode: FileMode::Regular,
                object_id: id_a,
            }],
        }),
        2 => Object::Patch(Patch {
            target_path: format!("src/{name}"),
            codec_id: "binary".to_string(),
            base_object: Some(id_a),
            result_object: Some(id_b),
            ops: vec![PatchOp {
                address: "B0".to_string(),
                op_type: "replace".to_string(),
                old_data: Some(non_empty(data, 0x01)),
                new_data: Some(non_empty(data, 0x02)),
                context_hash: Some(1),
            }],
            codec_payload: Some(non_empty(data, 0x03)),
        }),
        3 => Object::Revision(Revision {
            change_id: Some(change_id),
            parents: vec![id_a],
            patches: vec![id_b],
            snapshot_base: Some(id_a),
            tree: Some(id_b),
            capsule_id: Some(id_a),
            author: "agent@example.com".to_string(),
            created_at_ms: 1_700_000_000_000,
            summary: name.to_string(),
            policy_evidence: vec!["policy-ok".to_string()],
        }),
        4 => Object::Snapshot(Snapshot {
            tree_root: id_a,
            revision_id: id_b,
            created_at_ms: 1_700_000_000_001,
        }),
        5 => Object::Intent(Intent {
            id: intent_id,
            title: name.to_string(),
            goal: "prove serialization roundtrips".to_string(),
            constraints: vec!["deterministic".to_string()],
            acceptance_tests: vec!["cargo test -p claw-vcs-core".to_string()],
            links: vec!["claw://test-vector".to_string()],
            policy_refs: vec!["public-launch".to_string()],
            agents: vec!["codex".to_string()],
            change_ids: vec![change_id.to_string()],
            depends_on: vec![],
            supersedes: vec![],
            status: IntentStatus::Open,
            created_at_ms: 1_700_000_000_002,
            updated_at_ms: 1_700_000_000_003,
        }),
        6 => Object::Change(Change {
            id: change_id,
            intent_id,
            head_revision: Some(id_a),
            workstream_id: Some("launch-hardening".to_string()),
            status: ChangeStatus::Ready,
            created_at_ms: 1_700_000_000_004,
            updated_at_ms: 1_700_000_000_005,
        }),
        7 => Object::Conflict(Conflict {
            base_revision: Some(id_a),
            left_revision: id_b,
            right_revision: id_a,
            file_path: format!("src/{name}"),
            codec_id: "text/line".to_string(),
            left_patch_ids: vec![id_a],
            right_patch_ids: vec![id_b],
            resolution_patch_ids: vec![],
            status: ConflictStatus::Open,
            created_at_ms: 1_700_000_000_006,
        }),
        8 => Object::Capsule(Capsule {
            revision_id: id_a,
            public_fields: CapsulePublic {
                agent_id: "codex".to_string(),
                agent_version: Some("test".to_string()),
                toolchain_digest: Some("sha256:test".to_string()),
                env_fingerprint: Some("linux-test".to_string()),
                evidence: vec![Evidence {
                    name: "ci".to_string(),
                    status: "pass".to_string(),
                    duration_ms: 42,
                    artifact_refs: vec!["artifact://unit".to_string()],
                    summary: Some("ok".to_string()),
                    revision_id: Some(id_a),
                    command: Some("cargo test".to_string()),
                    exit_code: Some(0),
                    started_at_ms: Some(1_700_000_000_006),
                    ended_at_ms: Some(1_700_000_000_007),
                    environment_digest: Some("sha256:env".to_string()),
                    runner_identity: Some("runner-a".to_string()),
                    log_digest: Some("sha256:log".to_string()),
                    artifact_digest: None,
                    expires_at_ms: Some(1_700_000_086_407),
                    trust_domain: Some("ci".to_string()),
                    signature: Some(non_empty(data, 0x06)),
                }],
            },
            encrypted_private: Some(non_empty(data, 0x04)),
            encryption: "xchacha20poly1305".to_string(),
            key_id: Some("test-key".to_string()),
            recipients: vec![],
            signatures: vec![CapsuleSignature {
                signer_id: "signer".to_string(),
                signature: non_empty(data, 0x05),
            }],
        }),
        9 => Object::Policy(Policy {
            policy_id: "public-launch".to_string(),
            required_checks: vec!["ci".to_string()],
            required_reviewers: vec!["release".to_string()],
            sensitive_paths: vec!["secrets/".to_string()],
            quarantine_lane: selector.is_multiple_of(2),
            min_trust_score: Some("0.75".to_string()),
            visibility: policy_visibility,
            authorized_recipients: vec!["security".to_string()],
            revoked_recipients: vec![],
            evidence_policy: EvidencePolicy {
                require_fresh_evidence: true,
                trusted_runner_identities: vec!["runner-a".to_string()],
                ..EvidencePolicy::default()
            },
        }),
        10 => Object::Workstream(Workstream {
            workstream_id: "ws-launch".to_string(),
            change_stack: vec![change_id],
        }),
        _ => Object::RefLog(RefLog {
            ref_name: "heads/main".to_string(),
            entries: vec![RefLogEntry {
                old_target: Some(id_a),
                new_target: id_b,
                author: "codex".to_string(),
                message: "advance".to_string(),
                timestamp: 1_700_000_000_007,
            }],
        }),
    }
}

fn safe_tree_name(raw: &str) -> String {
    let mut candidate = raw.trim_end_matches([' ', '.']).to_string();
    if candidate.is_empty() || candidate == "." || candidate == ".." {
        candidate = "file".to_string();
    }
    if validate_tree_entry_name(&candidate).is_err() {
        candidate = format!("_{candidate}");
    }
    validate_tree_entry_name(&candidate)
        .expect("proptest tree name sanitizer must produce valid names");
    candidate
}

fn non_empty(data: &[u8], fallback: u8) -> Vec<u8> {
    if data.is_empty() {
        vec![fallback]
    } else {
        data.to_vec()
    }
}

fn type_tag_from_name(name: &str) -> TypeTag {
    match name {
        "blob" => TypeTag::Blob,
        "tree" => TypeTag::Tree,
        "patch" => TypeTag::Patch,
        "revision" => TypeTag::Revision,
        "snapshot" => TypeTag::Snapshot,
        "intent" => TypeTag::Intent,
        "change" => TypeTag::Change,
        "conflict" => TypeTag::Conflict,
        "capsule" => TypeTag::Capsule,
        "policy" => TypeTag::Policy,
        "workstream" => TypeTag::Workstream,
        "reflog" => TypeTag::RefLog,
        other => panic!("unknown type tag in vector: {other}"),
    }
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert!(
        input.len().is_multiple_of(2),
        "hex input must have even length"
    );
    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&input[i..i + 2], 16).unwrap())
        .collect()
}

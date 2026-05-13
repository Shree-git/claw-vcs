use claw_core::cof::cof_decode;
use claw_core::id::{ChangeId, IntentId, ObjectId};
use claw_core::object::Object;
use claw_core::object::TypeTag;
use claw_core::types::{
    Blob, Capsule, CapsulePublic, Change, ChangeStatus, Conflict, ConflictStatus, Evidence,
    FileMode, Intent, IntentStatus, Patch, PatchOp, Policy, RefLog, RefLogEntry, Revision,
    Snapshot, Tree, TreeEntry, Visibility, Workstream,
};
use claw_store::layout::RepoLayout;
use claw_store::loose::loose_object_path;
use claw_store::pack::{read_object_from_pack, read_pack_index, PackWriter};
use claw_store::ClawStore;
use proptest::prelude::*;

proptest! {
    #[test]
    fn store_load_blob_roundtrips(
        data in prop::collection::vec(any::<u8>(), 0..2048),
        media in prop::option::of("[a-z]+/[a-z0-9.+-]+")
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let object = Object::Blob(Blob {
            data: data.clone(),
            media_type: media.clone(),
        });

        let id = store.store_object(&object).unwrap();
        prop_assert!(store.has_object(&id));
        prop_assert!(store.list_object_ids().unwrap().contains(&id));

        let raw = store.load_cof_bytes(&id).unwrap();
        let (tag, _payload) = cof_decode(&raw).unwrap();
        prop_assert_eq!(tag, TypeTag::Blob);

        match store.load_object(&id).unwrap() {
            Object::Blob(blob) => {
                prop_assert_eq!(blob.data, data);
                prop_assert_eq!(blob.media_type, media);
            }
            other => prop_assert!(false, "loaded unexpected object: {:?}", other.type_tag()),
        }
    }

    #[test]
    fn store_load_all_core_object_types_roundtrip(
        selector in 0u8..12,
        data in prop::collection::vec(any::<u8>(), 0..512)
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let object = object_for(selector, &data);

        let id = store.store_object(&object).unwrap();
        prop_assert!(store.has_object(&id));

        let loaded = store.load_object(&id).unwrap();
        prop_assert_eq!(loaded.type_tag(), object.type_tag());
        prop_assert_eq!(
            loaded.serialize_payload().unwrap(),
            object.serialize_payload().unwrap()
        );
    }
}

fn object_for(selector: u8, data: &[u8]) -> Object {
    let id_a = ObjectId::from_bytes([selector; 32]);
    let id_b = ObjectId::from_bytes([selector.wrapping_add(1); 32]);
    let intent_id = IntentId::from_bytes([selector; 16]);
    let change_id = ChangeId::from_bytes([selector.wrapping_add(1); 16]);

    match selector % 12 {
        0 => Object::Blob(Blob {
            data: data.to_vec(),
            media_type: None,
        }),
        1 => Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: "file.bin".to_string(),
                mode: FileMode::Regular,
                object_id: id_a,
            }],
        }),
        2 => Object::Patch(Patch {
            target_path: "file.bin".to_string(),
            codec_id: "bytes/test".to_string(),
            base_object: Some(id_a),
            result_object: Some(id_b),
            ops: vec![PatchOp {
                address: "0".to_string(),
                op_type: "replace".to_string(),
                old_data: Some(Vec::new()),
                new_data: Some(data.to_vec()),
                context_hash: Some(data.len() as u64),
            }],
            codec_payload: None,
        }),
        3 => Object::Revision(Revision {
            change_id: Some(change_id),
            parents: vec![id_a],
            patches: vec![id_b],
            snapshot_base: None,
            tree: Some(id_a),
            capsule_id: None,
            author: "store-prop".to_string(),
            created_at_ms: data.len() as u64,
            summary: "roundtrip".to_string(),
            policy_evidence: vec![],
        }),
        4 => Object::Snapshot(Snapshot {
            tree_root: id_a,
            revision_id: id_b,
            created_at_ms: data.len() as u64,
        }),
        5 => Object::Intent(Intent {
            id: intent_id,
            title: "store property".to_string(),
            goal: "roundtrip all object types".to_string(),
            constraints: vec![],
            acceptance_tests: vec![],
            links: vec![],
            policy_refs: vec![],
            agents: vec![],
            change_ids: vec![change_id.to_string()],
            depends_on: vec![],
            supersedes: vec![],
            status: IntentStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 2,
        }),
        6 => Object::Change(Change {
            id: change_id,
            intent_id,
            head_revision: Some(id_a),
            workstream_id: Some("store-prop".to_string()),
            status: ChangeStatus::Ready,
            created_at_ms: 1,
            updated_at_ms: 2,
        }),
        7 => Object::Conflict(Conflict {
            base_revision: Some(id_a),
            left_revision: id_a,
            right_revision: id_b,
            file_path: "file.bin".to_string(),
            codec_id: "bytes/test".to_string(),
            left_patch_ids: vec![id_a],
            right_patch_ids: vec![id_b],
            resolution_patch_ids: vec![],
            status: ConflictStatus::Open,
            created_at_ms: 1,
        }),
        8 => Object::Capsule(Capsule {
            revision_id: id_a,
            public_fields: CapsulePublic {
                agent_id: "store-prop".to_string(),
                agent_version: None,
                toolchain_digest: None,
                env_fingerprint: None,
                evidence: vec![Evidence {
                    name: "test".to_string(),
                    status: "pass".to_string(),
                    duration_ms: data.len() as u64,
                    artifact_refs: vec![],
                    summary: None,
                    revision_id: Some(id_a),
                    command: None,
                    exit_code: None,
                    started_at_ms: None,
                    ended_at_ms: None,
                    environment_digest: None,
                    runner_identity: None,
                    log_digest: None,
                    artifact_digest: None,
                    expires_at_ms: None,
                    trust_domain: None,
                    signature: None,
                }],
            },
            encrypted_private: None,
            encryption: String::new(),
            key_id: None,
            recipients: vec![],
            signatures: vec![],
        }),
        9 => Object::Policy(Policy {
            policy_id: "store-prop".to_string(),
            required_checks: vec!["test".to_string()],
            required_reviewers: vec![],
            sensitive_paths: vec![],
            quarantine_lane: false,
            min_trust_score: None,
            visibility: Visibility::Public,
            authorized_recipients: vec![],
            revoked_recipients: vec![],
            evidence_policy: Default::default(),
        }),
        10 => Object::Workstream(Workstream {
            workstream_id: "store-prop".to_string(),
            change_stack: vec![change_id],
        }),
        _ => Object::RefLog(RefLog {
            ref_name: "heads/main".to_string(),
            entries: vec![RefLogEntry {
                old_target: Some(id_a),
                new_target: id_b,
                author: "store-prop".to_string(),
                message: "advance".to_string(),
                timestamp: data.len() as u64,
            }],
        }),
    }
}

#[test]
fn corrupt_loose_object_fails_to_load() {
    let tmp = tempfile::tempdir().unwrap();
    let store = ClawStore::init(tmp.path()).unwrap();
    let id = store
        .store_object(&Object::Blob(Blob {
            data: b"tamper target".to_vec(),
            media_type: None,
        }))
        .unwrap();
    let path = loose_object_path(store.layout(), &id);
    let mut raw = std::fs::read(&path).unwrap();
    let last = raw.len() - 1;
    raw[last] ^= 0x01;
    std::fs::write(&path, raw).unwrap();

    assert!(store.load_object(&id).is_err());
}

#[test]
fn pack_index_vectors_roundtrip_objects() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = RepoLayout::new(tmp.path());
    layout.create_dirs().unwrap();
    let mut writer = PackWriter::new();
    let first = Object::Blob(Blob {
        data: b"first".to_vec(),
        media_type: None,
    });
    let second = Object::Blob(Blob {
        data: b"second".to_vec(),
        media_type: Some("text/plain".to_string()),
    });
    let first_id = writer.add_object(&first).unwrap();
    let second_id = writer.add_object(&second).unwrap();
    let (pack_path, idx_path) = writer.write_pack(&layout).unwrap();

    let index = read_pack_index(&idx_path).unwrap();
    assert_eq!(index.len(), 2);
    assert_eq!(index[0].0, first_id);
    assert_eq!(index[1].0, second_id);

    let first_loaded = read_object_from_pack(&pack_path, index[0].1).unwrap();
    let second_loaded = read_object_from_pack(&pack_path, index[1].1).unwrap();
    assert!(matches!(first_loaded, Object::Blob(Blob { data, .. }) if data.as_slice() == b"first"));
    assert!(
        matches!(second_loaded, Object::Blob(Blob { data, .. }) if data.as_slice() == b"second")
    );
}

#[test]
fn corrupt_pack_index_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = RepoLayout::new(tmp.path());
    layout.create_dirs().unwrap();
    let mut writer = PackWriter::new();
    writer
        .add_object(&Object::Blob(Blob {
            data: b"indexed".to_vec(),
            media_type: None,
        }))
        .unwrap();
    let (_pack_path, idx_path) = writer.write_pack(&layout).unwrap();

    let mut idx = std::fs::read(&idx_path).unwrap();
    idx.truncate(idx.len() - 1);
    std::fs::write(&idx_path, idx).unwrap();

    let err = read_pack_index(&idx_path).expect_err("truncated index must fail");
    assert!(err.to_string().contains("truncated pack index"));
}

#[test]
fn corrupt_pack_object_entry_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let layout = RepoLayout::new(tmp.path());
    layout.create_dirs().unwrap();
    let mut writer = PackWriter::new();
    writer
        .add_object(&Object::Blob(Blob {
            data: b"packed".to_vec(),
            media_type: None,
        }))
        .unwrap();
    let (pack_path, idx_path) = writer.write_pack(&layout).unwrap();
    let index = read_pack_index(&idx_path).unwrap();

    let mut pack = std::fs::read(&pack_path).unwrap();
    pack.truncate(pack.len() - 1);
    std::fs::write(&pack_path, pack).unwrap();

    let err =
        read_object_from_pack(&pack_path, index[0].1).expect_err("truncated pack object must fail");
    assert!(err.to_string().contains("truncated pack object entry"));
}

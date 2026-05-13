#![no_main]

use claw_core::id::{ChangeId, IntentId, ObjectId};
use claw_core::object::Object;
use claw_core::types::{
    Blob, Capsule, CapsulePublic, Change, ChangeStatus, Conflict, ConflictStatus, Evidence,
    FileMode, Intent, IntentStatus, Patch, PatchOp, Policy, RefLog, RefLogEntry, Revision,
    Snapshot, Tree, TreeEntry, Visibility, Workstream,
};
use claw_store::pack::{read_object_from_pack, read_pack_index, PackWriter};
use claw_store::ClawStore;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let capped = &data[..data.len().min(4096)];
    if let Ok(tmp) = tempfile::tempdir() {
        if let Ok(store) = ClawStore::init(tmp.path()) {
            let object = object_from_seed(capped);
            if let Ok(id) = store.store_object(&object) {
                let _ = store.has_object(&id);
                let _ = store.load_cof_bytes(&id);
                let _ = store.load_object(&id);
                let _ = store.list_object_ids();
            }

            let mut pack = PackWriter::new();
            if pack.add_object(&object).is_ok() {
                if let Ok((pack_path, index_path)) = pack.write_pack(store.layout()) {
                    if let Ok(index) = read_pack_index(&index_path) {
                        for (_, offset) in index {
                            let _ = read_object_from_pack(&pack_path, offset);
                        }
                    }
                }
            }
        }
    }
});

fn object_from_seed(data: &[u8]) -> Object {
    let selector = data.first().copied().unwrap_or_default() % 12;
    let text = String::from_utf8_lossy(&data[..data.len().min(64)]).to_string();
    match selector {
        0 => Object::Blob(Blob {
            data: data.to_vec(),
            media_type: Some("application/octet-stream".to_string()),
        }),
        1 => Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: "file.bin".to_string(),
                mode: FileMode::Regular,
                object_id: oid(1),
            }],
        }),
        2 => Object::Patch(Patch {
            target_path: "file.bin".to_string(),
            codec_id: "bytes/test".to_string(),
            base_object: Some(oid(1)),
            result_object: Some(oid(2)),
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
            change_id: Some(change_id(1)),
            parents: vec![oid(1)],
            patches: vec![oid(2)],
            snapshot_base: None,
            tree: Some(oid(3)),
            capsule_id: None,
            author: text,
            created_at_ms: data.len() as u64,
            summary: "fuzz revision".to_string(),
            policy_evidence: vec!["test=pass".to_string()],
        }),
        4 => Object::Snapshot(Snapshot {
            tree_root: oid(1),
            revision_id: oid(2),
            created_at_ms: data.len() as u64,
        }),
        5 => Object::Intent(Intent {
            id: intent_id(1),
            title: "fuzz intent".to_string(),
            goal: text,
            constraints: vec!["bounded".to_string()],
            acceptance_tests: vec!["store roundtrip".to_string()],
            links: vec![],
            policy_refs: vec!["default".to_string()],
            agents: vec!["fuzzer".to_string()],
            change_ids: vec![change_id(1).to_string()],
            depends_on: vec![],
            supersedes: vec![],
            status: IntentStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 2,
        }),
        6 => Object::Change(Change {
            id: change_id(1),
            intent_id: intent_id(1),
            head_revision: Some(oid(1)),
            workstream_id: Some("fuzz".to_string()),
            status: ChangeStatus::Ready,
            created_at_ms: 1,
            updated_at_ms: 2,
        }),
        7 => Object::Conflict(Conflict {
            base_revision: Some(oid(1)),
            left_revision: oid(2),
            right_revision: oid(3),
            file_path: "file.bin".to_string(),
            codec_id: "bytes/test".to_string(),
            left_patch_ids: vec![oid(4)],
            right_patch_ids: vec![oid(5)],
            resolution_patch_ids: vec![],
            status: ConflictStatus::Open,
            created_at_ms: 1,
        }),
        8 => Object::Capsule(Capsule {
            revision_id: oid(1),
            public_fields: CapsulePublic {
                agent_id: "fuzzer".to_string(),
                agent_version: None,
                toolchain_digest: None,
                env_fingerprint: None,
                evidence: vec![Evidence {
                    name: "fuzz".to_string(),
                    status: "pass".to_string(),
                    duration_ms: data.len() as u64,
                    artifact_refs: vec![],
                    summary: None,
                    revision_id: Some(oid(1)),
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
            policy_id: "fuzz".to_string(),
            required_checks: vec!["fuzz".to_string()],
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
            workstream_id: "fuzz".to_string(),
            change_stack: vec![change_id(1)],
        }),
        _ => Object::RefLog(RefLog {
            ref_name: "heads/fuzz".to_string(),
            entries: vec![RefLogEntry {
                old_target: Some(oid(1)),
                new_target: oid(2),
                author: "fuzzer".to_string(),
                message: text,
                timestamp: data.len() as u64,
            }],
        }),
    }
}

fn oid(seed: u8) -> ObjectId {
    ObjectId::from_bytes([seed; 32])
}

fn intent_id(seed: u8) -> IntentId {
    IntentId::from_bytes([seed; 16])
}

fn change_id(seed: u8) -> ChangeId {
    ChangeId::from_bytes([seed; 16])
}

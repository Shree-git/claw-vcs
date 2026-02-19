use clap::Args;

use std::collections::{HashSet, VecDeque};

use claw_core::id::ChangeId;
use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Capsule, Policy, Revision};
use claw_merge::emit::merge;
use claw_patch::CodecRegistry;
use claw_policy::evaluator::evaluate_policy;
use claw_store::{ClawStore, HeadState};

use crate::config::find_repo_root;
use crate::conflict_writer;
use crate::merge_state::{self, ConflictEntry, MergeInfo, MergeState};
use crate::worktree;

#[derive(Args)]
pub struct IntegrateArgs {
    /// Left ref (default: HEAD's branch)
    #[arg(long)]
    left: Option<String>,
    /// Right ref to integrate
    #[arg(long)]
    right: String,
    /// Author name
    #[arg(short, long, default_value = "anonymous")]
    author: String,
    /// Merge message
    #[arg(short, long, default_value = "Integrate changes")]
    message: String,
}

pub fn run(args: IntegrateArgs) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;
    let registry = CodecRegistry::default();

    // Resolve left ref: default to HEAD's branch
    let left_ref = match args.left {
        Some(r) => r,
        None => {
            let head = store.read_head()?;
            match head {
                HeadState::Symbolic { ref_name } => ref_name,
                HeadState::Detached { .. } => {
                    anyhow::bail!("cannot integrate in detached HEAD state; use --left to specify")
                }
            }
        }
    };

    let left_id = store
        .get_ref(&left_ref)?
        .ok_or_else(|| anyhow::anyhow!("ref not found: {}", left_ref))?;
    let right_id = store
        .get_ref(&args.right)?
        .ok_or_else(|| anyhow::anyhow!("ref not found: {}", args.right))?;

    enforce_integration_policies(&store, &left_id, &right_id)?;

    let result = merge(
        &store,
        &registry,
        &left_id,
        &right_id,
        &args.author,
        &args.message,
    )?;

    if result.conflicts.is_empty() {
        // Clean merge: store revision, materialize tree, advance ref
        let rev_id = store.store_object(&Object::Revision(result.revision))?;
        store.update_ref_cas(
            &left_ref,
            Some(&left_id),
            &rev_id,
            &args.author,
            &args.message,
        )?;

        // Materialize merged tree
        if let Some(tree_id) = store.load_object(&rev_id)?.as_revision_tree() {
            worktree::materialize_tree(&store, &tree_id, &root)?;
        }

        println!("Integrated successfully: {rev_id}");
    } else {
        // Conflicted merge: write conflict artifacts, MERGE_STATE.toml, do NOT advance ref
        let mut conflict_entries = Vec::new();

        for conflict in &result.conflicts {
            let base_content =
                load_file_from_revision(&store, &result.ancestor, &conflict.file_path);
            let left_content = load_file_from_revision(&store, &left_id, &conflict.file_path);
            let right_content = load_file_from_revision(&store, &right_id, &conflict.file_path);

            let conflict_id = claw_core::id::ConflictId::new().to_string();

            match conflict.codec_id.as_str() {
                "json/tree" => {
                    conflict_writer::write_json_conflict(
                        &root,
                        &conflict.file_path,
                        &base_content,
                        &left_content,
                        &right_content,
                    )?;
                }
                "binary" => {
                    conflict_writer::write_binary_conflict(
                        &root,
                        &conflict.file_path,
                        &base_content,
                        &left_content,
                        &right_content,
                    )?;
                }
                _ => {
                    conflict_writer::write_text_conflict(
                        &root,
                        &conflict.file_path,
                        &base_content,
                        &left_content,
                        &right_content,
                        &left_ref,
                        &args.right,
                    )?;
                }
            }

            conflict_entries.push(ConflictEntry {
                file_path: conflict.file_path.clone(),
                conflict_id,
                codec_id: conflict.codec_id.clone(),
            });
        }

        // Write MERGE_STATE.toml
        let merge_state = MergeState {
            merge: MergeInfo {
                left_ref: left_ref.clone(),
                right_ref: args.right.clone(),
                left_revision: left_id.to_hex(),
                right_revision: right_id.to_hex(),
                base_revision: result.ancestor.to_hex(),
            },
            conflicts: conflict_entries,
        };
        merge_state::write_to(&store.layout().claw_dir(), &merge_state)?;

        // Also materialize non-conflicting changes from the merged tree
        // Use the left tree as the base for the working copy
        let left_obj = store.load_object(&left_id)?;
        if let Object::Revision(ref rev) = left_obj {
            if let Some(ref tree_id) = rev.tree {
                worktree::materialize_tree(&store, tree_id, &root)?;
            }
        }

        println!(
            "Merge has {} conflict(s). Resolve them and run `claw snapshot` to complete.",
            result.conflicts.len()
        );
        for c in &result.conflicts {
            println!("  CONFLICT: {} ({})", c.file_path, c.codec_id);
        }
    }

    Ok(())
}

fn enforce_integration_policies(
    store: &ClawStore,
    left_id: &ObjectId,
    right_id: &ObjectId,
) -> anyhow::Result<()> {
    let applicable = collect_applicable_revisions(store, left_id, right_id)?;

    for rev_id in applicable {
        let rev = load_revision(store, &rev_id)?;
        let policies = load_policies_for_revision(store, &rev)?;
        if policies.is_empty() {
            continue;
        }

        let capsule = load_capsule_for_revision(store, &rev_id, &rev)?.ok_or_else(|| {
            anyhow::anyhow!(
                "policy-gated revision {} has no capsule evidence",
                rev_id.to_hex()
            )
        })?;

        for policy in policies {
            evaluate_policy(&policy, &rev, &capsule).map_err(|err| {
                anyhow::anyhow!(
                    "policy '{}' failed for revision {}: {}",
                    policy.policy_id,
                    rev_id.to_hex(),
                    err
                )
            })?;
        }
    }

    Ok(())
}

fn collect_applicable_revisions(
    store: &ClawStore,
    left_id: &ObjectId,
    right_id: &ObjectId,
) -> anyhow::Result<Vec<ObjectId>> {
    let left_reachable = collect_reachable_revisions(store, left_id)?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([*right_id]);

    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        if left_reachable.contains(&id) {
            continue;
        }

        let rev = load_revision(store, &id)?;
        out.push(id);
        for parent in rev.parents {
            queue.push_back(parent);
        }
    }

    Ok(out)
}

fn collect_reachable_revisions(
    store: &ClawStore,
    start: &ObjectId,
) -> anyhow::Result<HashSet<ObjectId>> {
    let mut reachable = HashSet::new();
    let mut queue = VecDeque::from([*start]);

    while let Some(id) = queue.pop_front() {
        if !reachable.insert(id) {
            continue;
        }
        let rev = load_revision(store, &id)?;
        for parent in rev.parents {
            queue.push_back(parent);
        }
    }

    Ok(reachable)
}

fn load_revision(store: &ClawStore, id: &ObjectId) -> anyhow::Result<Revision> {
    let obj = store.load_object(id)?;
    match obj {
        Object::Revision(rev) => Ok(rev),
        _ => anyhow::bail!("object is not a revision: {}", id.to_hex()),
    }
}

fn load_policies_for_revision(store: &ClawStore, rev: &Revision) -> anyhow::Result<Vec<Policy>> {
    let change_id = match rev.change_id {
        Some(id) => id,
        None => return Ok(vec![]),
    };

    let intent = load_intent_for_change(store, &change_id)?;
    let mut policies = Vec::new();
    let mut seen = HashSet::new();

    for policy_ref in &intent.policy_refs {
        let policy = load_policy_ref(store, policy_ref)?;
        if seen.insert(policy.policy_id.clone()) {
            policies.push(policy);
        }
    }

    Ok(policies)
}

fn load_intent_for_change(
    store: &ClawStore,
    change_id: &ChangeId,
) -> anyhow::Result<claw_core::types::Intent> {
    let change_obj_id = store
        .get_ref(&format!("changes/{change_id}"))?
        .ok_or_else(|| anyhow::anyhow!("change not found: {}", change_id))?;
    let change_obj = store.load_object(&change_obj_id)?;
    let change = match change_obj {
        Object::Change(c) => c,
        _ => anyhow::bail!(
            "change ref does not point to a change object: {}",
            change_id
        ),
    };

    let intent_obj_id = store
        .get_ref(&format!("intents/{}", change.intent_id))?
        .ok_or_else(|| anyhow::anyhow!("intent not found for change: {}", change_id))?;
    let intent_obj = store.load_object(&intent_obj_id)?;
    match intent_obj {
        Object::Intent(intent) => Ok(intent),
        _ => anyhow::bail!(
            "intent ref does not point to an intent object: {}",
            change.intent_id
        ),
    }
}

fn load_policy_ref(store: &ClawStore, policy_ref: &str) -> anyhow::Result<Policy> {
    let ref_name = if store.get_ref(policy_ref)?.is_some() {
        policy_ref.to_string()
    } else {
        format!("policies/{policy_ref}")
    };

    let policy_obj_id = store
        .get_ref(&ref_name)?
        .ok_or_else(|| anyhow::anyhow!("policy ref not found: {}", ref_name))?;
    let policy_obj = store.load_object(&policy_obj_id)?;
    match policy_obj {
        Object::Policy(policy) => Ok(policy),
        _ => anyhow::bail!("ref does not point to a policy object: {}", ref_name),
    }
}

fn load_capsule_for_revision(
    store: &ClawStore,
    rev_id: &ObjectId,
    rev: &Revision,
) -> anyhow::Result<Option<Capsule>> {
    let capsule_id = if let Some(id) = rev.capsule_id {
        Some(id)
    } else {
        let full = rev_id.to_hex();
        store
            .get_ref(&format!("capsules/by-revision/{full}"))?
            .or_else(|| {
                let prefix = &full[..16];
                store
                    .get_ref(&format!("capsules/by-revision/{prefix}"))
                    .ok()
                    .flatten()
            })
    };

    let Some(capsule_id) = capsule_id else {
        return Ok(None);
    };
    let capsule_obj = store.load_object(&capsule_id)?;
    match capsule_obj {
        Object::Capsule(capsule) => Ok(Some(capsule)),
        _ => anyhow::bail!(
            "capsule mapping points to non-capsule object for revision {}",
            rev_id.to_hex()
        ),
    }
}

fn load_file_from_revision(store: &ClawStore, rev_id: &ObjectId, path: &str) -> Vec<u8> {
    let obj = match store.load_object(rev_id) {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let tree_id = match obj {
        Object::Revision(ref rev) => match rev.tree {
            Some(t) => t,
            None => return vec![],
        },
        _ => return vec![],
    };
    find_blob_in_tree(store, &tree_id, path).unwrap_or_default()
}

fn find_blob_in_tree(store: &ClawStore, tree_id: &ObjectId, path: &str) -> Option<Vec<u8>> {
    let parts: Vec<&str> = path.split('/').collect();
    find_blob_recursive(store, tree_id, &parts)
}

fn find_blob_recursive(store: &ClawStore, tree_id: &ObjectId, parts: &[&str]) -> Option<Vec<u8>> {
    if parts.is_empty() {
        return None;
    }
    let obj = store.load_object(tree_id).ok()?;
    let tree = match obj {
        Object::Tree(t) => t,
        _ => return None,
    };
    for entry in &tree.entries {
        if entry.name == parts[0] {
            if parts.len() == 1 {
                let blob_obj = store.load_object(&entry.object_id).ok()?;
                if let Object::Blob(b) = blob_obj {
                    return Some(b.data);
                }
                return None;
            } else {
                return find_blob_recursive(store, &entry.object_id, &parts[1..]);
            }
        }
    }
    None
}

// Helper trait for Object
trait ObjectExt {
    fn as_revision_tree(&self) -> Option<ObjectId>;
}

impl ObjectExt for Object {
    fn as_revision_tree(&self) -> Option<ObjectId> {
        match self {
            Object::Revision(rev) => rev.tree,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_applicable_revisions, enforce_integration_policies};
    use claw_core::id::{ChangeId, IntentId};
    use claw_core::object::Object;
    use claw_core::types::{
        Capsule, CapsulePublic, Change, ChangeStatus, Intent, IntentStatus, Policy, Revision,
        Visibility,
    };
    use claw_store::ClawStore;

    fn test_store() -> (tempfile::TempDir, ClawStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        (tmp, store)
    }

    #[test]
    fn collect_applicable_revisions_excludes_left_side() {
        let (_tmp, store) = test_store();

        let base = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 1,
                summary: "base".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let right_only = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![base],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 2,
                summary: "right".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let applicable = collect_applicable_revisions(&store, &base, &right_only).unwrap();
        assert_eq!(applicable, vec![right_only]);
    }

    #[test]
    fn policy_check_blocks_integration_when_evidence_missing() {
        let (_tmp, store) = test_store();

        let intent_id = IntentId::new();
        let change_id = ChangeId::new();

        let intent_obj = Object::Intent(Intent {
            id: intent_id,
            title: "intent".to_string(),
            goal: "goal".to_string(),
            constraints: vec![],
            acceptance_tests: vec![],
            links: vec![],
            policy_refs: vec!["ci-required".to_string()],
            agents: vec![],
            change_ids: vec![],
            depends_on: vec![],
            supersedes: vec![],
            status: IntentStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 1,
        });
        let intent_obj_id = store.store_object(&intent_obj).unwrap();
        store
            .set_ref(&format!("intents/{intent_id}"), &intent_obj_id)
            .unwrap();

        let change_obj = Object::Change(Change {
            id: change_id,
            intent_id,
            head_revision: None,
            workstream_id: None,
            status: ChangeStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 1,
        });
        let change_obj_id = store.store_object(&change_obj).unwrap();
        store
            .set_ref(&format!("changes/{change_id}"), &change_obj_id)
            .unwrap();

        let policy_obj = Object::Policy(Policy {
            policy_id: "ci-required".to_string(),
            required_checks: vec!["ci/test".to_string()],
            required_reviewers: vec![],
            sensitive_paths: vec![],
            quarantine_lane: false,
            min_trust_score: None,
            visibility: Visibility::Public,
        });
        let policy_obj_id = store.store_object(&policy_obj).unwrap();
        store
            .set_ref("policies/ci-required", &policy_obj_id)
            .unwrap();

        let left = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 2,
                summary: "left".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let right = store
            .store_object(&Object::Revision(Revision {
                change_id: Some(change_id),
                parents: vec![left],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 3,
                summary: "right".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let capsule_obj = Object::Capsule(Capsule {
            revision_id: right,
            public_fields: CapsulePublic {
                agent_id: "agent".to_string(),
                agent_version: None,
                toolchain_digest: None,
                env_fingerprint: None,
                evidence: vec![],
            },
            encrypted_private: None,
            encryption: String::new(),
            key_id: None,
            signatures: vec![],
        });
        let capsule_id = store.store_object(&capsule_obj).unwrap();
        store
            .set_ref(
                &format!("capsules/by-revision/{}", &right.to_hex()[..16]),
                &capsule_id,
            )
            .unwrap();

        let err = enforce_integration_policies(&store, &left, &right).unwrap_err();
        assert!(
            err.to_string().contains("missing required check"),
            "unexpected error: {err}"
        );
    }
}

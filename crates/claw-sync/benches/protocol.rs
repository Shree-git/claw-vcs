use std::collections::HashSet;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_store::ClawStore;
use claw_sync::negotiation::{compute_want_have, ordered_reachable_objects};
use claw_sync::protocol::{
    negotiate_capabilities, CAP_EVENT_BUS, CAP_PARTIAL_CLONE, CAP_REQUEST_LIMITS,
};

struct HistoryFixture {
    _temp: TempDir,
    store: ClawStore,
    head: ObjectId,
    local_objects: HashSet<ObjectId>,
    remote_objects: HashSet<ObjectId>,
}

fn seed_history(revision_count: usize) -> HistoryFixture {
    let temp = tempfile::tempdir().expect("create temp repo");
    let store = ClawStore::init(temp.path()).expect("init store");
    let mut parent = None;
    let mut all_revision_ids = Vec::with_capacity(revision_count);

    for idx in 0..revision_count {
        let blob_id = store
            .store_object(&Object::Blob(Blob {
                data: format!("revision {idx}\n").into_bytes(),
                media_type: Some("text/plain".to_string()),
            }))
            .expect("store blob");
        let tree_id = store
            .store_object(&Object::Tree(Tree {
                entries: vec![TreeEntry {
                    name: "state.txt".to_string(),
                    mode: FileMode::Regular,
                    object_id: blob_id,
                }],
            }))
            .expect("store tree");
        let revision_id = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: parent.into_iter().collect(),
                patches: vec![],
                snapshot_base: None,
                tree: Some(tree_id),
                capsule_id: None,
                author: "criterion".to_string(),
                created_at_ms: 1_700_000_000_000 + idx as u64,
                summary: format!("revision {idx}"),
                policy_evidence: vec![],
            }))
            .expect("store revision");
        all_revision_ids.push(revision_id);
        parent = Some(revision_id);
    }

    let head = parent.expect("at least one revision");
    let local_objects = ordered_reachable_objects(&store, &[head])
        .into_iter()
        .collect::<HashSet<_>>();
    let remote_objects = all_revision_ids
        .iter()
        .step_by(2)
        .copied()
        .collect::<HashSet<_>>();

    HistoryFixture {
        _temp: temp,
        store,
        head,
        local_objects,
        remote_objects,
    }
}

fn bench_sync_protocol(c: &mut Criterion) {
    let client_caps = vec![
        CAP_REQUEST_LIMITS.to_string(),
        "unknown-capability".to_string(),
        CAP_PARTIAL_CLONE.to_string(),
        CAP_EVENT_BUS.to_string(),
    ];
    c.bench_function("sync_negotiate_capabilities", |b| {
        b.iter(|| negotiate_capabilities(black_box(&client_caps)))
    });

    let fixture = seed_history(200);
    c.bench_function("sync_ordered_reachable_objects_200_revisions", |b| {
        b.iter(|| ordered_reachable_objects(black_box(&fixture.store), black_box(&[fixture.head])))
    });

    c.bench_function("sync_compute_want_have_200_revisions", |b| {
        b.iter(|| {
            black_box(compute_want_have(
                black_box(&fixture.local_objects),
                black_box(&fixture.remote_objects),
            ))
        })
    });
}

criterion_group!(benches, bench_sync_protocol);
criterion_main!(benches);

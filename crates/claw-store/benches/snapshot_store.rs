use std::fs;
use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use tempfile::TempDir;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Snapshot, Tree, TreeEntry};
use claw_store::ClawStore;

const FILE_COUNT: usize = 128;
const PAYLOAD_SIZE: usize = 4 * 1024;

struct SeededRepo {
    _temp: TempDir,
    store: ClawStore,
    object_ids: Vec<ObjectId>,
    snapshot_id: ObjectId,
}

fn seed_repo(file_count: usize, payload_size: usize) -> SeededRepo {
    let temp = tempfile::tempdir().expect("create temp repo");
    let store = ClawStore::init(temp.path()).expect("init store");
    let mut object_ids = Vec::with_capacity(file_count + 3);
    let mut entries = Vec::with_capacity(file_count);

    for idx in 0..file_count {
        let payload = vec![(idx % 251) as u8; payload_size];
        let blob_id = store
            .store_object(&Object::Blob(Blob {
                data: payload,
                media_type: Some("application/octet-stream".to_string()),
            }))
            .expect("store blob");
        object_ids.push(blob_id);
        entries.push(TreeEntry {
            name: format!("file-{idx:04}.bin"),
            mode: FileMode::Regular,
            object_id: blob_id,
        });
    }

    let tree_id = store
        .store_object(&Object::Tree(Tree { entries }))
        .expect("store tree");
    object_ids.push(tree_id);

    let revision_id = store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: "criterion".to_string(),
            created_at_ms: 1_700_000_000_000,
            summary: "benchmark snapshot".to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision");
    object_ids.push(revision_id);

    let snapshot_id = store
        .store_object(&Object::Snapshot(Snapshot {
            tree_root: tree_id,
            revision_id,
            created_at_ms: 1_700_000_000_001,
        }))
        .expect("store snapshot");
    object_ids.push(snapshot_id);

    SeededRepo {
        _temp: temp,
        store,
        object_ids,
        snapshot_id,
    }
}

fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}

fn bench_snapshot_store(c: &mut Criterion) {
    c.bench_function("snapshot_store_128_files_4k", |b| {
        b.iter_batched(
            || (),
            |_| {
                let repo = seed_repo(FILE_COUNT, PAYLOAD_SIZE);
                black_box(repo.snapshot_id)
            },
            BatchSize::SmallInput,
        )
    });

    let repo = seed_repo(FILE_COUNT, PAYLOAD_SIZE);
    c.bench_function("store_load_all_snapshot_objects_128_files_4k", |b| {
        b.iter(|| {
            for id in &repo.object_ids {
                black_box(repo.store.load_object(black_box(id)).expect("load object"));
            }
        })
    });

    let claw_dir = repo.store.layout().claw_dir();
    c.bench_function("repo_size_scan_claw_store_128_files_4k", |b| {
        b.iter(|| black_box(dir_size(black_box(&claw_dir)).expect("scan repo size")))
    });
}

criterion_group!(benches, bench_snapshot_store);
criterion_main!(benches);

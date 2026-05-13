use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Patch, Revision, Tree, TreeEntry};
use claw_merge::emit::merge;
use claw_patch::text_line::TextLineCodec;
use claw_patch::{Codec, CodecRegistry};
use claw_store::ClawStore;

struct MergeFixture {
    _temp: TempDir,
    store: ClawStore,
    registry: CodecRegistry,
    left_head: ObjectId,
    right_head: ObjectId,
}

fn lines_with_change(line_count: usize, changed_line: Option<usize>, label: &str) -> Vec<u8> {
    let mut out = String::new();
    for idx in 0..line_count {
        if Some(idx) == changed_line {
            out.push_str(&format!("line {idx} changed by {label}\n"));
        } else {
            out.push_str(&format!("line {idx}\n"));
        }
    }
    out.into_bytes()
}

fn store_file_revision(
    store: &ClawStore,
    parent: Option<ObjectId>,
    content: Vec<u8>,
    patches: Vec<ObjectId>,
    summary: &str,
) -> ObjectId {
    let blob_id = store
        .store_object(&Object::Blob(Blob {
            data: content,
            media_type: Some("text/plain".to_string()),
        }))
        .expect("store blob");
    let tree_id = store
        .store_object(&Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: "main.rs".to_string(),
                mode: FileMode::Regular,
                object_id: blob_id,
            }],
        }))
        .expect("store tree");

    store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: parent.into_iter().collect(),
            patches,
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: "criterion".to_string(),
            created_at_ms: 1_700_000_000_000,
            summary: summary.to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision")
}

fn seed_merge_fixture() -> MergeFixture {
    let temp = tempfile::tempdir().expect("create temp repo");
    let store = ClawStore::init(temp.path()).expect("init store");
    let codec = TextLineCodec;

    let base = lines_with_change(2_000, None, "");
    let left = lines_with_change(2_000, Some(200), "left");
    let right = lines_with_change(2_000, Some(1_800), "right");

    let base_head = store_file_revision(&store, None, base.clone(), vec![], "base");

    let left_patch = store
        .store_object(&Object::Patch(Patch {
            target_path: "main.rs".to_string(),
            codec_id: codec.id().to_string(),
            base_object: None,
            result_object: None,
            ops: codec.diff(&base, &left).expect("left diff"),
            codec_payload: None,
        }))
        .expect("store left patch");
    let left_head = store_file_revision(
        &store,
        Some(base_head),
        left,
        vec![left_patch],
        "left branch",
    );

    let right_patch = store
        .store_object(&Object::Patch(Patch {
            target_path: "main.rs".to_string(),
            codec_id: codec.id().to_string(),
            base_object: None,
            result_object: None,
            ops: codec.diff(&base, &right).expect("right diff"),
            codec_payload: None,
        }))
        .expect("store right patch");
    let right_head = store_file_revision(
        &store,
        Some(base_head),
        right,
        vec![right_patch],
        "right branch",
    );

    MergeFixture {
        _temp: temp,
        store,
        registry: CodecRegistry::default_registry(),
        left_head,
        right_head,
    }
}

fn bench_merge(c: &mut Criterion) {
    let fixture = seed_merge_fixture();
    c.bench_function("merge_non_overlapping_text_patches_2000_lines", |b| {
        b.iter(|| {
            black_box(
                merge(
                    black_box(&fixture.store),
                    black_box(&fixture.registry),
                    black_box(&fixture.left_head),
                    black_box(&fixture.right_head),
                    "criterion",
                    "merge benchmark",
                )
                .expect("merge revisions"),
            )
        })
    });
}

criterion_group!(benches, bench_merge);
criterion_main!(benches);

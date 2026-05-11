use std::fs;
use std::path::{Path, PathBuf};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use tempfile::TempDir;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_git::exporter::GitExporter;
use claw_git::importer::GitImporter;
use claw_store::ClawStore;

const FILE_COUNT: usize = 64;
const PAYLOAD_SIZE: usize = 2 * 1024;

struct ClawFixture {
    _temp: TempDir,
    store: ClawStore,
    head: ObjectId,
}

struct GitFixture {
    _temp: TempDir,
    git_dir: PathBuf,
}

fn seed_claw_repo(file_count: usize, payload_size: usize) -> ClawFixture {
    let temp = tempfile::tempdir().expect("create temp repo");
    let store = ClawStore::init(temp.path()).expect("init store");
    let mut entries = Vec::with_capacity(file_count);

    for idx in 0..file_count {
        let blob_id = store
            .store_object(&Object::Blob(Blob {
                data: vec![(idx % 251) as u8; payload_size],
                media_type: Some("application/octet-stream".to_string()),
            }))
            .expect("store blob");
        entries.push(TreeEntry {
            name: format!("file-{idx:04}.bin"),
            mode: FileMode::Regular,
            object_id: blob_id,
        });
    }

    let tree_id = store
        .store_object(&Object::Tree(Tree { entries }))
        .expect("store tree");
    let head = store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: "criterion".to_string(),
            created_at_ms: 1_700_000_000_000,
            summary: "git bridge benchmark".to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision");

    ClawFixture {
        _temp: temp,
        store,
        head,
    }
}

fn seed_git_repo(file_count: usize, payload_size: usize) -> GitFixture {
    let claw = seed_claw_repo(file_count, payload_size);
    let temp = tempfile::tempdir().expect("create temp git repo");
    let git_dir = temp.path().join(".git");
    let objects_dir = git_dir.join("objects");
    let mut exporter = GitExporter::new(&claw.store);
    let head_sha1 = exporter
        .export(&claw.head, &objects_dir)
        .expect("export git objects");
    write_git_ref(&git_dir, "refs/heads/main", &head_sha1);

    GitFixture {
        _temp: temp,
        git_dir,
    }
}

fn write_git_ref(git_dir: &Path, name: &str, sha1: &[u8; 20]) {
    let path = git_dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create git ref dir");
    }
    fs::write(path, format!("{}\n", hex::encode(sha1))).expect("write git ref");
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

fn bench_git_bridge(c: &mut Criterion) {
    let claw = seed_claw_repo(FILE_COUNT, PAYLOAD_SIZE);
    c.bench_function("git_export_64_files_2k", |b| {
        b.iter_batched(
            || tempfile::tempdir().expect("create git export dir"),
            |temp| {
                let mut exporter = GitExporter::new(&claw.store);
                let objects_dir = temp.path().join("objects");
                black_box(
                    exporter
                        .export(black_box(&claw.head), black_box(&objects_dir))
                        .expect("export git objects"),
                );
            },
            BatchSize::SmallInput,
        )
    });

    let git = seed_git_repo(FILE_COUNT, PAYLOAD_SIZE);
    c.bench_function("git_import_64_files_2k", |b| {
        b.iter_batched(
            || tempfile::tempdir().expect("create import repo"),
            |temp| {
                let store = ClawStore::init(temp.path()).expect("init import store");
                let mut importer = GitImporter::new(&store);
                black_box(
                    importer
                        .import_ref(black_box(&git.git_dir), "refs/heads/main", "heads/imported")
                        .expect("import git ref"),
                );
            },
            BatchSize::SmallInput,
        )
    });

    let claw_dir = claw.store.layout().claw_dir();
    let git_objects_dir = git.git_dir.join("objects");
    c.bench_function("repo_size_scan_claw_vs_git_64_files_2k", |b| {
        b.iter(|| {
            black_box((
                dir_size(black_box(&claw_dir)).expect("scan claw dir"),
                dir_size(black_box(&git_objects_dir)).expect("scan git objects dir"),
            ))
        })
    });
}

criterion_group!(benches, bench_git_bridge);
criterion_main!(benches);

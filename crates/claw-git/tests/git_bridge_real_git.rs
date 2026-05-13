use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_git::exporter::GitExporter;
use claw_git::importer::{list_git_refs, GitImporter};
use claw_store::ClawStore;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

type TestResult = Result<(), Box<dyn Error>>;

fn git_is_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git(repo: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "git -C {} {} failed\nstdout:\n{}\nstderr:\n{}",
            repo.display(),
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn init_git_repo(repo: &Path) -> TestResult {
    fs::create_dir_all(repo)?;
    git(repo, &["init"])?;
    git(repo, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    git(repo, &["config", "user.name", "Git User"])?;
    git(repo, &["config", "user.email", "git@example.com"])?;
    git(repo, &["config", "core.autocrlf", "false"])?;
    git(repo, &["config", "core.fileMode", "true"])?;
    Ok(())
}

fn write_file(repo: &Path, relative_path: &str, data: &[u8]) -> TestResult {
    let path = repo.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, data)?;
    Ok(())
}

fn unicode_leaf() -> String {
    "\u{30e6}\u{30cb}\u{30b3}\u{30fc}\u{30c9}.txt".to_string()
}

fn binary_bytes() -> Vec<u8> {
    vec![0, 1, 2, b'g', b'i', b't', 0xff, 0xfe, b'\n', 0]
}

fn large_bytes() -> Vec<u8> {
    (0..(1024 * 1024)).map(|idx| (idx % 251) as u8).collect()
}

fn store_blob(store: &ClawStore, data: Vec<u8>) -> Result<ObjectId, Box<dyn Error>> {
    Ok(store.store_object(&Object::Blob(Blob {
        data,
        media_type: None,
    }))?)
}

fn store_tree(
    store: &ClawStore,
    entries: Vec<(&str, FileMode, ObjectId)>,
) -> Result<ObjectId, Box<dyn Error>> {
    Ok(store.store_object(&Object::Tree(Tree {
        entries: entries
            .into_iter()
            .map(|(name, mode, object_id)| TreeEntry {
                name: name.to_string(),
                mode,
                object_id,
            })
            .collect(),
    }))?)
}

fn store_revision(
    store: &ClawStore,
    tree: ObjectId,
    parents: Vec<ObjectId>,
    summary: &str,
    created_at_ms: u64,
) -> Result<ObjectId, Box<dyn Error>> {
    Ok(store.store_object(&Object::Revision(Revision {
        change_id: None,
        parents,
        patches: Vec::new(),
        snapshot_base: None,
        tree: Some(tree),
        capsule_id: None,
        author: "Exporter".to_string(),
        created_at_ms,
        summary: summary.to_string(),
        policy_evidence: Vec::new(),
    }))?)
}

fn load_revision(store: &ClawStore, id: ObjectId) -> Revision {
    match store.load_object(&id).unwrap() {
        Object::Revision(revision) => revision,
        other => panic!("expected revision, got {other:?}"),
    }
}

fn load_tree(store: &ClawStore, id: ObjectId) -> Tree {
    match store.load_object(&id).unwrap() {
        Object::Tree(tree) => tree,
        other => panic!("expected tree, got {other:?}"),
    }
}

fn entry_id(tree: &Tree, name: &str, mode: FileMode) -> ObjectId {
    let entry = tree
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("missing tree entry {name}"));
    assert_eq!(entry.mode, mode, "unexpected mode for {name}");
    entry.object_id
}

fn assert_claw_blob(
    store: &ClawStore,
    root_tree: ObjectId,
    components: &[&str],
    expected_mode: FileMode,
    expected_data: &[u8],
) {
    let mut tree = load_tree(store, root_tree);
    for component in &components[..components.len() - 1] {
        let child = entry_id(&tree, component, FileMode::Directory);
        tree = load_tree(store, child);
    }

    let leaf_name = components.last().unwrap();
    let blob_id = entry_id(&tree, leaf_name, expected_mode);
    let blob = match store.load_object(&blob_id).unwrap() {
        Object::Blob(blob) => blob,
        other => panic!("expected blob, got {other:?}"),
    };
    assert_eq!(blob.data, expected_data);
}

fn find_revision_by_summary(store: &ClawStore, start: ObjectId, summary: &str) -> Option<ObjectId> {
    let mut stack = vec![start];
    let mut seen = HashSet::new();

    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let revision = load_revision(store, id);
        if revision.summary == summary {
            return Some(id);
        }
        stack.extend(revision.parents);
    }

    None
}

fn assert_git_checkout(repo: &Path, unicode_file: &str, large: &[u8]) -> TestResult {
    assert_eq!(fs::read(repo.join("README.md"))?, b"hello from claw\n");
    assert_eq!(
        fs::read(repo.join("bin/run.sh"))?,
        b"#!/bin/sh\necho claw\n"
    );
    assert_eq!(fs::read(repo.join("nested/dir/file.txt"))?, b"nested\n");
    assert_eq!(fs::read(repo.join("data/blob.bin"))?, binary_bytes());
    assert_eq!(fs::read(repo.join("large.bin"))?, large);
    assert_eq!(
        fs::read(repo.join("unicode").join(unicode_file))?,
        b"unicode path\n"
    );

    #[cfg(unix)]
    {
        let mode = fs::metadata(repo.join("bin/run.sh"))?.permissions().mode();
        assert_ne!(mode & 0o111, 0, "executable bit was not preserved");
    }

    Ok(())
}

#[test]
fn exports_claw_history_to_real_git_objects_checkout_and_cat_file() -> TestResult {
    if !git_is_available() {
        eprintln!("skipping real-git integration test because git is not available");
        return Ok(());
    }

    let tmp = tempdir()?;
    let claw_root = tmp.path().join("claw");
    let store = ClawStore::init(&claw_root)?;

    let unicode_file = unicode_leaf();
    let binary = binary_bytes();
    let large = large_bytes();

    let empty_tree = store_tree(&store, Vec::new())?;
    let empty_revision = store_revision(
        &store,
        empty_tree,
        Vec::new(),
        "Empty tree",
        1_700_000_000_000,
    )?;

    let readme = store_blob(&store, b"hello from claw\n".to_vec())?;
    let executable = store_blob(&store, b"#!/bin/sh\necho claw\n".to_vec())?;
    let nested_file = store_blob(&store, b"nested\n".to_vec())?;
    let unicode_blob = store_blob(&store, b"unicode path\n".to_vec())?;
    let binary_blob = store_blob(&store, binary)?;
    let large_blob = store_blob(&store, large.clone())?;

    let bin_tree = store_tree(&store, vec![("run.sh", FileMode::Executable, executable)])?;
    let data_tree = store_tree(&store, vec![("blob.bin", FileMode::Regular, binary_blob)])?;
    let nested_dir_tree = store_tree(&store, vec![("file.txt", FileMode::Regular, nested_file)])?;
    let nested_tree = store_tree(&store, vec![("dir", FileMode::Directory, nested_dir_tree)])?;
    let unicode_tree = store_tree(
        &store,
        vec![(unicode_file.as_str(), FileMode::Regular, unicode_blob)],
    )?;

    let base_tree = store_tree(
        &store,
        vec![
            ("README.md", FileMode::Regular, readme),
            ("bin", FileMode::Directory, bin_tree),
            ("data", FileMode::Directory, data_tree),
            ("large.bin", FileMode::Regular, large_blob),
            ("nested", FileMode::Directory, nested_tree),
            ("unicode", FileMode::Directory, unicode_tree),
        ],
    )?;
    let base_revision = store_revision(
        &store,
        base_tree,
        vec![empty_revision],
        "Add files",
        1_700_000_001_000,
    )?;

    let main_only = store_blob(&store, b"main side\n".to_vec())?;
    let main_tree = store_tree(
        &store,
        vec![
            ("README.md", FileMode::Regular, readme),
            ("bin", FileMode::Directory, bin_tree),
            ("data", FileMode::Directory, data_tree),
            ("large.bin", FileMode::Regular, large_blob),
            ("main.txt", FileMode::Regular, main_only),
            ("nested", FileMode::Directory, nested_tree),
            ("unicode", FileMode::Directory, unicode_tree),
        ],
    )?;
    let main_revision = store_revision(
        &store,
        main_tree,
        vec![base_revision],
        "Main side",
        1_700_000_002_000,
    )?;

    let feature_only = store_blob(&store, b"feature side\n".to_vec())?;
    let feature_tree = store_tree(
        &store,
        vec![
            ("README.md", FileMode::Regular, readme),
            ("bin", FileMode::Directory, bin_tree),
            ("data", FileMode::Directory, data_tree),
            ("feature.txt", FileMode::Regular, feature_only),
            ("large.bin", FileMode::Regular, large_blob),
            ("nested", FileMode::Directory, nested_tree),
            ("unicode", FileMode::Directory, unicode_tree),
        ],
    )?;
    let feature_revision = store_revision(
        &store,
        feature_tree,
        vec![base_revision],
        "Feature side",
        1_700_000_003_000,
    )?;

    let merge_tree = store_tree(
        &store,
        vec![
            ("README.md", FileMode::Regular, readme),
            ("bin", FileMode::Directory, bin_tree),
            ("data", FileMode::Directory, data_tree),
            ("feature.txt", FileMode::Regular, feature_only),
            ("large.bin", FileMode::Regular, large_blob),
            ("main.txt", FileMode::Regular, main_only),
            ("nested", FileMode::Directory, nested_tree),
            ("unicode", FileMode::Directory, unicode_tree),
        ],
    )?;
    let merge_revision = store_revision(
        &store,
        merge_tree,
        vec![main_revision, feature_revision],
        "Merge side branches",
        1_700_000_004_000,
    )?;

    let git_repo = tmp.path().join("git-export");
    init_git_repo(&git_repo)?;

    let mut exporter = GitExporter::new(&store);
    let merge_sha = exporter.export(&merge_revision, &git_repo.join(".git/objects"))?;
    let empty_sha = exporter
        .get_sha1(&empty_revision)
        .expect("empty revision should be exported as an ancestor");
    let feature_sha = exporter
        .get_sha1(&feature_revision)
        .expect("feature revision should be exported as an ancestor");
    let large_sha = exporter
        .get_sha1(&large_blob)
        .expect("large blob should be exported");

    let merge_hex = hex::encode(merge_sha);
    let empty_hex = hex::encode(empty_sha);
    let feature_hex = hex::encode(feature_sha);
    let large_hex = hex::encode(large_sha);

    git(
        &git_repo,
        &["update-ref", "refs/heads/main", merge_hex.as_str()],
    )?;
    git(
        &git_repo,
        &["update-ref", "refs/heads/empty", empty_hex.as_str()],
    )?;
    git(
        &git_repo,
        &["update-ref", "refs/heads/feature", feature_hex.as_str()],
    )?;

    git(&git_repo, &["fsck", "--strict"])?;
    assert_eq!(
        git(&git_repo, &["cat-file", "-t", "main"])?.trim(),
        "commit"
    );
    assert_eq!(
        git(&git_repo, &["cat-file", "-t", "main^{tree}"])?.trim(),
        "tree"
    );
    assert_eq!(
        git(&git_repo, &["cat-file", "-t", large_hex.as_str()])?.trim(),
        "blob"
    );

    let parent_line = git(&git_repo, &["log", "--format=%P", "-1", "main"])?;
    assert_eq!(parent_line.split_whitespace().count(), 2);

    let tree_listing = git(
        &git_repo,
        &[
            "-c",
            "core.quotePath=false",
            "ls-tree",
            "-r",
            "--full-tree",
            "main",
        ],
    )?;
    assert!(tree_listing.contains("100755 blob"));
    assert!(tree_listing.contains("bin/run.sh"));
    assert!(tree_listing.contains(&format!("unicode/{unicode_file}")));

    git(&git_repo, &["checkout", "--force", "empty"])?;
    let worktree_entries = fs::read_dir(&git_repo)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != ".git")
        .count();
    assert_eq!(worktree_entries, 0);

    git(&git_repo, &["checkout", "--force", "main"])?;
    assert_git_checkout(&git_repo, &unicode_file, &large)?;
    assert_eq!(fs::read(git_repo.join("feature.txt"))?, b"feature side\n");
    assert_eq!(fs::read(git_repo.join("main.txt"))?, b"main side\n");

    Ok(())
}

#[test]
fn imports_real_git_branches_merge_modes_paths_and_roundtrips_to_checkout() -> TestResult {
    if !git_is_available() {
        eprintln!("skipping real-git integration test because git is not available");
        return Ok(());
    }

    let tmp = tempdir()?;
    let git_repo = tmp.path().join("git-import");
    init_git_repo(&git_repo)?;

    let unicode_file = unicode_leaf();
    let unicode_path = format!("unicode/{unicode_file}");
    let binary = binary_bytes();
    let large = large_bytes();

    git(&git_repo, &["commit", "--allow-empty", "-m", "Empty tree"])?;

    write_file(&git_repo, "README.md", b"hello from claw\n")?;
    write_file(&git_repo, "bin/run.sh", b"#!/bin/sh\necho claw\n")?;
    write_file(&git_repo, "nested/dir/file.txt", b"nested\n")?;
    write_file(&git_repo, unicode_path.as_str(), b"unicode path\n")?;
    write_file(&git_repo, "data/blob.bin", &binary)?;
    write_file(&git_repo, "large.bin", &large)?;
    git(
        &git_repo,
        &[
            "add",
            "README.md",
            "bin/run.sh",
            "nested",
            "unicode",
            "data",
            "large.bin",
        ],
    )?;
    git(&git_repo, &["update-index", "--chmod=+x", "bin/run.sh"])?;
    git(&git_repo, &["commit", "-m", "Add files"])?;

    git(&git_repo, &["checkout", "-b", "feature"])?;
    write_file(&git_repo, "feature.txt", b"feature side\n")?;
    git(&git_repo, &["add", "feature.txt"])?;
    git(&git_repo, &["commit", "-m", "Feature side"])?;

    git(&git_repo, &["checkout", "main"])?;
    write_file(&git_repo, "main.txt", b"main side\n")?;
    git(&git_repo, &["add", "main.txt"])?;
    git(&git_repo, &["commit", "-m", "Main side"])?;
    git(
        &git_repo,
        &["merge", "--no-ff", "feature", "-m", "Merge feature"],
    )?;
    git(&git_repo, &["pack-refs", "--all"])?;

    let claw_root = tmp.path().join("claw-import");
    let store = ClawStore::init(&claw_root)?;
    let mut importer = GitImporter::new(&store);
    let main_revision = importer.import_ref(&git_repo.join(".git"), "main", "heads/main")?;
    let feature_revision = importer.import_ref(
        &git_repo.join(".git"),
        "refs/heads/feature",
        "heads/feature",
    )?;

    assert_eq!(store.get_ref("heads/main")?, Some(main_revision));
    assert_eq!(store.get_ref("heads/feature")?, Some(feature_revision));

    let refs = list_git_refs(&git_repo.join(".git"), "refs/heads/")?;
    assert!(refs.iter().any(|(name, _)| name == "refs/heads/main"));
    assert!(refs.iter().any(|(name, _)| name == "refs/heads/feature"));

    let merge_revision = load_revision(&store, main_revision);
    assert_eq!(merge_revision.summary, "Merge feature");
    assert_eq!(merge_revision.parents.len(), 2);
    let root_tree = merge_revision
        .tree
        .expect("merge revision should have a tree");

    assert_claw_blob(
        &store,
        root_tree,
        &["README.md"],
        FileMode::Regular,
        b"hello from claw\n",
    );
    assert_claw_blob(
        &store,
        root_tree,
        &["bin", "run.sh"],
        FileMode::Executable,
        b"#!/bin/sh\necho claw\n",
    );
    assert_claw_blob(
        &store,
        root_tree,
        &["nested", "dir", "file.txt"],
        FileMode::Regular,
        b"nested\n",
    );
    assert_claw_blob(
        &store,
        root_tree,
        &["unicode", unicode_file.as_str()],
        FileMode::Regular,
        b"unicode path\n",
    );
    assert_claw_blob(
        &store,
        root_tree,
        &["data", "blob.bin"],
        FileMode::Regular,
        &binary,
    );
    assert_claw_blob(&store, root_tree, &["large.bin"], FileMode::Regular, &large);

    let empty_revision = find_revision_by_summary(&store, main_revision, "Empty tree")
        .expect("empty-tree ancestor should import");
    let empty_revision = load_revision(&store, empty_revision);
    let empty_tree = load_tree(&store, empty_revision.tree.unwrap());
    assert!(empty_tree.entries.is_empty());

    let roundtrip_repo = tmp.path().join("git-roundtrip");
    init_git_repo(&roundtrip_repo)?;
    let mut exporter = GitExporter::new(&store);
    let roundtrip_sha = exporter.export(&main_revision, &roundtrip_repo.join(".git/objects"))?;
    let roundtrip_hex = hex::encode(roundtrip_sha);
    git(
        &roundtrip_repo,
        &["update-ref", "refs/heads/main", roundtrip_hex.as_str()],
    )?;

    git(&roundtrip_repo, &["fsck", "--strict"])?;
    assert_eq!(
        git(&roundtrip_repo, &["cat-file", "-t", "main"])?.trim(),
        "commit"
    );
    let parent_line = git(&roundtrip_repo, &["log", "--format=%P", "-1", "main"])?;
    assert_eq!(parent_line.split_whitespace().count(), 2);

    git(&roundtrip_repo, &["checkout", "--force", "main"])?;
    assert_git_checkout(&roundtrip_repo, &unicode_file, &large)?;
    assert_eq!(
        fs::read(roundtrip_repo.join("feature.txt"))?,
        b"feature side\n"
    );
    assert_eq!(fs::read(roundtrip_repo.join("main.txt"))?, b"main side\n");

    Ok(())
}

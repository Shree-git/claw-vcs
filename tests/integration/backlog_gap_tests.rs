mod support;

use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;

use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_store::ClawStore;
use claw_sync::client::SyncClient;
use claw_sync::compat::{compatibility_report, CompatibilityLevel};
use claw_sync::proto::sync::sync_service_server::{SyncService, SyncServiceServer};
use claw_sync::proto::sync::{
    AdvertiseRefsRequest, AdvertiseRefsResponse, FetchObjectsRequest, HelloRequest, HelloResponse,
    ObjectChunk, PushObjectsResponse, UpdateRefsRequest, UpdateRefsResponse,
};
use support::CliTestEnv;
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

fn large_repo_file(repo: &Path, index: usize) -> PathBuf {
    repo.join("modules")
        .join(format!("{:02}", index % 12))
        .join(format!("component_{index:03}"))
        .join(format!("file_{index:03}.txt"))
}

#[test]
fn large_repo_synthetic_snapshot_status_and_checkout_scale() {
    const FILES: usize = 144;
    let env = CliTestEnv::new();
    let repo = env.init_repo("large-synthetic");

    for index in 0..FILES {
        env.write_file(
            &large_repo_file(&repo, index),
            &format!("component {index}\nline two\n"),
        );
    }

    let initial_status = env.run_ok(&repo, ["status", "--json"]);
    let initial_changes = initial_status.stdout_json()["changes"]
        .as_array()
        .expect("changes array")
        .clone();
    assert_eq!(initial_changes.len(), FILES);
    assert!(initial_changes
        .iter()
        .any(|change| change["path"] == "modules/00/component_000/file_000.txt"));

    env.run_ok(&repo, ["snapshot", "-m", "Synthetic large repo baseline"]);
    let clean = env.run_ok(&repo, ["status", "--json"]);
    assert_eq!(
        clean.stdout_json()["changes"]
            .as_array()
            .expect("clean changes")
            .len(),
        0
    );

    env.run_ok(&repo, ["branch", "create", "synthetic-edit"]);
    env.run_ok(&repo, ["checkout", "synthetic-edit"]);

    for index in 0..12 {
        env.write_file(
            &large_repo_file(&repo, index),
            &format!("component {index}\nmodified on feature\n"),
        );
    }
    for index in 20..26 {
        fs::remove_file(large_repo_file(&repo, index)).expect("delete synthetic file");
    }
    for index in 0..7 {
        env.write_file(
            &repo
                .join("new")
                .join(format!("fanout_{index:02}"))
                .join("added.txt"),
            &format!("new file {index}\n"),
        );
    }

    let dirty = env.run_ok(&repo, ["status", "--json"]);
    let dirty_json = dirty.stdout_json();
    let dirty_changes = dirty_json["changes"].as_array().expect("dirty changes");
    assert_eq!(dirty_changes.len(), 25);
    assert!(dirty_changes
        .iter()
        .any(|change| change["path"] == "new/fanout_00/added.txt" && change["status"] == "added"));
    assert!(dirty_changes.iter().any(|change| change["path"]
        == "modules/08/component_020/file_020.txt"
        && change["status"] == "deleted"));
    assert!(dirty_changes.iter().any(|change| change["path"]
        == "modules/00/component_000/file_000.txt"
        && change["status"] == "modified"));

    env.run_ok(&repo, ["snapshot", "-m", "Synthetic large repo edit"]);

    env.run_ok(&repo, ["checkout", "main"]);
    assert_eq!(
        env.read_file(&large_repo_file(&repo, 0)),
        "component 0\nline two\n"
    );
    assert!(large_repo_file(&repo, 20).exists());
    assert!(!repo
        .join("new")
        .join("fanout_00")
        .join("added.txt")
        .exists());

    env.run_ok(&repo, ["checkout", "synthetic-edit"]);
    assert_eq!(
        env.read_file(&large_repo_file(&repo, 0)),
        "component 0\nmodified on feature\n"
    );
    assert!(!large_repo_file(&repo, 20).exists());
    assert!(repo
        .join("new")
        .join("fanout_00")
        .join("added.txt")
        .exists());
}

#[test]
#[ignore = "10k-file large-repo drill is intentionally operator-triggered"]
fn large_repo_10k_file_snapshot_status_and_path_filter_drill() {
    const FILES: usize = 10_000;
    let env = CliTestEnv::new();
    let repo = env.init_repo("large-10k-synthetic");

    for index in 0..FILES {
        env.write_file(
            &large_repo_file(&repo, index),
            &format!("component {index}\nline two\n"),
        );
    }
    env.write_file(
        &repo.join("fixtures").join("large.json"),
        &format!(
            "{{\"items\":[{}]}}\n",
            (0..2048)
                .map(|idx| format!("{{\"id\":{idx},\"value\":\"v{idx}\"}}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    fs::create_dir_all(repo.join("fixtures")).expect("create fixtures directory");
    fs::write(
        repo.join("fixtures").join("large.bin"),
        vec![0x5au8; 1024 * 1024],
    )
    .expect("write large binary fixture");

    let status = env.run_ok(&repo, ["status", "--json"]);
    let status_json = status.stdout_json();
    let changes = status_json["changes"]
        .as_array()
        .expect("large repo status changes");
    assert_eq!(changes.len(), FILES + 2);

    env.run_ok(&repo, ["snapshot", "-m", "10k synthetic baseline"]);
    env.write_file(
        &large_repo_file(&repo, 9_999),
        "component 9999\npath-filter drill\n",
    );

    let dirty = env.run_ok(&repo, ["status", "--json"]);
    let dirty_json = dirty.stdout_json();
    let dirty_changes = dirty_json["changes"]
        .as_array()
        .expect("large repo dirty changes");
    assert_eq!(dirty_changes.len(), 1);
    assert_eq!(
        dirty_changes[0]["path"],
        "modules/03/component_9999/file_9999.txt"
    );
}

#[test]
fn admin_backup_verify_and_rollback_restore_corrupted_metadata() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("disaster-recovery");

    env.write_file(&repo.join("src").join("main.rs"), "fn main() {}\n");
    env.run_ok(&repo, ["snapshot", "-m", "Recoverable baseline"]);

    let created = env.run_ok(&repo, ["admin", "backup", "create"]);
    let backup_id = created.value_after("Created backup: ");
    let verified = env.run_ok(
        &repo,
        [
            "admin",
            "backup",
            "verify",
            "--backup-id",
            backup_id.as_str(),
        ],
    );
    assert!(verified
        .stdout
        .contains(&format!("Backup verified: {backup_id}")));

    let main_ref = repo.join(".claw").join("refs").join("heads").join("main");
    let original_ref = fs::read_to_string(&main_ref).expect("read original main ref");
    fs::write(
        &main_ref,
        "0000000000000000000000000000000000000000000000000000000000000000\n",
    )
    .expect("corrupt main ref");
    let stray_ref = repo.join(".claw").join("refs").join("heads").join("stray");
    fs::write(&stray_ref, "not-a-real-ref\n").expect("write stray ref");

    let plan = env.run_ok(
        &repo,
        [
            "admin",
            "rollback",
            "plan",
            "--backup-id",
            backup_id.as_str(),
        ],
    );
    assert!(plan
        .stdout
        .contains(&format!("Rollback plan from backup: {backup_id}")));

    let executed = env.run_ok(
        &repo,
        [
            "admin",
            "rollback",
            "execute",
            "--backup-id",
            backup_id.as_str(),
        ],
    );
    assert!(executed
        .stdout
        .contains(&format!("Rollback executed: {backup_id}")));

    assert_eq!(
        fs::read_to_string(&main_ref).expect("read restored main ref"),
        original_ref
    );
    assert!(
        !stray_ref.exists(),
        "rollback should remove metadata files absent from the backup snapshot"
    );

    env.run_ok(
        &repo,
        [
            "admin",
            "backup",
            "verify",
            "--backup-id",
            backup_id.as_str(),
        ],
    );
    let head = env.run_ok(&repo, ["show", "heads/main"]);
    assert!(head.stdout.contains("Recoverable baseline"));
}

#[test]
fn git_notes_export_and_import_roundtrip_policy_evidence() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("git-notes-interop");
    run_git_ok(&repo, &["init", "-q"]);

    let original_revision = seed_policy_evidence_revision(&repo);
    let exported = env.run_ok(
        &repo,
        [
            "git-export",
            "--git-notes",
            "--notes-ref",
            "claw-provenance",
        ],
    );
    assert!(exported
        .stdout
        .contains("wrote 1 provenance note(s) to refs/notes/claw-provenance"));
    let commit_hex = exported.value_after("SHA-1: ");

    let note = run_git_ok(
        &repo,
        &[
            "--git-dir",
            ".git",
            "notes",
            "--ref",
            "claw-provenance",
            "show",
            commit_hex.as_str(),
        ],
    );
    let note_json: serde_json::Value = serde_json::from_str(&note).expect("git note json");
    assert_eq!(note_json["revision_id"], original_revision);
    assert_eq!(note_json["policy_evidence"][0], "ci/git-notes=pass");

    let imported = env.run_ok(
        &repo,
        [
            "git-import",
            "--read-notes",
            "--notes-ref",
            "claw-provenance",
            "--git-ref",
            "refs/heads/claw/main",
            "--ref-name",
            "heads/imported-with-notes",
        ],
    );
    assert!(imported
        .stdout
        .contains("Imported 1 provenance note(s) from refs/notes/claw-provenance"));
    let imported_revision = imported.value_after("Revision: ");

    let store = ClawStore::open(&repo).expect("open store after git import");
    let evidence_ref = format!("notes/provenance/policy-evidence/{imported_revision}");
    let evidence_blob_id = store
        .get_ref(&evidence_ref)
        .expect("read imported evidence ref")
        .expect("evidence ref should exist");
    let evidence = match store
        .load_object(&evidence_blob_id)
        .expect("load evidence blob")
    {
        Object::Blob(blob) => {
            serde_json::from_slice::<Vec<String>>(&blob.data).expect("evidence json")
        }
        other => panic!("expected evidence blob, got {other:?}"),
    };
    assert_eq!(
        evidence,
        vec![
            "ci/git-notes=pass".to_string(),
            "artifact/provenance=present".to_string()
        ]
    );
}

#[test]
fn cli_dx_json_and_policy_dry_run_surfaces_are_machine_readable() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("cli-dx-json");

    let version = env.run_ok(env.temp_root(), ["version", "--json"]);
    let version_json = version.stdout_json();
    assert_eq!(version_json["object_format_version"], 1);
    assert_eq!(version_json["sync_protocol_version"], "claw-sync/1");
    assert!(version_json["build"]["target"].as_str().is_some());

    let doctor = env.run_ok(&repo, ["doctor", "--json"]);
    let doctor_json = doctor.stdout_json();
    let check_names: Vec<_> = doctor_json["checks"]
        .as_array()
        .expect("doctor checks")
        .iter()
        .filter_map(|check| check["name"].as_str())
        .collect();
    assert!(check_names.contains(&"git"));
    assert!(check_names.contains(&"object_format"));
    assert!(check_names.contains(&"refs"));
    assert!(check_names.contains(&"daemon_auth"));

    let dry_run = env.run_ok(
        &repo,
        [
            "policy",
            "apply",
            "--id",
            "release",
            "--check",
            "ci",
            "--dry-run",
            "--json",
        ],
    );
    let dry_json = dry_run.stdout_json();
    assert_eq!(dry_json["dry_run"], true);
    assert_eq!(dry_json["ref"], "policies/release");
    let store = ClawStore::open(&repo).expect("open repo after policy dry-run");
    assert!(
        store
            .get_ref("policies/release")
            .expect("read policy ref")
            .is_none(),
        "policy apply --dry-run must not write the policy ref"
    );

    env.run_ok(
        &repo,
        ["policy", "create", "--id", "release", "--check", "ci"],
    );
    let shown = env.run_ok(&repo, ["show", "--json", "policies/release"]);
    let shown_json = shown.stdout_json();
    assert_eq!(shown_json["object"]["type"], "policy");
    assert_eq!(
        shown_json["object"]["value"]["Policy"]["policy_id"],
        "release"
    );
}

#[test]
fn git_export_dry_run_skips_git_writes() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("git-export-dry-run");
    env.write_file(&repo.join("hello.txt"), "hello\n");
    env.run_ok(&repo, ["snapshot", "-m", "initial"]);

    let git_dir = repo.join("exported.git");
    let dry_run = env.run_ok(
        &repo,
        [
            "git-export",
            "--git-dir",
            git_dir.to_str().expect("git dir utf-8"),
            "--dry-run",
        ],
    );

    assert!(dry_run.stdout.contains("Dry run: would export"));
    assert!(
        !git_dir.exists(),
        "git-export --dry-run must not create the target git directory"
    );
}

#[test]
fn git_import_dry_run_skips_claw_ref_writes() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("git-import-dry-run");
    let git_repo = env.repo_path("source-git");
    fs::create_dir_all(&git_repo).expect("create git source repo");
    run_git_ok(&git_repo, &["init", "-q"]);
    run_git_ok(&git_repo, &["config", "user.name", "Claw Tests"]);
    run_git_ok(&git_repo, &["config", "user.email", "tests@example.com"]);
    fs::write(git_repo.join("hello.txt"), "hello from git\n").expect("write git source file");
    run_git_ok(&git_repo, &["add", "hello.txt"]);
    run_git_ok(&git_repo, &["commit", "-q", "-m", "initial"]);
    run_git_ok(&git_repo, &["branch", "-M", "main"]);

    let dry_run = env.run_ok(
        &repo,
        [
            "git-import",
            "--git-dir",
            git_repo
                .join(".git")
                .to_str()
                .expect("git source path utf-8"),
            "--git-ref",
            "refs/heads/main",
            "--ref-name",
            "heads/imported",
            "--dry-run",
        ],
    );

    assert!(dry_run.stdout.contains("Dry run: would import"));
    let store = ClawStore::open(&repo).expect("open repo after git import dry-run");
    assert!(
        store
            .get_ref("heads/imported")
            .expect("read imported ref")
            .is_none(),
        "git-import --dry-run must not write the destination ref"
    );
}

#[tokio::test]
async fn remote_compatibility_classification_is_exercised_over_hello() {
    let local = env!("CARGO_PKG_VERSION");
    let cases = [
        (local.to_string(), CompatibilityLevel::Full),
        (adjacent_minor_version(local), CompatibilityLevel::Limited),
        (next_major_version(local), CompatibilityLevel::Unsupported),
    ];

    for (server_version, expected) in cases {
        let (endpoint, shutdown) = spawn_hello_remote(server_version.clone()).await;
        let mut client = connect_with_retry(&endpoint).await;
        let hello = client.hello().await.expect("remote hello");
        let report = compatibility_report(local, &hello.server_version);

        assert_eq!(hello.server_version, server_version);
        assert_eq!(report.level, expected);
        assert!(
            hello.capabilities.iter().any(|cap| cap == "partial-clone"),
            "compatibility remotes should expose the baseline sync capability"
        );

        let _ = shutdown.send(());
    }
}

fn seed_policy_evidence_revision(repo: &Path) -> String {
    let store = ClawStore::open(repo).expect("open seeded store");
    let blob_id = store
        .store_object(&Object::Blob(Blob {
            data: b"tracked through git notes\n".to_vec(),
            media_type: Some("text/plain".to_string()),
        }))
        .expect("store blob");
    let tree_id = store
        .store_object(&Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: "provenance.txt".to_string(),
                mode: FileMode::Regular,
                object_id: blob_id,
            }],
        }))
        .expect("store tree");
    let revision_id = store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: "integration-test".to_string(),
            created_at_ms: 1_700_000_000_000,
            summary: "Revision with policy evidence".to_string(),
            policy_evidence: vec![
                "ci/git-notes=pass".to_string(),
                "artifact/provenance=present".to_string(),
            ],
        }))
        .expect("store revision");
    store
        .set_ref("heads/main", &revision_id)
        .expect("seed main ref");
    fs::write(repo.join("provenance.txt"), "tracked through git notes\n")
        .expect("materialize provenance file");
    revision_id.to_hex()
}

fn run_git_ok(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("run git {:?}: {err}", args));
    assert!(
        output.status.success(),
        "git command failed in {}\n$ git {}\nstdout:\n{}\nstderr:\n{}",
        cwd.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[derive(Clone)]
struct HelloOnlyService {
    server_version: String,
}

#[tonic::async_trait]
impl SyncService for HelloOnlyService {
    type FetchObjectsStream = ReceiverStream<Result<ObjectChunk, Status>>;

    async fn hello(
        &self,
        _request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        Ok(Response::new(HelloResponse {
            server_version: self.server_version.clone(),
            capabilities: vec!["partial-clone".to_string()],
        }))
    }

    async fn advertise_refs(
        &self,
        _request: Request<AdvertiseRefsRequest>,
    ) -> Result<Response<AdvertiseRefsResponse>, Status> {
        Err(Status::unimplemented("hello-only compatibility service"))
    }

    async fn fetch_objects(
        &self,
        _request: Request<FetchObjectsRequest>,
    ) -> Result<Response<Self::FetchObjectsStream>, Status> {
        Err(Status::unimplemented("hello-only compatibility service"))
    }

    async fn push_objects(
        &self,
        _request: Request<tonic::Streaming<ObjectChunk>>,
    ) -> Result<Response<PushObjectsResponse>, Status> {
        Err(Status::unimplemented("hello-only compatibility service"))
    }

    async fn update_refs(
        &self,
        _request: Request<UpdateRefsRequest>,
    ) -> Result<Response<UpdateRefsResponse>, Status> {
        Err(Status::unimplemented("hello-only compatibility service"))
    }
}

async fn spawn_hello_remote(server_version: String) -> (String, oneshot::Sender<()>) {
    let addr = free_local_addr();
    let endpoint = format!("http://{addr}");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let service = HelloOnlyService { server_version };

    tokio::spawn(async move {
        Server::builder()
            .add_service(SyncServiceServer::new(service))
            .serve_with_shutdown(addr, async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("serve compatibility hello remote");
    });

    (endpoint, shutdown_tx)
}

async fn connect_with_retry(endpoint: &str) -> SyncClient {
    let mut last_error = String::new();
    for _ in 0..50 {
        match SyncClient::connect(endpoint).await {
            Ok(client) => return client,
            Err(err) => {
                last_error = err.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    }

    panic!("failed to connect to compatibility remote {endpoint}: {last_error}");
}

fn free_local_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test port");
    let addr = listener.local_addr().expect("read local test port");
    drop(listener);
    addr
}

fn adjacent_minor_version(version: &str) -> String {
    let (major, minor) = major_minor(version);
    format!("{major}.{}.0", minor + 1)
}

fn next_major_version(version: &str) -> String {
    let (major, _minor) = major_minor(version);
    format!("{}.0.0", major + 1)
}

fn major_minor(version: &str) -> (u64, u64) {
    let clean = version.trim_start_matches('v');
    let mut parts = clean.split('.');
    let major = parts
        .next()
        .and_then(|part| part.parse::<u64>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|part| part.parse::<u64>().ok())
        .unwrap_or(0);
    (major, minor)
}

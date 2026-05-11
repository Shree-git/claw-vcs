#[path = "live_daemon_support.rs"]
mod live_daemon_support;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::Revision;
use claw_store::ClawStore;
use claw_sync::client::SyncClient;
use claw_sync::compat::{compatibility_report, CompatibilityLevel};
use live_daemon_support::{init_temp_repo, read_workspace_file, run_claw_command, LiveDaemon};
use serde_json::Value;

fn store_revision(
    store: &ClawStore,
    parent: Option<ObjectId>,
    summary: &str,
    created_at_ms: u64,
) -> ObjectId {
    store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: parent.into_iter().collect(),
            patches: vec![],
            snapshot_base: None,
            tree: None,
            capsule_id: None,
            author: "integration-test".to_string(),
            created_at_ms,
            summary: summary.to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision")
}

#[tokio::test]
async fn live_daemon_hello_matches_the_published_current_wire_version() {
    let repo = init_temp_repo();
    let daemon = LiveDaemon::spawn(repo.path(), &[]).await;

    let mut client = SyncClient::connect(&daemon.grpc_endpoint)
        .await
        .expect("connect runtime gRPC client");
    let hello = client.hello().await.expect("invoke live daemon hello");

    assert_eq!(hello.server_version, env!("CARGO_PKG_VERSION"));
    assert!(
        hello.capabilities.iter().any(|cap| cap == "partial-clone"),
        "live daemon should advertise partial-clone capability"
    );
    assert_eq!(
        compatibility_report(env!("CARGO_PKG_VERSION"), &hello.server_version).level,
        CompatibilityLevel::Full
    );

    let compatibility_doc = read_workspace_file("docs/reference/compatibility.md");
    assert!(
        compatibility_doc.contains(&format!("server_version: {}", hello.server_version)),
        "compatibility reference should document the live daemon hello version"
    );
}

#[tokio::test]
async fn compat_check_clone_succeeds_end_to_end_against_the_live_same_version_daemon() {
    let remote_repo = init_temp_repo();
    let remote_store = ClawStore::open(remote_repo.path()).expect("open remote store");
    let base = store_revision(&remote_store, None, "base", 1);
    let head = store_revision(&remote_store, Some(base), "head", 2);
    remote_store
        .set_ref("heads/main", &head)
        .expect("seed remote main ref");

    let daemon = LiveDaemon::spawn(remote_repo.path(), &[]).await;

    let clone_dir = tempfile::tempdir().expect("create clone dir");
    let clone_path = clone_dir.path().join("clone-target");
    let output = run_claw_command(
        remote_repo.path(),
        &[
            "--compat-check",
            "sync",
            "clone",
            &daemon.grpc_endpoint,
            clone_path.to_str().expect("clone path utf-8"),
        ],
    );

    assert!(
        output.status.success(),
        "compat-check clone failed: stdout=\n{}\n\nstderr=\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let cloned_store = ClawStore::open(&clone_path).expect("open cloned store");
    assert_eq!(
        cloned_store
            .get_ref("heads/main")
            .expect("read cloned main ref"),
        Some(head)
    );
}

#[test]
fn version_fixtures_cover_limited_and_unsupported_runtime_classifications() {
    let raw = read_workspace_file("tests/integration/version_compatibility_fixtures.json");
    let fixtures: Vec<Value> =
        serde_json::from_str(&raw).expect("version compatibility fixtures should parse");

    for fixture in fixtures {
        let case = fixture
            .get("case")
            .and_then(Value::as_str)
            .expect("fixture case label");
        let local = fixture
            .get("local")
            .and_then(Value::as_str)
            .expect("fixture local version");
        let remote = fixture
            .get("remote")
            .and_then(Value::as_str)
            .expect("fixture remote version");
        let expected = fixture
            .get("expected")
            .and_then(Value::as_str)
            .expect("fixture expected level");

        let level = compatibility_report(local, remote).level;
        let expected_level = match expected {
            "full" => CompatibilityLevel::Full,
            "limited" => CompatibilityLevel::Limited,
            "unsupported" => CompatibilityLevel::Unsupported,
            other => panic!("unsupported fixture expected level: {other}"),
        };

        assert_eq!(level, expected_level, "fixture case failed: {case}");
    }
}

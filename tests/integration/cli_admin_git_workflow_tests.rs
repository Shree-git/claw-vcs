mod support;

use std::path::Path;
use std::process::Command;

use serde_json::Value;
use support::CliTestEnv;

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

#[test]
fn integrate_command_merges_feature_branch_and_updates_the_worktree() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("integrate-workflow");

    env.write_file(&repo.join("feature.txt"), "base\n");
    env.run_ok(&repo, ["snapshot", "-m", "Base revision"]);
    env.run_ok(&repo, ["branch", "create", "feature"]);
    env.run_ok(&repo, ["checkout", "feature"]);

    env.write_file(&repo.join("feature.txt"), "feature branch\n");
    env.run_ok(&repo, ["snapshot", "-m", "Feature revision"]);
    env.run_ok(&repo, ["checkout", "main"]);

    let integrated = env.run_ok(
        &repo,
        [
            "integrate",
            "--right",
            "heads/feature",
            "-m",
            "Merge feature",
        ],
    );
    assert!(integrated.stdout.contains("Integrated successfully"));
    assert_eq!(env.read_file(&repo.join("feature.txt")), "feature branch\n");

    let head = env.run_ok(&repo, ["show", "heads/main"]);
    assert!(head.stdout.contains("Merge feature"));
    assert!(head.stdout.contains("parents:"));

    let status = env.run_ok(&repo, ["status", "--json"]);
    assert_eq!(
        status.stdout_json()["changes"]
            .as_array()
            .expect("status changes array")
            .len(),
        0
    );
}

#[test]
fn admin_preflight_and_support_bundle_match_operator_docs() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("admin-operator-docs");

    let preflight = env.run_ok(&repo, ["admin", "preflight"]);
    assert!(preflight.stdout.contains("Preflight: PASS"));
    assert!(preflight.stdout.contains("metadata directory"));
    assert!(preflight.stdout.contains("tls configuration"));

    let bundle_path = env.temp_root().join("support-bundle.json");
    let bundle = env.run_ok(
        &repo,
        [
            "admin",
            "support-bundle",
            "--out",
            bundle_path.to_str().expect("support bundle path utf-8"),
        ],
    );
    assert!(bundle.stdout.contains("Support bundle written:"));
    assert!(
        bundle_path.exists(),
        "support bundle file should be written"
    );

    let bundle_json: Value =
        serde_json::from_slice(&std::fs::read(&bundle_path).expect("read support bundle JSON"))
            .expect("support bundle must be valid JSON");
    let expected_repo_root = std::fs::canonicalize(&repo).expect("canonicalize repo root");
    assert_eq!(
        bundle_json["repo_root"].as_str(),
        Some(expected_repo_root.to_str().expect("repo path utf-8"))
    );
    assert!(
        bundle_json["request_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("req_")),
        "support bundle must include a generated request id"
    );
    assert!(
        bundle_json["refs_count"].as_u64().is_some(),
        "support bundle must include refs_count"
    );

    let ledger = std::fs::read_to_string(repo.join(".claw/migrations/ledger.jsonl"))
        .expect("support bundle should append admin ledger entry");
    assert!(ledger.contains("\"action\":\"support-bundle\""));
}

#[test]
fn admin_migrate_and_git_bridge_commands_work_end_to_end() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("git-bridge");

    run_git_ok(&repo, &["init", "-q"]);
    env.write_file(&repo.join("hello.txt"), "hello from claw\n");
    env.run_ok(&repo, ["snapshot", "-m", "Seed revision"]);

    let migration_plan = env.run_ok(&repo, ["admin", "migrate", "plan"]);
    assert!(migration_plan.stdout.contains("Migration plan ->"));
    assert!(migration_plan.stdout.contains(".claw/config.toml"));

    let dry_run = env.run_ok(&repo, ["admin", "migrate", "apply", "--dry-run"]);
    assert!(dry_run
        .stdout
        .contains("Dry run complete. No files changed."));

    let applied = env.run_ok(&repo, ["admin", "migrate", "apply"]);
    assert!(applied.stdout.contains("Migration applied."));
    let ledger = std::fs::read_to_string(repo.join(".claw/migrations/ledger.jsonl"))
        .expect("migration ledger should be written");
    assert!(ledger.contains("\"action\":\"migrate.apply\""));

    let exported = env.run_ok(&repo, ["git-export"]);
    assert!(exported
        .stdout
        .contains("Exported to git: refs/heads/claw/main"));
    let exported_ref = run_git_ok(&repo, &["rev-parse", "--verify", "refs/heads/claw/main"]);
    assert_eq!(exported_ref.trim().len(), 40);

    let imported = env.run_ok(
        &repo,
        [
            "git-import",
            "--git-ref",
            "refs/heads/claw/main",
            "--ref-name",
            "heads/imported",
        ],
    );
    assert!(imported
        .stdout
        .contains("Imported git ref refs/heads/claw/main -> heads/imported"));

    let imported_ref = env.run_ok(&repo, ["show", "heads/imported"]);
    assert!(imported_ref.stdout.contains("Seed revision"));

    let roundtrip = env.run_ok(&repo, ["git-roundtrip"]);
    assert!(roundtrip.stdout.contains("Roundtrip verified."));
    assert!(roundtrip
        .stdout
        .contains("Imported ref: heads/roundtrip-verify"));
}

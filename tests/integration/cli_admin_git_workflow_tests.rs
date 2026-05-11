mod support;

use std::path::Path;
use std::process::Command;

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

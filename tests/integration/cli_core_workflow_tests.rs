mod support;

use claw_crypto::recipient::recipient_public_key;
use support::CliTestEnv;

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn core_cli_workflow_covers_init_snapshot_and_ship() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("core-workflow");

    let branch = env.run_ok(&repo, ["branch"]);
    assert!(branch.stdout.contains("* main (no commits yet)"));

    let status = env.run_ok(&repo, ["status", "--json"]);
    let json = status.stdout_json();
    assert_eq!(json["branch"], "main");
    assert_eq!(json["in_merge"], false);
    assert_eq!(json["changes"].as_array().map(Vec::len), Some(0));

    let intent = env.run_ok(
        &repo,
        [
            "intent",
            "new",
            "--title",
            "Core workflow coverage",
            "--goal",
            "Exercise the local CLI path",
        ],
    );
    let intent_id = intent.value_after("Created intent: ");

    let listed_intents = env.run_ok(&repo, ["intent", "list"]);
    assert!(listed_intents.stdout.contains(&intent_id));
    assert!(listed_intents.stdout.contains("Core workflow coverage"));

    let change = env.run_ok(&repo, ["change", "new", "--intent", intent_id.as_str()]);
    let change_id = change.value_after("Created change: ");

    let listed_changes = env.run_ok(&repo, ["change", "list", "--intent", intent_id.as_str()]);
    assert!(listed_changes.stdout.contains(&change_id));

    env.write_file(
        &repo.join("src/lib.rs"),
        "pub fn greeting() -> &'static str {\n    \"hello from claw\"\n}\n",
    );

    let dirty_status = env.run_ok(&repo, ["status", "--json"]);
    let dirty_json = dirty_status.stdout_json();
    let changes = dirty_json["changes"]
        .as_array()
        .expect("status changes to be an array");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["path"], "src/lib.rs");
    assert_eq!(changes[0]["status"], "added");

    let snapshot = env.run_ok(
        &repo,
        [
            "snapshot",
            "-m",
            "Initial workflow snapshot",
            "--change",
            change_id.as_str(),
        ],
    );
    let revision_display = snapshot.value_after("Snapshot: ");
    assert!(revision_display.starts_with("clw_"));

    let show_head = env.run_ok(&repo, ["show", "heads/main"]);
    assert!(show_head.stdout.to_ascii_lowercase().contains("revision"));
    assert!(show_head.stdout.contains("change_id"));
    assert!(show_head.stdout.contains(&change_id));
    assert!(show_head.stdout.contains("Initial workflow snapshot"));

    let log_before_ship = env.run_ok(&repo, ["log", "--json"]);
    let entries_before_ship = log_before_ship
        .stdout_json()
        .as_array()
        .cloned()
        .expect("log json array");
    assert_eq!(entries_before_ship.len(), 1);
    assert_eq!(
        entries_before_ship[0]["change_id"].as_str(),
        Some(change_id.as_str())
    );
    assert!(entries_before_ship[0].get("capsule_id").is_none());

    let recipient_secret = [9u8; 32];
    let recipient_public = recipient_public_key(&recipient_secret);
    env.write_file(
        &repo.join("capsule-private.json"),
        "{\"ticket\":\"SEC-1\",\"note\":\"reviewed\"}\n",
    );
    env.write_file(&repo.join("security.x25519"), &to_hex(&recipient_secret));

    let shipped = env.run_ok(
        &repo,
        [
            "ship",
            "--intent",
            intent_id.as_str(),
            "--evidence",
            "test=pass:42",
            "--evidence",
            "lint=pass",
            "--evidence-command",
            "cargo test --workspace",
            "--runner",
            "github-actions/release",
            "--environment-digest",
            "sha256:toolchain",
            "--log-digest",
            "sha256:log",
            "--evidence-expires-in-ms",
            "86400000",
            "--private-file",
            "capsule-private.json",
            "--recipient-key",
            &format!("security:security-key:{}", to_hex(&recipient_public)),
        ],
    );
    let capsule_id = shipped.value_after("Capsule: ");
    assert!(capsule_id.starts_with("clw_"));

    let show_capsule = env.run_ok(&repo, ["show", capsule_id.as_str()]);
    assert!(show_capsule.stdout.contains("agent_id"));
    assert!(show_capsule.stdout.contains("test (pass)"));
    assert!(show_capsule.stdout.contains("lint (pass)"));
    assert!(show_capsule.stdout.contains("private"));
    assert!(show_capsule.stdout.contains("security (security-key)"));

    let capsule_json = env.run_ok(&repo, ["show", "--json", capsule_id.as_str()]);
    let capsule_value = capsule_json.stdout_json();
    let evidence = &capsule_value["object"]["value"]["Capsule"]["public_fields"]["evidence"][0];
    assert_eq!(evidence["command"], "cargo test --workspace");
    assert_eq!(evidence["runner_identity"], "github-actions/release");
    assert_eq!(evidence["environment_digest"], "sha256:toolchain");
    assert_eq!(evidence["log_digest"], "sha256:log");

    let decrypted = env.run_ok(
        &repo,
        [
            "show",
            capsule_id.as_str(),
            "--decrypt-private",
            "--recipient",
            "security",
            "--recipient-secret-key",
            "security.x25519",
        ],
    );
    assert!(decrypted.stdout.contains("\"ticket\":\"SEC-1\""));

    let intent_show = env.run_ok(&repo, ["intent", "show", intent_id.as_str()]);
    assert!(intent_show.stdout.contains("Status: Done"));

    let change_show = env.run_ok(&repo, ["change", "show", change_id.as_str()]);
    assert!(change_show.stdout.contains("Status: Integrated"));

    let log_after_ship = env.run_ok(&repo, ["log", "--json"]);
    let entries_after_ship = log_after_ship
        .stdout_json()
        .as_array()
        .cloned()
        .expect("log json array");
    assert_eq!(entries_after_ship.len(), 1);
    assert_eq!(
        entries_after_ship[0]["capsule_id"].as_str(),
        Some(capsule_id.as_str())
    );
}

#[test]
fn agent_revoke_blocks_ship_until_rotation() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("agent-revoke-rotate");

    let intent = env.run_ok(
        &repo,
        [
            "intent",
            "create",
            "--title",
            "Agent lifecycle",
            "--goal",
            "Exercise explicit agent key revocation",
        ],
    );
    let intent_id = intent.value_after("Created intent: ");
    let change = env.run_ok(&repo, ["change", "create", "--intent", intent_id.as_str()]);
    let change_id = change.value_after("Created change: ");

    env.write_file(&repo.join("README.md"), "agent lifecycle\n");
    env.run_ok(
        &repo,
        [
            "snapshot",
            "-m",
            "tracked revision",
            "--change",
            change_id.as_str(),
        ],
    );

    env.run_ok(&repo, ["agent", "register", "--name", "ci-agent"]);

    let dry_revoke = env.run_ok(
        &repo,
        [
            "agent",
            "revoke",
            "--name",
            "ci-agent",
            "--reason",
            "compromised",
            "--dry-run",
        ],
    );
    assert!(dry_revoke.stdout.contains("Dry run: would revoke agent"));

    let active = env.run_ok(&repo, ["agent", "status", "ci-agent"]);
    assert!(active.stdout.contains("Status: active"));

    env.run_ok(
        &repo,
        [
            "agent",
            "revoke",
            "--name",
            "ci-agent",
            "--reason",
            "compromised",
        ],
    );
    let revoked = env.run_ok(&repo, ["agent", "status", "ci-agent"]);
    assert!(revoked.stdout.contains("Status: revoked"));
    assert!(revoked.stdout.contains("compromised"));

    let blocked = env.run_fail(
        &repo,
        [
            "ship",
            "--intent",
            intent_id.as_str(),
            "--agent",
            "ci-agent",
            "--evidence",
            "test=pass",
        ],
    );
    assert!(blocked
        .combined_output()
        .contains("agent 'ci-agent' is revoked"));

    env.run_ok(
        &repo,
        [
            "agent",
            "rotate",
            "--name",
            "ci-agent",
            "--version",
            "rotated",
        ],
    );
    let rotated = env.run_ok(&repo, ["agent", "status", "ci-agent"]);
    assert!(rotated.stdout.contains("Status: active"));
    assert!(rotated.stdout.contains("Version: rotated"));

    let shipped = env.run_ok(
        &repo,
        [
            "ship",
            "--intent",
            intent_id.as_str(),
            "--agent",
            "ci-agent",
            "--evidence",
            "test=pass",
        ],
    );
    assert!(shipped.stdout.contains("Capsule: "));
}

#[test]
fn checkout_requires_force_when_worktree_is_dirty() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("checkout-safety");

    env.write_file(&repo.join("tracked.txt"), "base line\n");
    env.run_ok(&repo, ["snapshot", "-m", "Base snapshot"]);
    env.run_ok(&repo, ["branch", "create", "feature"]);

    env.write_file(&repo.join("tracked.txt"), "dirty line\n");
    let blocked = env.run_fail(&repo, ["checkout", "feature"]);
    assert!(blocked.combined_output().contains("uncommitted changes"));

    let forced = env.run_ok(&repo, ["checkout", "--force", "feature"]);
    assert!(forced.stdout.contains("Switched to branch 'feature'"));
    assert_eq!(env.read_file(&repo.join("tracked.txt")), "base line\n");
}

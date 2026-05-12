mod support;

use support::CliTestEnv;

#[test]
fn daemon_auth_health_and_sync_round_trip_work_via_built_binary() {
    let env = CliTestEnv::new();
    let source = env.init_repo("source");

    env.run_ok(
        env.temp_root(),
        [
            "auth",
            "token",
            "set",
            "super-secret-token",
            "--profile",
            "e2e",
            "--base-url",
            "http://127.0.0.1",
        ],
    );

    env.write_file(&source.join("sync.txt"), "version one\n");
    env.run_ok(&source, ["snapshot", "-m", "Seed remote repository"]);

    let daemon = env.spawn_daemon(&source, ["--auth-profile", "e2e"]);

    let live = daemon.get("/v1/health/live");
    assert_eq!(live.status_code, 200);
    assert!(live.body.contains("\"status\":\"live\""));

    let ready = daemon.get("/v1/health/ready");
    assert_eq!(ready.status_code, 200);
    assert!(ready.body.contains("\"status\":\"ready\""));

    let metrics = daemon.get("/v1/metrics");
    assert_eq!(metrics.status_code, 200);
    assert!(metrics.status_line.contains("200"));
    assert!(metrics
        .body
        .contains("claw_daemon_http_request_latency_seconds"));
    assert!(metrics.body.contains("claw_daemon_auth_failures_total"));

    let unauth_clone = env.repo_path("clone-without-token");
    let denied = env.run_fail(
        env.temp_root(),
        [
            "sync",
            "clone",
            daemon.grpc_endpoint.as_str(),
            unauth_clone.to_str().expect("clone path utf-8"),
        ],
    );
    assert!(
        denied.combined_output().contains("missing bearer token")
            || denied.combined_output().contains("Unauthenticated")
    );

    env.run_ok(
        env.temp_root(),
        [
            "auth",
            "token",
            "set",
            "stale-or-wrong-token",
            "--profile",
            "stale",
            "--base-url",
            "http://127.0.0.1",
        ],
    );
    let stale_token_clone = env.repo_path("clone-with-stale-token");
    let stale_denied = env.run_fail(
        env.temp_root(),
        [
            "sync",
            "clone",
            "--token-profile",
            "stale",
            daemon.grpc_endpoint.as_str(),
            stale_token_clone.to_str().expect("clone path utf-8"),
        ],
    );
    assert!(
        stale_denied
            .combined_output()
            .contains("invalid bearer token")
            || stale_denied.combined_output().contains("Unauthenticated")
    );

    let clone_a = env.repo_path("clone-a");
    let clone_b = env.repo_path("clone-b");

    let clone_a_out = env.run_ok(
        env.temp_root(),
        [
            "sync",
            "clone",
            "--token-profile",
            "e2e",
            daemon.grpc_endpoint.as_str(),
            clone_a.to_str().expect("clone-a path utf-8"),
        ],
    );
    assert!(clone_a_out.stdout.contains("Cloned"));
    assert_eq!(env.read_file(&clone_a.join("sync.txt")), "version one\n");

    env.run_ok(
        env.temp_root(),
        [
            "sync",
            "clone",
            "--token-profile",
            "e2e",
            daemon.grpc_endpoint.as_str(),
            clone_b.to_str().expect("clone-b path utf-8"),
        ],
    );
    assert_eq!(env.read_file(&clone_b.join("sync.txt")), "version one\n");

    let remotes = env.run_ok(&clone_a, ["remote", "list"]);
    assert!(remotes.stdout.contains("origin\tgrpc\t"));
    assert!(remotes.stdout.contains("\te2e"));

    env.write_file(&clone_a.join("sync.txt"), "version two from clone a\n");
    env.run_ok(&clone_a, ["snapshot", "-m", "Advance remote head"]);

    let pushed = env.run_ok(&clone_a, ["sync", "push", "--remote", "origin"]);
    assert!(pushed.stdout.contains("Pushed heads/main to origin"));

    assert_eq!(env.read_file(&clone_b.join("sync.txt")), "version one\n");
    let pulled = env.run_ok(&clone_b, ["sync", "pull", "--remote", "origin"]);
    assert!(pulled.stdout.contains("Working tree updated."));
    assert_eq!(
        env.read_file(&clone_b.join("sync.txt")),
        "version two from clone a\n"
    );
}

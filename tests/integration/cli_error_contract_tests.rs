mod support;

use support::CliTestEnv;

fn sorted_keys(value: &serde_json::Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .expect("json value to be an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

#[test]
fn json_error_format_wraps_failures_with_a_machine_readable_envelope() {
    let env = CliTestEnv::new();

    let result = env.run_fail(env.temp_root(), ["--error-format", "json", "status"]);
    let error = result.stderr_json();

    assert_eq!(
        sorted_keys(&error),
        vec![
            "code",
            "details",
            "exit_code",
            "message",
            "remediation",
            "request_id"
        ]
    );
    assert_eq!(error["code"], "NOT_REPOSITORY");
    assert!(error["message"]
        .as_str()
        .expect("error message to be a string")
        .contains("not in a claw repository"));
    assert!(error["request_id"]
        .as_str()
        .expect("request_id to be a string")
        .starts_with("req_"));
    assert_eq!(error["exit_code"], 3);
    assert!(error["remediation"]
        .as_str()
        .expect("remediation to be a string")
        .contains("claw init"));
}

#[test]
fn json_error_format_wraps_usage_failures() {
    let env = CliTestEnv::new();

    let result = env.run_fail(
        env.temp_root(),
        ["--error-format", "json", "--not-a-real-flag"],
    );
    let error = result.stderr_json();

    assert_eq!(error["code"], "USAGE_ERROR");
    assert!(error["message"]
        .as_str()
        .expect("error message to be a string")
        .contains("--not-a-real-flag"));
    assert!(error["request_id"]
        .as_str()
        .expect("request_id to be a string")
        .starts_with("req_"));
    assert_eq!(error["exit_code"], 2);
    assert_eq!(error["details"]["kind"], "UnknownArgument");
    assert!(error["remediation"]
        .as_str()
        .expect("remediation to be a string")
        .contains("claw --help"));
}

#[test]
fn json_error_format_classifies_remote_failures() {
    let env = CliTestEnv::new();
    let repo = env.init_repo("json-remote-error");

    let result = env.run_fail(
        &repo,
        [
            "--error-format",
            "json",
            "sync",
            "pull",
            "--remote",
            "origin",
        ],
    );
    let error = result.stderr_json();

    assert_eq!(
        sorted_keys(&error),
        vec![
            "code",
            "details",
            "exit_code",
            "message",
            "remediation",
            "request_id"
        ]
    );
    assert_eq!(error["code"], "REMOTE_ERROR");
    assert_eq!(error["exit_code"], 7);
    assert!(error["remediation"]
        .as_str()
        .expect("remediation to be a string")
        .contains("claw remote list"));
}

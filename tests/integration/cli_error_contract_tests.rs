mod support;

use support::CliTestEnv;

#[test]
fn json_error_format_wraps_failures_with_a_machine_readable_envelope() {
    let env = CliTestEnv::new();

    let result = env.run_fail(env.temp_root(), ["--error-format", "json", "status"]);
    let error = result.stderr_json();

    assert_eq!(error["code"], "CLI_ERROR");
    assert!(error["message"]
        .as_str()
        .expect("error message to be a string")
        .contains("not in a claw repository"));
    assert!(error["request_id"]
        .as_str()
        .expect("request_id to be a string")
        .starts_with("req_"));
}

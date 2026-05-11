#[path = "live_daemon_support.rs"]
mod live_daemon_support;

use live_daemon_support::{
    init_temp_repo, json_body, raw_http_request, read_workspace_json, LiveDaemon,
};
use serde_json::Value;

#[tokio::test]
async fn documented_health_paths_are_live_and_match_the_envelope_contract() {
    let repo = init_temp_repo();
    let daemon = LiveDaemon::spawn(repo.path(), &[]).await;

    let openapi = read_workspace_json("docs/reference/daemon-http-openapi-v1.json");
    let paths = openapi
        .get("paths")
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must contain object key: paths");
    let schemas = openapi
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must contain components.schemas");
    let envelope = schemas
        .get("HealthEnvelope")
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must define components.schemas.HealthEnvelope");
    let required_fields = envelope
        .get("required")
        .and_then(Value::as_array)
        .expect("HealthEnvelope must declare required field list");

    for path in [
        "/v1/health/live",
        "/v1/health/ready",
        "/v1/health/deps",
        "/v1/metrics",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI artifact missing required path: {path}"
        );
    }

    for field in ["code", "message", "request_id", "details"] {
        assert!(
            required_fields
                .iter()
                .any(|item| item.as_str() == Some(field)),
            "HealthEnvelope required list missing field: {field}"
        );
    }

    for (path, request_id, expected_status) in [
        ("/v1/health/live", "live-check-1", "live"),
        ("/v1/health/ready", "ready-check-1", "ready"),
        ("/v1/health/deps", "deps-check-1", "ok"),
    ] {
        let response = raw_http_request(
            daemon.health_addr,
            "GET",
            path,
            &[("X-Request-Id", request_id)],
            &[],
        )
        .await
        .expect("issue live health request");

        assert_eq!(response.status_line, "HTTP/1.1 200 OK");
        let body = json_body(&response);
        assert_eq!(body.get("code").and_then(Value::as_str), Some("ok"));
        assert_eq!(
            body.get("request_id").and_then(Value::as_str),
            Some(request_id)
        );
        assert_eq!(
            body.pointer("/details/status").and_then(Value::as_str),
            Some(expected_status)
        );
        for field in ["code", "message", "request_id", "details"] {
            assert!(body.get(field).is_some(), "live response missing {field}");
        }
    }
}

#[tokio::test]
async fn live_metrics_and_error_paths_match_documented_http_behavior() {
    let repo = init_temp_repo();
    let daemon = LiveDaemon::spawn(repo.path(), &[]).await;

    let metrics = raw_http_request(daemon.health_addr, "GET", "/v1/metrics", &[], &[])
        .await
        .expect("issue live metrics request");
    assert_eq!(metrics.status_line, "HTTP/1.1 200 OK");
    assert!(metrics
        .headers
        .contains("content-type: text/plain; version=0.0.4; charset=utf-8"));
    let metrics_body = String::from_utf8(metrics.body).expect("metrics body should be utf-8");
    assert!(metrics_body.contains("# HELP claw_daemon_http_request_latency_seconds"));
    assert!(metrics_body.contains("# HELP claw_daemon_auth_failures_total"));
    assert!(metrics_body.contains("# HELP claw_daemon_retry_total"));
    assert!(metrics_body.contains("# HELP claw_daemon_policy_eval_duration_seconds"));
    assert!(metrics_body.contains("# HELP claw_daemon_queue_depth"));
    assert!(metrics_body.contains("# HELP claw_daemon_worker_pool_size"));

    let method_not_allowed = raw_http_request(
        daemon.health_addr,
        "POST",
        "/v1/health/live",
        &[("X-Request-Id", "method-check-1")],
        b"{}",
    )
    .await
    .expect("issue invalid method request");
    assert_eq!(
        method_not_allowed.status_line,
        "HTTP/1.1 405 Method Not Allowed"
    );
    let method_body = json_body(&method_not_allowed);
    assert_eq!(
        method_body.get("code").and_then(Value::as_str),
        Some("method_not_allowed")
    );
    assert_eq!(
        method_body.get("request_id").and_then(Value::as_str),
        Some("method-check-1")
    );

    let not_found = raw_http_request(
        daemon.health_addr,
        "GET",
        "/v1/health/not-real",
        &[("X-Request-Id", "missing-check-1")],
        &[],
    )
    .await
    .expect("issue missing path request");
    assert_eq!(not_found.status_line, "HTTP/1.1 404 Not Found");
    let not_found_body = json_body(&not_found);
    assert_eq!(
        not_found_body.get("code").and_then(Value::as_str),
        Some("not_found")
    );
    assert_eq!(
        not_found_body.get("request_id").and_then(Value::as_str),
        Some("missing-check-1")
    );
}

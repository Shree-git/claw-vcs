use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn workspace_path(relative: &str) -> PathBuf {
    workspace_root().join(relative)
}

fn read_workspace_json(relative: &str) -> Value {
    let path = workspace_path(relative);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse {} as JSON: {err}", path.display()))
}

#[test]
fn daemon_http_openapi_artifact_has_required_paths() {
    let json = read_workspace_json("docs/reference/daemon-http-openapi-v1.json");
    let paths = json
        .get("paths")
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must contain object key: paths");

    for required_path in [
        "/v1/health/live",
        "/v1/health/ready",
        "/v1/health/deps",
        "/v1/metrics",
    ] {
        assert!(
            paths.contains_key(required_path),
            "OpenAPI artifact missing required path: {required_path}"
        );
    }
}

#[test]
fn health_envelope_schema_requires_expected_fields() {
    let json = read_workspace_json("docs/reference/daemon-http-openapi-v1.json");
    let schemas = json
        .get("components")
        .and_then(Value::as_object)
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must contain components.schemas");

    let envelope = schemas
        .get("HealthEnvelope")
        .and_then(Value::as_object)
        .expect("OpenAPI artifact must define components.schemas.HealthEnvelope");

    let required = envelope
        .get("required")
        .and_then(Value::as_array)
        .expect("HealthEnvelope must declare required field list");

    for field in ["code", "message", "request_id", "details"] {
        assert!(
            required.iter().any(|item| item.as_str() == Some(field)),
            "HealthEnvelope required list missing field: {field}"
        );
    }
}

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use claw_core::types::{Capsule, Policy, Revision};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::PolicyContext;
use crate::PolicyError;

const PLUGIN_LIST_ENV: &str = "CLAW_POLICY_PLUGINS";
const PLUGIN_TIMEOUT_ENV: &str = "CLAW_POLICY_PLUGIN_TIMEOUT_MS";
const DEFAULT_TIMEOUT_MS: u64 = 5_000;

const INIT_REQUEST_ID: u64 = 1;
const CHECK_REQUEST_ID: u64 = 2;

pub fn evaluate_plugins(
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    let config = PluginRuntimeConfig::from_env()?;
    if config.executables.is_empty() {
        return Ok(());
    }

    for executable in &config.executables {
        run_plugin(
            executable,
            config.timeout,
            policy,
            revision,
            capsule,
            context,
        )?;
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct PluginRuntimeConfig {
    executables: Vec<PathBuf>,
    timeout: Duration,
}

impl PluginRuntimeConfig {
    fn from_env() -> Result<Self, PolicyError> {
        parse_env_config(
            env::var(PLUGIN_LIST_ENV).ok().as_deref(),
            env::var(PLUGIN_TIMEOUT_ENV).ok().as_deref(),
        )
    }
}

fn parse_env_config(
    plugins_value: Option<&str>,
    timeout_value: Option<&str>,
) -> Result<PluginRuntimeConfig, PolicyError> {
    let executables = plugins_value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect::<Vec<_>>();

    let timeout_ms = match timeout_value {
        Some(raw) => {
            let parsed = raw.trim().parse::<u64>().map_err(|_| {
                PolicyError::PluginConfig(format!(
                    "{} must be a positive integer in milliseconds",
                    PLUGIN_TIMEOUT_ENV
                ))
            })?;
            if parsed == 0 {
                return Err(PolicyError::PluginConfig(format!(
                    "{} must be greater than zero",
                    PLUGIN_TIMEOUT_ENV
                )));
            }
            parsed
        }
        None => DEFAULT_TIMEOUT_MS,
    };

    Ok(PluginRuntimeConfig {
        executables,
        timeout: Duration::from_millis(timeout_ms),
    })
}

fn run_plugin(
    executable: &Path,
    timeout: Duration,
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    let plugin_name = executable.display().to_string();
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| PolicyError::PluginSpawn {
            plugin: plugin_name.clone(),
            reason: err.to_string(),
        })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| PolicyError::PluginProtocol {
            plugin: plugin_name.clone(),
            reason: "child stdin unavailable".to_string(),
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PolicyError::PluginProtocol {
            plugin: plugin_name.clone(),
            reason: "child stdout unavailable".to_string(),
        })?;

    let (line_tx, line_rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };

            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let protocol_result = run_protocol(
        &plugin_name,
        &mut child,
        &mut stdin,
        &line_rx,
        timeout,
        policy,
        revision,
        capsule,
        context,
    );

    let _ = child.kill();
    let _ = child.wait();

    protocol_result
}

#[allow(clippy::too_many_arguments)]
fn run_protocol(
    plugin_name: &str,
    child: &mut Child,
    stdin: &mut ChildStdin,
    line_rx: &Receiver<String>,
    timeout: Duration,
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    let initialize_request = RpcRequest {
        id: INIT_REQUEST_ID,
        method: "initialize",
        params: InitializeParams {
            protocol: "claw-policy-plugin/1",
        },
    };
    send_request(stdin, &initialize_request).map_err(|reason| PolicyError::PluginProtocol {
        plugin: plugin_name.to_string(),
        reason,
    })?;

    let initialize_response = receive_response(plugin_name, child, line_rx, timeout, "initialize")?;
    validate_initialize_response(plugin_name, initialize_response)?;

    let check_request = RpcRequest {
        id: CHECK_REQUEST_ID,
        method: "policy.check",
        params: PolicyCheckParams::from_inputs(policy, revision, capsule, context),
    };
    send_request(stdin, &check_request).map_err(|reason| PolicyError::PluginProtocol {
        plugin: plugin_name.to_string(),
        reason,
    })?;

    let check_response = receive_response(plugin_name, child, line_rx, timeout, "policy.check")?;
    validate_check_response(plugin_name, check_response)
}

fn send_request<T: Serialize>(
    stdin: &mut ChildStdin,
    request: &RpcRequest<T>,
) -> Result<(), String> {
    let mut line = serde_json::to_vec(request).map_err(|err| err.to_string())?;
    line.push(b'\n');
    stdin.write_all(&line).map_err(|err| err.to_string())?;
    stdin.flush().map_err(|err| err.to_string())
}

fn receive_response(
    plugin_name: &str,
    child: &mut Child,
    line_rx: &Receiver<String>,
    timeout: Duration,
    phase: &'static str,
) -> Result<RpcResponse, PolicyError> {
    match line_rx.recv_timeout(timeout) {
        Ok(line) => parse_response_line(&line).map_err(|reason| PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!("{}: {}", phase, reason),
        }),
        Err(RecvTimeoutError::Timeout) => {
            let _ = child.kill();
            Err(PolicyError::PluginTimeout {
                plugin: plugin_name.to_string(),
                phase,
                timeout_ms: timeout.as_millis() as u64,
            })
        }
        Err(RecvTimeoutError::Disconnected) => Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!("{}: plugin stdout closed unexpectedly", phase),
        }),
    }
}

fn parse_response_line(line: &str) -> Result<RpcResponse, String> {
    serde_json::from_str::<RpcResponse>(line)
        .map_err(|err| format!("invalid JSON-RPC response: {}", err))
}

fn validate_initialize_response(
    plugin_name: &str,
    response: RpcResponse,
) -> Result<(), PolicyError> {
    if response.id != INIT_REQUEST_ID {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!("initialize: mismatched response id {}", response.id),
        });
    }
    if let Some(err) = response.error {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!(
                "initialize: plugin returned error{}{}",
                err.code
                    .map(|code| format!(" code={}", code))
                    .unwrap_or_default(),
                if err.message.is_empty() {
                    String::new()
                } else {
                    format!(" message='{}'", err.message)
                }
            ),
        });
    }
    if response.result.is_none() {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: "initialize: missing result".to_string(),
        });
    }

    Ok(())
}

fn validate_check_response(plugin_name: &str, response: RpcResponse) -> Result<(), PolicyError> {
    if response.id != CHECK_REQUEST_ID {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!("policy.check: mismatched response id {}", response.id),
        });
    }
    if let Some(err) = response.error {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!(
                "policy.check: plugin returned error{}{}",
                err.code
                    .map(|code| format!(" code={}", code))
                    .unwrap_or_default(),
                if err.message.is_empty() {
                    String::new()
                } else {
                    format!(" message='{}'", err.message)
                }
            ),
        });
    }

    let Some(result_value) = response.result else {
        return Err(PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: "policy.check: missing result".to_string(),
        });
    };

    let parsed: PluginCheckResult =
        serde_json::from_value(result_value).map_err(|err| PolicyError::PluginProtocol {
            plugin: plugin_name.to_string(),
            reason: format!("policy.check: invalid result payload: {}", err),
        })?;

    if parsed.allow {
        return Ok(());
    }

    Err(PolicyError::PluginDenied {
        plugin: plugin_name.to_string(),
        reason: parsed
            .reason
            .unwrap_or_else(|| "plugin denied policy check".to_string()),
    })
}

#[derive(Debug, Serialize)]
struct RpcRequest<T: Serialize> {
    id: u64,
    method: &'static str,
    params: T,
}

#[derive(Debug, Serialize)]
struct InitializeParams {
    protocol: &'static str,
}

#[derive(Debug, Serialize)]
struct PolicyCheckParams {
    policy_id: String,
    revision: RevisionMetadata,
    context: ContextMetadata,
    evidence: Vec<EvidenceSummary>,
}

impl PolicyCheckParams {
    fn from_inputs(
        policy: &Policy,
        revision: &Revision,
        capsule: &Capsule,
        context: &PolicyContext,
    ) -> Self {
        Self {
            policy_id: policy.policy_id.clone(),
            revision: RevisionMetadata {
                author: revision.author.clone(),
                created_at_ms: revision.created_at_ms,
                summary: revision.summary.clone(),
                has_change_id: revision.change_id.is_some(),
                policy_evidence_count: revision.policy_evidence.len(),
            },
            context: ContextMetadata {
                signer_agent_ids: context.signer_agent_ids.clone(),
                signer_key_ids: context.signer_key_ids.clone(),
                touched_paths: context.touched_paths.clone(),
                trust_score: context.trust_score,
            },
            evidence: capsule
                .public_fields
                .evidence
                .iter()
                .map(|entry| EvidenceSummary {
                    name: entry.name.clone(),
                    status: entry.status.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct RevisionMetadata {
    author: String,
    created_at_ms: u64,
    summary: String,
    has_change_id: bool,
    policy_evidence_count: usize,
}

#[derive(Debug, Serialize)]
struct ContextMetadata {
    signer_agent_ids: Vec<String>,
    signer_key_ids: Vec<String>,
    touched_paths: Vec<String>,
    trust_score: Option<f32>,
}

#[derive(Debug, Serialize)]
struct EvidenceSummary {
    name: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    #[serde(default)]
    code: Option<i64>,
    #[serde(default)]
    message: String,
}

#[derive(Debug, Deserialize)]
struct PluginCheckResult {
    allow: bool,
    #[serde(default)]
    reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_config_extracts_plugins_and_default_timeout() {
        let config = parse_env_config(Some(" /bin/p1 , ,/opt/p2 "), None).unwrap();

        assert_eq!(
            config.executables,
            vec![PathBuf::from("/bin/p1"), PathBuf::from("/opt/p2")]
        );
        assert_eq!(config.timeout, Duration::from_millis(DEFAULT_TIMEOUT_MS));
    }

    #[test]
    fn parse_env_config_rejects_invalid_timeout() {
        let err = parse_env_config(Some("/bin/p1"), Some("nope")).unwrap_err();
        assert!(matches!(err, PolicyError::PluginConfig(_)));
    }

    #[test]
    fn parse_response_line_accepts_valid_json_rpc() {
        let response = parse_response_line(r#"{"id":2,"result":{"allow":true}}"#).unwrap();
        assert_eq!(response.id, CHECK_REQUEST_ID);
        assert!(response.error.is_none());
    }

    #[test]
    fn parse_response_line_rejects_invalid_json() {
        let err = parse_response_line("not-json").unwrap_err();
        assert!(err.contains("invalid JSON-RPC response"));
    }

    #[test]
    fn validate_check_response_rejects_denied_result() {
        let response = RpcResponse {
            id: CHECK_REQUEST_ID,
            result: Some(serde_json::json!({"allow": false, "reason": "denied by plugin"})),
            error: None,
        };

        let err = validate_check_response("/bin/plugin", response).unwrap_err();
        assert!(matches!(err, PolicyError::PluginDenied { .. }));
    }
}

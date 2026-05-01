use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::Args;
use prometheus::{
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    Registry, TextEncoder,
};
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tonic::transport::{Identity, Server, ServerTlsConfig};
use tonic::{Request, Status};

use claw_store::ClawStore;
use claw_sync::capsule_service::CapsuleServer;
use claw_sync::change_service::ChangeServer;
use claw_sync::event_service::EventServer;
use claw_sync::intent_service::IntentServer;
use claw_sync::proto::capsule::capsule_service_server::CapsuleServiceServer;
use claw_sync::proto::change::change_service_server::ChangeServiceServer;
use claw_sync::proto::event::event_stream_service_server::EventStreamServiceServer;
use claw_sync::proto::intent::intent_service_server::IntentServiceServer;
use claw_sync::proto::sync::sync_service_server::SyncServiceServer;
use claw_sync::proto::workstream::workstream_service_server::WorkstreamServiceServer;
use claw_sync::server::{SyncServer, SyncServerOptions};
use claw_sync::workstream_service::WorkstreamServer;

use crate::auth_store;
use crate::commands::RuntimeOptions;
use crate::config::{self, find_repo_root};

#[derive(Args)]
pub struct DaemonArgs {
    /// Listen address
    #[arg(short, long, default_value = "[::1]:50051")]
    listen: String,
    /// HTTP health listen address
    #[arg(long, default_value = "[::1]:50052")]
    health_listen: String,
    /// Use stdio instead of TCP (for embedded use)
    #[arg(long)]
    stdio: bool,
    /// Require bearer auth token for all gRPC requests
    #[arg(long)]
    auth_token: Option<String>,
    /// Read bearer auth token from a saved auth profile
    #[arg(long)]
    auth_profile: Option<String>,
    /// TLS certificate (PEM) path
    #[arg(long)]
    tls_cert: Option<PathBuf>,
    /// TLS private key (PEM) path
    #[arg(long)]
    tls_key: Option<PathBuf>,
}

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct BearerAuthInterceptor {
    expected_header: Arc<str>,
    metrics: Arc<DaemonMetrics>,
}

impl BearerAuthInterceptor {
    fn new(token: String, metrics: Arc<DaemonMetrics>) -> Self {
        Self {
            expected_header: Arc::<str>::from(format!("Bearer {token}")),
            metrics,
        }
    }
}

impl tonic::service::Interceptor for BearerAuthInterceptor {
    fn call(&mut self, request: Request<()>) -> Result<Request<()>, Status> {
        let provided = request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok());

        match provided {
            Some(value) if value == self.expected_header.as_ref() => Ok(request),
            Some(_) => {
                self.metrics
                    .auth_failures
                    .with_label_values(&["invalid"])
                    .inc();
                Err(Status::unauthenticated("invalid bearer token"))
            }
            None => {
                self.metrics
                    .auth_failures
                    .with_label_values(&["missing"])
                    .inc();
                Err(Status::unauthenticated("missing bearer token"))
            }
        }
    }
}

#[derive(Clone)]
struct DaemonMetrics {
    registry: Registry,
    request_latency: HistogramVec,
    auth_failures: IntCounterVec,
    retry_total: IntCounter,
    policy_eval_duration: Histogram,
    queue_depth: IntGauge,
    worker_pool_size: IntGauge,
}

impl DaemonMetrics {
    fn new(worker_pool_size: usize) -> anyhow::Result<Self> {
        let registry = Registry::new();

        let request_latency = HistogramVec::new(
            HistogramOpts::new(
                "claw_daemon_http_request_latency_seconds",
                "HTTP request latency on daemon health listener",
            ),
            &["endpoint"],
        )?;
        let auth_failures = IntCounterVec::new(
            Opts::new(
                "claw_daemon_auth_failures_total",
                "Authentication failures by reason",
            ),
            &["reason"],
        )?;
        let retry_total = IntCounter::new(
            "claw_daemon_retry_total",
            "Retry attempts made by daemon workers",
        )?;
        let policy_eval_duration = Histogram::with_opts(HistogramOpts::new(
            "claw_daemon_policy_eval_duration_seconds",
            "Duration of policy evaluations",
        ))?;
        let queue_depth = IntGauge::new("claw_daemon_queue_depth", "Current daemon queue depth")?;
        let worker_pool_size_gauge = IntGauge::new(
            "claw_daemon_worker_pool_size",
            "Configured daemon worker pool size",
        )?;

        registry.register(Box::new(request_latency.clone()))?;
        registry.register(Box::new(auth_failures.clone()))?;
        registry.register(Box::new(retry_total.clone()))?;
        registry.register(Box::new(policy_eval_duration.clone()))?;
        registry.register(Box::new(queue_depth.clone()))?;
        registry.register(Box::new(worker_pool_size_gauge.clone()))?;

        request_latency.with_label_values(&["health_live"]);
        request_latency.with_label_values(&["health_ready"]);
        request_latency.with_label_values(&["health_deps"]);
        request_latency.with_label_values(&["metrics"]);
        request_latency.with_label_values(&["unknown"]);
        auth_failures.with_label_values(&["missing"]);
        auth_failures.with_label_values(&["invalid"]);

        queue_depth.set(0);
        worker_pool_size_gauge.set(worker_pool_size as i64);

        Ok(Self {
            registry,
            request_latency,
            auth_failures,
            retry_total,
            policy_eval_duration,
            queue_depth,
            worker_pool_size: worker_pool_size_gauge,
        })
    }

    fn endpoint_label(path: &str) -> &'static str {
        match path {
            "/v1/health/live" => "health_live",
            "/v1/health/ready" => "health_ready",
            "/v1/health/deps" => "health_deps",
            "/v1/metrics" => "metrics",
            _ => "unknown",
        }
    }

    fn observe_http_latency(&self, path: &str, start: Instant) {
        let endpoint = Self::endpoint_label(path);
        self.request_latency
            .with_label_values(&[endpoint])
            .observe(start.elapsed().as_secs_f64());
    }

    fn render_prometheus(&self) -> anyhow::Result<Vec<u8>> {
        let metric_families = self.registry.gather();
        let mut out = Vec::new();
        TextEncoder::new().encode(&metric_families, &mut out)?;
        Ok(out)
    }

    fn touch_placeholders(&self) {
        let _ = self.retry_total.get();
        let _ = self.policy_eval_duration.get_sample_count();
        let _ = self.queue_depth.get();
        let _ = self.worker_pool_size.get();
    }
}

fn resolve_daemon_auth_token(args: &DaemonArgs) -> anyhow::Result<Option<String>> {
    if args.auth_token.is_some() && args.auth_profile.is_some() {
        anyhow::bail!("use either --auth-token or --auth-profile, not both");
    }

    if let Some(token) = &args.auth_token {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            anyhow::bail!("--auth-token cannot be empty");
        }
        return Ok(Some(trimmed.to_string()));
    }

    if let Some(profile) = &args.auth_profile {
        let token = auth_store::resolve_access_token(Some(profile)).ok_or_else(|| {
            anyhow::anyhow!(
                "no token for profile '{}'; run `claw auth login --profile {}`",
                profile,
                profile
            )
        })?;
        return Ok(Some(token));
    }

    Ok(None)
}

fn is_local_bind_addr(addr: &SocketAddr) -> bool {
    addr.ip().is_loopback()
}

fn new_request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("req-{nanos:x}-{seq:x}")
}

#[derive(Serialize)]
struct HealthEnvelope<T: Serialize> {
    code: String,
    message: String,
    request_id: String,
    details: T,
}

#[derive(Serialize)]
struct HealthDetails {
    status: String,
}

fn health_response(
    request_id: String,
    message: &str,
    details: HealthDetails,
) -> HealthEnvelope<HealthDetails> {
    HealthEnvelope {
        code: "ok".to_string(),
        message: message.to_string(),
        request_id,
        details,
    }
}

fn health_live(request_id: String) -> HealthEnvelope<HealthDetails> {
    health_response(
        request_id,
        "daemon is live",
        HealthDetails {
            status: "live".to_string(),
        },
    )
}

fn health_ready(request_id: String) -> HealthEnvelope<HealthDetails> {
    health_response(
        request_id,
        "daemon is ready",
        HealthDetails {
            status: "ready".to_string(),
        },
    )
}

fn health_deps(request_id: String) -> HealthEnvelope<HealthDetails> {
    health_response(
        request_id,
        "dependencies are healthy",
        HealthDetails {
            status: "ok".to_string(),
        },
    )
}

fn extract_request_id(raw_request: &str) -> String {
    for line in raw_request.lines().skip(1) {
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("x-request-id") {
                let candidate = value.trim();
                if !candidate.is_empty() {
                    return candidate.to_string();
                }
            }
        }
    }
    new_request_id()
}

async fn write_http_json_response<T: Serialize>(
    stream: &mut tokio::net::TcpStream,
    status_line: &str,
    body: &T,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(body)?;
    let header = format!(
        "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&body).await?;
    Ok(())
}

async fn write_http_text_response(
    stream: &mut tokio::net::TcpStream,
    status_line: &str,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    let header = format!(
        "HTTP/1.1 {status_line}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}

fn log_health_request(request_id: &str, path: &str, status_code: u16) {
    tracing::info!(
        request_id = %request_id,
        path = %path,
        status_code,
        "health_http_request"
    );
}

#[derive(Debug, Deserialize)]
struct BackupManifest {
    files: Vec<BackupEntry>,
}

#[derive(Debug, Deserialize)]
struct BackupEntry {
    path: String,
    bytes: u64,
    sha256: String,
}

fn startup_verify_latest_backup(root: &Path, cfg: &config::ClawConfigV1) -> anyhow::Result<()> {
    if !cfg.backup.verify_integrity_on_startup {
        return Ok(());
    }

    let Some(backup_id) = latest_backup_id(root)? else {
        tracing::info!("startup backup integrity check enabled but no backups found");
        return Ok(());
    };

    match verify_backup_integrity(root, &backup_id) {
        Ok(()) => {
            tracing::info!(backup_id = %backup_id, "startup backup integrity check passed");
            Ok(())
        }
        Err(err) => {
            if cfg.backup.strict_startup_checks {
                anyhow::bail!(
                    "startup backup integrity check failed for {}: {}",
                    backup_id,
                    err
                );
            }
            tracing::warn!(
                backup_id = %backup_id,
                error = %err,
                "startup backup integrity check failed; continuing"
            );
            Ok(())
        }
    }
}

fn latest_backup_id(root: &Path) -> anyhow::Result<Option<String>> {
    let backups_dir = root.join(".claw").join("backups");
    if !backups_dir.is_dir() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(backups_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            candidates.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    candidates.sort();
    Ok(candidates.pop())
}

fn verify_backup_integrity(root: &Path, backup_id: &str) -> anyhow::Result<()> {
    let base = root.join(".claw").join("backups").join(backup_id);
    let manifest_path = base.join("manifest.json");
    let snapshot_root = base.join("snapshot");
    let manifest: BackupManifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)
        .map_err(|err| {
            anyhow::anyhow!("invalid backup manifest {}: {err}", manifest_path.display())
        })?;

    for entry in manifest.files {
        let path = snapshot_root.join(&entry.path);
        let bytes = std::fs::read(&path)
            .map_err(|err| anyhow::anyhow!("backup file missing {}: {err}", path.display()))?;
        if bytes.len() as u64 != entry.bytes {
            anyhow::bail!("backup size mismatch for {}", entry.path);
        }

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let digest = hex::encode(hasher.finalize());
        if digest != entry.sha256 {
            anyhow::bail!("backup checksum mismatch for {}", entry.path);
        }
    }

    Ok(())
}

async fn handle_health_connection(
    mut stream: tokio::net::TcpStream,
    metrics: Arc<DaemonMetrics>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buf[..n]);
    let request_id = extract_request_id(request.as_ref());
    let line = request.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let start = Instant::now();

    if method != "GET" {
        let envelope = HealthEnvelope {
            code: "method_not_allowed".to_string(),
            message: "method not allowed".to_string(),
            request_id: request_id.clone(),
            details: HealthDetails {
                status: "error".to_string(),
            },
        };
        write_http_json_response(&mut stream, "405 Method Not Allowed", &envelope).await?;
        log_health_request(&request_id, path, 405);
        metrics.observe_http_latency(path, start);
        return Ok(());
    }

    match path {
        "/v1/health/live" => {
            write_http_json_response(&mut stream, "200 OK", &health_live(request_id.clone()))
                .await?;
            log_health_request(&request_id, path, 200);
        }
        "/v1/health/ready" => {
            write_http_json_response(&mut stream, "200 OK", &health_ready(request_id.clone()))
                .await?;
            log_health_request(&request_id, path, 200);
        }
        "/v1/health/deps" => {
            write_http_json_response(&mut stream, "200 OK", &health_deps(request_id.clone()))
                .await?;
            log_health_request(&request_id, path, 200);
        }
        "/v1/metrics" => {
            metrics.touch_placeholders();
            let payload = metrics.render_prometheus()?;
            write_http_text_response(
                &mut stream,
                "200 OK",
                "text/plain; version=0.0.4; charset=utf-8",
                &payload,
            )
            .await?;
            log_health_request(&request_id, path, 200);
        }
        _ => {
            let envelope = HealthEnvelope {
                code: "not_found".to_string(),
                message: "not found".to_string(),
                request_id: request_id.clone(),
                details: HealthDetails {
                    status: "error".to_string(),
                },
            };
            write_http_json_response(&mut stream, "404 Not Found", &envelope).await?;
            log_health_request(&request_id, path, 404);
        }
    }
    metrics.observe_http_latency(path, start);

    Ok(())
}

async fn run_health_server(addr: SocketAddr, metrics: Arc<DaemonMetrics>) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let metrics = metrics.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_health_connection(stream, metrics).await {
                tracing::error!(error = %err, "health request failed");
            }
        });
    }
}

fn resolve_tls_identity(
    cert_path: Option<&Path>,
    key_path: Option<&Path>,
) -> anyhow::Result<Option<Identity>> {
    match (cert_path, key_path) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("--tls-cert and --tls-key must be provided together")
        }
        (Some(cert_path), Some(key_path)) => {
            let cert = std::fs::read(cert_path)?;
            let key = std::fs::read(key_path)?;
            Ok(Some(Identity::from_pem(cert, key)))
        }
    }
}

pub async fn run(args: DaemonArgs, runtime: &RuntimeOptions) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let cfg = config::load_or_default_config(&root)?;
    startup_verify_latest_backup(&root, &cfg)?;
    let store = ClawStore::open(&root)?;
    let mut auth_token = resolve_daemon_auth_token(&args)?;

    if args.stdio {
        // Stdio mode: read/write framed messages on stdin/stdout.
        // For embedded agent use. Uses length-prefixed JSON frames.
        eprintln!("Claw daemon running in stdio mode");
        eprintln!("Send JSON-RPC requests on stdin, receive responses on stdout");

        use tokio::io::{AsyncBufReadExt, BufReader};
        let stdin = BufReader::new(tokio::io::stdin());
        let mut lines = stdin.lines();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }
            // Simple echo-based protocol for MVP
            let response = match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(req) => {
                    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
                    match method {
                        "hello" => serde_json::json!({
                            "server_version": "0.1.0",
                            "capabilities": ["partial-clone"]
                        }),
                        "refs" => {
                            let prefix = req.get("prefix").and_then(|p| p.as_str()).unwrap_or("");
                            match store.list_refs(prefix) {
                                Ok(refs) => {
                                    let r: Vec<_> = refs.iter()
                                        .map(|(name, id)| serde_json::json!({"name": name, "target": id.to_string()}))
                                        .collect();
                                    serde_json::json!({"refs": r})
                                }
                                Err(e) => serde_json::json!({"error": e.to_string()}),
                            }
                        }
                        _ => serde_json::json!({"error": format!("unknown method: {method}")}),
                    }
                }
                Err(e) => serde_json::json!({"error": e.to_string()}),
            };
            println!("{}", serde_json::to_string(&response)?);
        }
        return Ok(());
    }

    let addr: SocketAddr = args.listen.parse()?;
    let health_addr: SocketAddr = args.health_listen.parse()?;
    let non_local_bind = !is_local_bind_addr(&addr);

    if auth_token.is_none() {
        let profile = config::default_profile(&cfg);
        auth_token = auth_store::resolve_access_token(Some(profile));
    }

    let tls_cert = args
        .tls_cert
        .clone()
        .or_else(|| cfg.tls.cert_path.as_ref().map(PathBuf::from));
    let tls_key = args
        .tls_key
        .clone()
        .or_else(|| cfg.tls.key_path.as_ref().map(PathBuf::from));
    let tls_identity = resolve_tls_identity(tls_cert.as_deref(), tls_key.as_deref())?;

    let enforce_prod_profile = runtime.profile.eq_ignore_ascii_case("prod");
    if non_local_bind && enforce_prod_profile {
        if cfg.auth.require_auth_for_daemon && auth_token.is_none() {
            anyhow::bail!(
                "non-local bind requires authentication; use --auth-token, --auth-profile, or set a token for default profile"
            );
        }
        if cfg.tls.require_for_non_localhost && tls_identity.is_none() {
            anyhow::bail!(
                "non-local bind requires TLS; provide --tls-cert/--tls-key or set tls.cert_path/tls.key_path in .claw/config.toml"
            );
        }
    }

    let shared_store = Arc::new(RwLock::new(store));
    let metrics = Arc::new(DaemonMetrics::new(cfg.queues.worker_pool_size)?);

    let sync_server = SyncServer::from_shared_with_options(
        shared_store.clone(),
        SyncServerOptions {
            worker_pool_size: cfg.queues.worker_pool_size,
            queue_capacity: cfg.queues.queue_capacity,
            backpressure: cfg.queues.backpressure,
            io_timeout: std::time::Duration::from_millis(cfg.timeouts.io_ms),
        },
    );
    let intent_server = IntentServer::new(shared_store.clone());
    let change_server = ChangeServer::new(shared_store.clone());
    let capsule_server = CapsuleServer::new(shared_store.clone());
    let workstream_server = WorkstreamServer::new(shared_store.clone());
    let event_server = EventServer::new(shared_store);

    let mut grpc_builder = Server::builder();
    if let Some(identity) = tls_identity {
        grpc_builder = grpc_builder.tls_config(ServerTlsConfig::new().identity(identity))?;
    }

    println!("Claw daemon listening (gRPC) on {}", addr);
    println!("Claw daemon listening (health) on {}", health_addr);
    println!("Runtime profile: {}", runtime.profile);
    if auth_token.is_some() {
        println!("gRPC auth enabled (bearer token required)");
    }

    let grpc_metrics = metrics.clone();
    let grpc_task = async move {
        if let Some(token) = auth_token {
            let interceptor = BearerAuthInterceptor::new(token, grpc_metrics.clone());
            grpc_builder
                .add_service(SyncServiceServer::with_interceptor(
                    sync_server,
                    interceptor.clone(),
                ))
                .add_service(IntentServiceServer::with_interceptor(
                    intent_server,
                    interceptor.clone(),
                ))
                .add_service(ChangeServiceServer::with_interceptor(
                    change_server,
                    interceptor.clone(),
                ))
                .add_service(CapsuleServiceServer::with_interceptor(
                    capsule_server,
                    interceptor.clone(),
                ))
                .add_service(WorkstreamServiceServer::with_interceptor(
                    workstream_server,
                    interceptor.clone(),
                ))
                .add_service(EventStreamServiceServer::with_interceptor(
                    event_server,
                    interceptor,
                ))
                .serve(addr)
                .await?;
        } else {
            grpc_builder
                .add_service(SyncServiceServer::new(sync_server))
                .add_service(IntentServiceServer::new(intent_server))
                .add_service(ChangeServiceServer::new(change_server))
                .add_service(CapsuleServiceServer::new(capsule_server))
                .add_service(WorkstreamServiceServer::new(workstream_server))
                .add_service(EventStreamServiceServer::new(event_server))
                .serve(addr)
                .await?;
        }
        Ok::<(), anyhow::Error>(())
    };

    tokio::try_join!(grpc_task, run_health_server(health_addr, metrics))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use tonic::metadata::MetadataValue;
    use tonic::service::Interceptor;

    fn test_metrics() -> Arc<DaemonMetrics> {
        Arc::new(DaemonMetrics::new(8).expect("create test metrics"))
    }

    async fn exercise_health_handler_raw(
        raw_request: &str,
        metrics: Arc<DaemonMetrics>,
    ) -> (String, String, String) {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind local health test listener");
        let addr = listener.local_addr().expect("read local listener address");

        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept test connection");
            handle_health_connection(stream, metrics).await
        });

        let mut client = tokio::net::TcpStream::connect(addr)
            .await
            .expect("connect to local health test listener");
        client
            .write_all(raw_request.as_bytes())
            .await
            .expect("write test request");
        client.shutdown().await.expect("shutdown write half");

        let mut response_bytes = Vec::new();
        client
            .read_to_end(&mut response_bytes)
            .await
            .expect("read full health response");

        server
            .await
            .expect("join health handler task")
            .expect("health handler succeeds");

        let response = String::from_utf8(response_bytes).expect("response is utf-8");
        let (head, body) = response
            .split_once("\r\n\r\n")
            .expect("response contains header/body separator");
        let status_line = head.lines().next().unwrap_or_default().to_string();
        (status_line, head.to_string(), body.to_string())
    }

    async fn exercise_health_handler(raw_request: &str) -> (String, Value) {
        let (status_line, _head, body) =
            exercise_health_handler_raw(raw_request, test_metrics()).await;
        let json = serde_json::from_str::<Value>(&body).expect("response body is valid json");

        (status_line, json)
    }

    #[test]
    fn localhost_bind_is_classified_as_local() {
        let ipv4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 50051);
        let ipv6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 50051);

        assert!(is_local_bind_addr(&ipv4));
        assert!(is_local_bind_addr(&ipv6));
    }

    #[test]
    fn non_local_bind_is_not_classified_as_local() {
        let non_local = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 50051);

        assert!(!is_local_bind_addr(&non_local));
    }

    #[test]
    fn request_id_helper_generates_prefixed_unique_ids() {
        let first = new_request_id();
        let second = new_request_id();

        assert!(first.starts_with("req-"));
        assert!(second.starts_with("req-"));
        assert_ne!(first, second);
    }

    #[test]
    fn extract_request_id_uses_header_when_present() {
        let req =
            "GET /v1/health/live HTTP/1.1\r\nHost: localhost\r\nX-Request-Id: abc-123\r\n\r\n";
        assert_eq!(extract_request_id(req), "abc-123");
    }

    #[test]
    fn extract_request_id_generates_when_missing() {
        let req = "GET /v1/health/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let id = extract_request_id(req);
        assert!(id.starts_with("req-"));
    }

    #[tokio::test]
    async fn health_live_returns_200_with_json_envelope() {
        let request = "GET /v1/health/live HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (status_line, body) = exercise_health_handler(request).await;

        assert_eq!(status_line, "HTTP/1.1 200 OK");
        assert!(body.get("code").is_some());
        assert!(body.get("message").is_some());
        assert!(body.get("request_id").is_some());
        assert!(body.get("details").is_some());
    }

    #[tokio::test]
    async fn health_unknown_path_returns_404_envelope() {
        let request = "GET /v1/health/unknown HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (status_line, body) = exercise_health_handler(request).await;

        assert_eq!(status_line, "HTTP/1.1 404 Not Found");
        assert_eq!(body.get("code").and_then(Value::as_str), Some("not_found"));
        assert!(body.get("request_id").is_some());
    }

    #[tokio::test]
    async fn health_echoes_x_request_id_into_envelope() {
        let request =
            "GET /v1/health/live HTTP/1.1\r\nHost: localhost\r\nX-Request-Id: req-test-123\r\n\r\n";
        let (status_line, body) = exercise_health_handler(request).await;

        assert_eq!(status_line, "HTTP/1.1 200 OK");
        assert_eq!(
            body.get("request_id").and_then(Value::as_str),
            Some("req-test-123")
        );
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_prometheus_text_shape() {
        let request = "GET /v1/metrics HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (status_line, head, body) = exercise_health_handler_raw(request, test_metrics()).await;

        assert_eq!(status_line, "HTTP/1.1 200 OK");
        assert!(head.contains("content-type: text/plain; version=0.0.4; charset=utf-8"));
        assert!(body.contains("# HELP claw_daemon_http_request_latency_seconds"));
        assert!(body.contains("# HELP claw_daemon_auth_failures_total"));
        assert!(body.contains("# HELP claw_daemon_retry_total"));
        assert!(body.contains("# HELP claw_daemon_policy_eval_duration_seconds"));
        assert!(body.contains("# HELP claw_daemon_queue_depth"));
        assert!(body.contains("# HELP claw_daemon_worker_pool_size"));
    }

    #[test]
    fn auth_failure_metrics_increment_for_missing_and_invalid_bearer() {
        let metrics = test_metrics();
        let mut interceptor =
            BearerAuthInterceptor::new("correct-token".to_string(), metrics.clone());

        let missing = interceptor.call(Request::new(()));
        assert!(missing.is_err());

        let mut invalid_req = Request::new(());
        invalid_req.metadata_mut().insert(
            "authorization",
            MetadataValue::try_from("Bearer wrong-token").expect("valid metadata value"),
        );
        let invalid = interceptor.call(invalid_req);
        assert!(invalid.is_err());

        assert_eq!(
            metrics.auth_failures.with_label_values(&["missing"]).get(),
            1
        );
        assert_eq!(
            metrics.auth_failures.with_label_values(&["invalid"]).get(),
            1
        );
    }
}

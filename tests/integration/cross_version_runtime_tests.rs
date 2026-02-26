use claw_sync::client::SyncClient;
use claw_sync::proto::sync::sync_service_server::{SyncService, SyncServiceServer};
use claw_sync::proto::sync::{
    AdvertiseRefsRequest, AdvertiseRefsResponse, FetchObjectsRequest, HelloRequest, HelloResponse,
    ObjectChunk, PushObjectsResponse, UpdateRefsRequest, UpdateRefsResponse,
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

#[derive(Clone)]
struct VersionedHelloService {
    server_version: String,
}

#[tonic::async_trait]
impl SyncService for VersionedHelloService {
    type FetchObjectsStream = ReceiverStream<Result<ObjectChunk, Status>>;

    async fn hello(
        &self,
        _request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        Ok(Response::new(HelloResponse {
            server_version: self.server_version.clone(),
            capabilities: vec!["partial-clone".to_string()],
        }))
    }

    async fn advertise_refs(
        &self,
        _request: Request<AdvertiseRefsRequest>,
    ) -> Result<Response<AdvertiseRefsResponse>, Status> {
        Ok(Response::new(AdvertiseRefsResponse { refs: vec![] }))
    }

    async fn fetch_objects(
        &self,
        _request: Request<FetchObjectsRequest>,
    ) -> Result<Response<Self::FetchObjectsStream>, Status> {
        let (tx, rx) = mpsc::channel(1);
        let _ = tx
            .send(Ok(ObjectChunk {
                id: None,
                object_type: 0,
                data: vec![],
                is_last: true,
            }))
            .await;
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn push_objects(
        &self,
        _request: Request<tonic::Streaming<ObjectChunk>>,
    ) -> Result<Response<PushObjectsResponse>, Status> {
        Ok(Response::new(PushObjectsResponse {
            success: true,
            message: "noop".to_string(),
            accepted: vec![],
        }))
    }

    async fn update_refs(
        &self,
        _request: Request<UpdateRefsRequest>,
    ) -> Result<Response<UpdateRefsResponse>, Status> {
        Ok(Response::new(UpdateRefsResponse {
            success: true,
            message: "noop".to_string(),
        }))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CompatibilityClassification {
    SameVersion,
    OneVersionStepMismatch,
    Unsupported,
}

fn parse_major_minor(version: &str) -> (u64, u64) {
    let mut parts = version.split('.');
    let major = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or_default();
    let minor = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or_default();
    (major, minor)
}

fn classify_runtime_compatibility(
    local_version: &str,
    remote_version: &str,
) -> CompatibilityClassification {
    if local_version == remote_version {
        return CompatibilityClassification::SameVersion;
    }

    let (local_major, local_minor) = parse_major_minor(local_version);
    let (remote_major, remote_minor) = parse_major_minor(remote_version);

    if local_major == remote_major && local_minor.abs_diff(remote_minor) == 1 {
        CompatibilityClassification::OneVersionStepMismatch
    } else {
        CompatibilityClassification::Unsupported
    }
}

struct RunningHelloServer {
    endpoint: String,
    shutdown_tx: oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

async fn spawn_hello_server(server_version: &str) -> RunningHelloServer {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral grpc port");
    let addr = probe.local_addr().expect("read ephemeral grpc port");
    drop(probe);

    let service = VersionedHelloService {
        server_version: server_version.to_string(),
    };

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(SyncServiceServer::new(service))
            .serve_with_shutdown(addr, async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("run grpc hello test server");
    });

    tokio::time::sleep(std::time::Duration::from_millis(25)).await;

    RunningHelloServer {
        endpoint: format!("http://{addr}"),
        shutdown_tx,
        handle,
    }
}

#[tokio::test]
async fn hello_runtime_same_version_is_classified_as_same_version() {
    let server = spawn_hello_server("0.1.0").await;

    let mut client = SyncClient::connect(&server.endpoint)
        .await
        .expect("connect runtime gRPC client");
    let hello = client.hello().await.expect("invoke hello over gRPC runtime");

    let classification = classify_runtime_compatibility("0.1.0", &hello.server_version);
    assert_eq!(classification, CompatibilityClassification::SameVersion);

    let _ = server.shutdown_tx.send(());
    server.handle.await.expect("join grpc server task");
}

#[tokio::test]
async fn hello_runtime_one_step_mismatch_is_classified_as_limited_compatibility() {
    let server = spawn_hello_server("0.2.0").await;

    let mut client = SyncClient::connect(&server.endpoint)
        .await
        .expect("connect runtime gRPC client");
    let hello = client.hello().await.expect("invoke hello over gRPC runtime");

    let classification = classify_runtime_compatibility("0.1.0", &hello.server_version);
    assert_eq!(
        classification,
        CompatibilityClassification::OneVersionStepMismatch
    );

    let _ = server.shutdown_tx.send(());
    server.handle.await.expect("join grpc server task");
}

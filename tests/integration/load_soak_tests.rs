use claw_sync::proto::sync::sync_service_client::SyncServiceClient;
use claw_sync::proto::sync::sync_service_server::{SyncService, SyncServiceServer};
use claw_sync::proto::sync::{
    AdvertiseRefsRequest, AdvertiseRefsResponse, FetchObjectsRequest, HelloRequest, HelloResponse,
    ObjectChunk, PushObjectsResponse, UpdateRefsRequest, UpdateRefsResponse,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

struct LoadState {
    accepted: AtomicUsize,
    rejected: AtomicUsize,
    inflight: AtomicUsize,
    max_inflight: AtomicUsize,
}

impl LoadState {
    fn new() -> Self {
        Self {
            accepted: AtomicUsize::new(0),
            rejected: AtomicUsize::new(0),
            inflight: AtomicUsize::new(0),
            max_inflight: AtomicUsize::new(0),
        }
    }

    fn record_enter(&self) {
        let now = self.inflight.fetch_add(1, Ordering::SeqCst) + 1;
        let mut max = self.max_inflight.load(Ordering::SeqCst);
        while now > max {
            match self
                .max_inflight
                .compare_exchange(max, now, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => break,
                Err(observed) => max = observed,
            }
        }
    }

    fn record_exit(&self) {
        self.inflight.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct OverloadAwareHelloService {
    permits: Arc<Semaphore>,
    latency: Duration,
    state: Arc<LoadState>,
}

#[tonic::async_trait]
impl SyncService for OverloadAwareHelloService {
    type FetchObjectsStream = ReceiverStream<Result<ObjectChunk, Status>>;

    async fn hello(
        &self,
        _request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let Ok(permit) = self.permits.clone().try_acquire_owned() else {
            self.state.rejected.fetch_add(1, Ordering::SeqCst);
            return Err(Status::resource_exhausted("server overloaded"));
        };

        self.state.record_enter();
        tokio::time::sleep(self.latency).await;
        self.state.record_exit();
        drop(permit);

        self.state.accepted.fetch_add(1, Ordering::SeqCst);
        Ok(Response::new(HelloResponse {
            server_version: "0.1.0".to_string(),
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

struct RunningLoadServer {
    endpoint: String,
    state: Arc<LoadState>,
    shutdown_tx: oneshot::Sender<()>,
    handle: tokio::task::JoinHandle<()>,
}

async fn spawn_load_server(max_inflight: usize, latency: Duration) -> RunningLoadServer {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral grpc port");
    let addr = probe.local_addr().expect("read ephemeral grpc port");
    drop(probe);

    let state = Arc::new(LoadState::new());
    let service = OverloadAwareHelloService {
        permits: Arc::new(Semaphore::new(max_inflight)),
        latency,
        state: state.clone(),
    };

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(SyncServiceServer::new(service))
            .serve_with_shutdown(addr, async {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("run grpc load server");
    });

    tokio::time::sleep(Duration::from_millis(25)).await;

    RunningLoadServer {
        endpoint: format!("http://{addr}"),
        state,
        shutdown_tx,
        handle,
    }
}

async fn hello_once(endpoint: &str) -> Result<(), tonic::Code> {
    let mut client = SyncServiceClient::connect(endpoint.to_string())
        .await
        .map_err(|_| tonic::Code::Unavailable)?;
    client
        .hello(Request::new(HelloRequest {
            client_version: "0.1.0".to_string(),
            capabilities: vec!["partial-clone".to_string()],
        }))
        .await
        .map(|_| ())
        .map_err(|err| err.code())
}

#[tokio::test]
async fn sustained_load_is_deterministic_and_bounded_without_overload() {
    let concurrency = 4usize;
    let rounds = 20usize;
    let server = spawn_load_server(concurrency, Duration::from_millis(5)).await;

    for _ in 0..rounds {
        let mut tasks = Vec::with_capacity(concurrency);
        for _ in 0..concurrency {
            let endpoint = server.endpoint.clone();
            tasks.push(tokio::spawn(async move { hello_once(&endpoint).await }));
        }

        for task in tasks {
            let result = task.await.expect("join sustained hello task");
            assert!(result.is_ok(), "sustained request should not overload");
        }
    }

    let total = concurrency * rounds;
    assert_eq!(server.state.accepted.load(Ordering::SeqCst), total);
    assert_eq!(server.state.rejected.load(Ordering::SeqCst), 0);
    assert!(
        server.state.max_inflight.load(Ordering::SeqCst) <= concurrency,
        "inflight must stay bounded by configured concurrency"
    );

    let _ = server.shutdown_tx.send(());
    server.handle.await.expect("join grpc load server");
}

#[tokio::test]
async fn burst_load_uses_explicit_overload_handling_and_remains_bounded() {
    let max_inflight = 3usize;
    let burst = 48usize;
    let server = spawn_load_server(max_inflight, Duration::from_millis(20)).await;
    let barrier = Arc::new(tokio::sync::Barrier::new(burst));

    let mut tasks = Vec::with_capacity(burst);
    for _ in 0..burst {
        let endpoint = server.endpoint.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            hello_once(&endpoint).await
        }));
    }

    let mut ok = 0usize;
    let mut overloaded = 0usize;
    for task in tasks {
        match task.await.expect("join burst hello task") {
            Ok(()) => ok += 1,
            Err(tonic::Code::ResourceExhausted) => overloaded += 1,
            Err(code) => panic!("unexpected burst error code: {code:?}"),
        }
    }

    assert_eq!(ok + overloaded, burst);
    assert!(
        overloaded > 0,
        "burst must trigger explicit overload responses"
    );
    assert_eq!(server.state.accepted.load(Ordering::SeqCst), ok);
    assert_eq!(server.state.rejected.load(Ordering::SeqCst), overloaded);
    assert!(
        server.state.max_inflight.load(Ordering::SeqCst) <= max_inflight,
        "inflight must stay bounded under burst pressure"
    );

    let _ = server.shutdown_tx.send(());
    server.handle.await.expect("join grpc load server");
}

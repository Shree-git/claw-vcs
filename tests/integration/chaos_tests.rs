use claw_core::cof::{cof_decode, cof_encode};
use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, FileMode, Revision, Tree, TreeEntry};
use claw_store::loose::loose_object_path;
use claw_store::ClawStore;
use claw_sync::proto::sync::sync_service_server::SyncService;
use claw_sync::proto::sync::{
    AdvertiseRefsRequest, AdvertiseRefsResponse, FetchObjectsRequest, HelloRequest, HelloResponse,
    ObjectChunk, PushObjectsResponse, UpdateRefsRequest, UpdateRefsResponse,
};
use claw_sync::server::SyncServer;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

#[derive(Clone, Copy)]
enum ChaosMode {
    InterruptDuringFetch,
    LatencySpike { delay: Duration },
}

struct ChaosSyncService {
    mode: ChaosMode,
}

impl ChaosSyncService {
    fn new(mode: ChaosMode) -> Self {
        Self { mode }
    }

    fn blob_chunk(bytes: &[u8]) -> ObjectChunk {
        let blob = Object::Blob(Blob {
            data: bytes.to_vec(),
            media_type: None,
        });
        let payload = blob.serialize_payload().expect("serialize blob payload");
        let cof = cof_encode(blob.type_tag(), &payload).expect("encode blob COF");

        ObjectChunk {
            id: Some(claw_sync::proto::common::ObjectId { hash: vec![7; 32] }),
            object_type: blob.type_tag() as i32,
            data: cof,
            is_last: false,
        }
    }
}

#[tonic::async_trait]
impl SyncService for ChaosSyncService {
    type FetchObjectsStream = ReceiverStream<Result<ObjectChunk, Status>>;

    async fn hello(
        &self,
        _request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        Err(Status::unimplemented("not used in chaos tests"))
    }

    async fn advertise_refs(
        &self,
        _request: Request<AdvertiseRefsRequest>,
    ) -> Result<Response<AdvertiseRefsResponse>, Status> {
        Err(Status::unimplemented("not used in chaos tests"))
    }

    async fn fetch_objects(
        &self,
        _request: Request<FetchObjectsRequest>,
    ) -> Result<Response<Self::FetchObjectsStream>, Status> {
        let (tx, rx) = mpsc::channel(8);

        match self.mode {
            ChaosMode::InterruptDuringFetch => {
                tokio::spawn(async move {
                    let _ = tx.send(Ok(Self::blob_chunk(b"first chunk"))).await;
                    let _ = tx
                        .send(Err(Status::unavailable(
                            "daemon interrupted during fetch",
                        )))
                        .await;
                });
            }
            ChaosMode::LatencySpike { delay } => {
                tokio::spawn(async move {
                    tokio::time::sleep(delay).await;
                    let _ = tx.send(Ok(Self::blob_chunk(b"delayed chunk"))).await;
                    let _ = tx
                        .send(Ok(ObjectChunk {
                            id: None,
                            object_type: 0,
                            data: vec![],
                            is_last: true,
                        }))
                        .await;
                });
            }
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn push_objects(
        &self,
        _request: Request<tonic::Streaming<ObjectChunk>>,
    ) -> Result<Response<PushObjectsResponse>, Status> {
        Err(Status::unimplemented("not used in chaos tests"))
    }

    async fn update_refs(
        &self,
        _request: Request<UpdateRefsRequest>,
    ) -> Result<Response<UpdateRefsResponse>, Status> {
        Err(Status::unimplemented("not used in chaos tests"))
    }
}

async fn fetch_and_store<S>(service: &S, store: &ClawStore) -> Result<Vec<ObjectId>, Status>
where
    S: SyncService,
    S::FetchObjectsStream: Unpin,
{
    let response = service
        .fetch_objects(Request::new(FetchObjectsRequest {
            want: vec![],
            have: vec![],
            filter: None,
        }))
        .await?;

    let mut stream = response.into_inner();
    let mut fetched = Vec::new();
    while let Some(item) = stream.next().await {
        let chunk = item?;
        if chunk.is_last {
            break;
        }

        let (type_tag, payload) = cof_decode(&chunk.data)
            .map_err(|err| Status::internal(format!("decode failed: {err}")))?;
        let obj = Object::deserialize_payload(type_tag, &payload)
            .map_err(|err| Status::internal(format!("deserialize failed: {err}")))?;
        let id = store
            .store_object(&obj)
            .map_err(|err| Status::internal(format!("store failed: {err}")))?;
        fetched.push(id);
    }

    Ok(fetched)
}

async fn collect_fetch_ids(
    response: Response<<SyncServer as SyncService>::FetchObjectsStream>,
) -> (Vec<ObjectId>, bool) {
    let mut stream = response.into_inner();
    let mut ids = Vec::new();
    let mut saw_last = false;

    while let Some(item) = stream.next().await {
        let chunk = item.expect("stream item should not fail");
        if chunk.is_last {
            saw_last = true;
            break;
        }

        let id = chunk.id.expect("chunk id");
        let bytes: [u8; 32] = id.hash.as_slice().try_into().expect("object id bytes");
        ids.push(ObjectId::from_bytes(bytes));
    }

    (ids, saw_last)
}

#[tokio::test]
async fn daemon_interruption_during_fetch_is_reported_deterministically() {
    let tmp = tempfile::tempdir().expect("temp repo");
    let local_store = ClawStore::init(tmp.path()).expect("init local store");
    let service = ChaosSyncService::new(ChaosMode::InterruptDuringFetch);

    let result = fetch_and_store(&service, &local_store).await;
    assert!(result.is_err(), "fetch should fail after daemon interruption");

    let err = result.expect_err("fetch should return interruption error");
    assert_eq!(err.code(), tonic::Code::Unavailable);
    assert!(
        err.message().contains("interrupted"),
        "error should mention interruption"
    );

    let stored = local_store.list_object_ids().expect("list stored objects");
    assert_eq!(stored.len(), 1, "exactly one chunk should persist before error");
}

#[tokio::test]
async fn latency_spike_path_times_out_with_bounded_runtime() {
    let tmp = tempfile::tempdir().expect("temp repo");
    let local_store = ClawStore::init(tmp.path()).expect("init local store");
    let service = ChaosSyncService::new(ChaosMode::LatencySpike {
        delay: Duration::from_millis(200),
    });

    let timed = tokio::time::timeout(Duration::from_millis(50), fetch_and_store(&service, &local_store)).await;

    assert!(timed.is_err(), "fetch should hit timeout on latency spike");

    let stored = local_store.list_object_ids().expect("list stored objects");
    assert!(
        stored.is_empty(),
        "no object should be stored when timeout fires before first chunk"
    );
}

#[tokio::test]
async fn missing_object_during_traversal_is_skipped_and_stream_completes() {
    let tmp = tempfile::tempdir().expect("temp repo");
    let store = ClawStore::init(tmp.path()).expect("init store");

    let blob_id = store
        .store_object(&Object::Blob(Blob {
            data: b"payload".to_vec(),
            media_type: None,
        }))
        .expect("store blob");

    let tree_id = store
        .store_object(&Object::Tree(Tree {
            entries: vec![TreeEntry {
                name: "file.txt".to_string(),
                mode: FileMode::Regular,
                object_id: blob_id,
            }],
        }))
        .expect("store tree");

    let revision_id = store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: Some(tree_id),
            capsule_id: None,
            author: "chaos-test".to_string(),
            created_at_ms: 1,
            summary: "missing object traversal".to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision");

    let blob_path = loose_object_path(store.layout(), &blob_id);
    assert!(blob_path.exists(), "blob path should exist before deletion");
    std::fs::remove_file(&blob_path).expect("remove blob object");

    let server = SyncServer::new(store);
    let response = server
        .fetch_objects(Request::new(FetchObjectsRequest {
            want: vec![claw_sync::proto::common::ObjectId {
                hash: revision_id.as_bytes().to_vec(),
            }],
            have: vec![],
            filter: None,
        }))
        .await
        .expect("fetch should succeed despite missing dependency");

    let (ids, saw_last) = collect_fetch_ids(response).await;
    assert!(saw_last, "stream should finish with terminal marker");
    assert!(ids.contains(&revision_id), "revision should still be streamed");
    assert!(ids.contains(&tree_id), "available dependency should be streamed");
    assert!(
        !ids.contains(&blob_id),
        "missing blob should be skipped gracefully"
    );
}

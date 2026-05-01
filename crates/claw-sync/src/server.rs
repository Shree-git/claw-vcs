use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use tonic::{Request, Response, Status};

use claw_core::cof::cof_encode;
use claw_core::id::ObjectId;
use claw_store::ClawStore;

use crate::ancestry::is_ancestor;
use crate::negotiation::{find_reachable_objects, find_reachable_objects_with_depth};
use crate::partial_clone::{CapsuleVisibilityFilter, PartialCloneFilter};
use crate::proto::sync::sync_service_server::SyncService;
use crate::proto::sync::*;

pub struct SyncServer {
    store: Arc<RwLock<ClawStore>>,
    limits: Arc<ServerLimits>,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncServerOptions {
    pub worker_pool_size: usize,
    pub queue_capacity: usize,
    pub backpressure: bool,
    pub io_timeout: Duration,
}

impl Default for SyncServerOptions {
    fn default() -> Self {
        Self {
            worker_pool_size: 8,
            queue_capacity: 1_024,
            backpressure: true,
            io_timeout: Duration::from_secs(10),
        }
    }
}

struct ServerLimits {
    queue: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    options: SyncServerOptions,
}

impl SyncServer {
    pub fn new(store: ClawStore) -> Self {
        Self::new_with_options(store, SyncServerOptions::default())
    }

    pub fn from_shared(store: Arc<RwLock<ClawStore>>) -> Self {
        Self::from_shared_with_options(store, SyncServerOptions::default())
    }

    pub fn new_with_options(store: ClawStore, options: SyncServerOptions) -> Self {
        Self::from_shared_with_options(Arc::new(RwLock::new(store)), options)
    }

    pub fn from_shared_with_options(
        store: Arc<RwLock<ClawStore>>,
        options: SyncServerOptions,
    ) -> Self {
        let worker_pool_size = options.worker_pool_size.max(1);
        let total_capacity = worker_pool_size
            .saturating_add(options.queue_capacity)
            .max(1);

        Self {
            store,
            limits: Arc::new(ServerLimits {
                queue: Arc::new(Semaphore::new(total_capacity)),
                workers: Arc::new(Semaphore::new(worker_pool_size)),
                options,
            }),
        }
    }

    #[allow(clippy::result_large_err)]
    fn acquire_queue_permit(&self) -> Result<Option<OwnedSemaphorePermit>, Status> {
        if !self.limits.options.backpressure {
            return Ok(None);
        }

        self.limits
            .queue
            .clone()
            .try_acquire_owned()
            .map(Some)
            .map_err(|_| Status::resource_exhausted("server overloaded: queue is full"))
    }

    async fn run_bounded<F, T>(&self, operation: &'static str, task: F) -> Result<T, Status>
    where
        F: std::future::Future<Output = Result<T, Status>>,
    {
        let _queue_permit = self.acquire_queue_permit()?;
        let worker = self.limits.workers.clone();
        let timeout = self.limits.options.io_timeout;

        let operation_future = async move {
            let _worker_permit = worker
                .acquire_owned()
                .await
                .map_err(|_| Status::unavailable("server workers are unavailable"))?;
            task.await
        };

        tokio::time::timeout(timeout, operation_future)
            .await
            .map_err(|_| Status::deadline_exceeded(format!("{operation} timed out")))?
    }
}

/// Convert a proto PartialCloneFilter to our internal filter type.
fn convert_filter(filter: &crate::proto::sync::PartialCloneFilter) -> PartialCloneFilter {
    PartialCloneFilter {
        intent_ids: filter
            .intent_ids
            .iter()
            .map(|id| id.to_ascii_uppercase())
            .collect(),
        path_prefixes: filter.path_prefixes.clone(),
        codec_ids: filter.codec_ids.clone(),
        time_range: if filter.time_range_start > 0 || filter.time_range_end > 0 {
            Some((filter.time_range_start, filter.time_range_end))
        } else {
            None
        },
        capsule_visibility: CapsuleVisibilityFilter::from_proto_value(&filter.capsule_visibility),
        max_depth: if filter.max_depth > 0 {
            Some(filter.max_depth)
        } else {
            None
        },
        max_bytes: if filter.max_bytes > 0 {
            Some(filter.max_bytes)
        } else {
            None
        },
    }
}

#[allow(clippy::result_large_err)]
fn decode_object_id(msg: &crate::proto::common::ObjectId) -> Result<ObjectId, Status> {
    let id_bytes: [u8; 32] = msg
        .hash
        .as_slice()
        .try_into()
        .map_err(|_| Status::invalid_argument("invalid object id"))?;
    Ok(ObjectId::from_bytes(id_bytes))
}

#[tonic::async_trait]
impl SyncService for SyncServer {
    async fn hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        self.run_bounded("hello", async move {
            let _req = request.into_inner();
            Ok(Response::new(HelloResponse {
                server_version: "0.1.0".to_string(),
                capabilities: vec!["partial-clone".to_string()],
            }))
        })
        .await
    }

    async fn advertise_refs(
        &self,
        request: Request<AdvertiseRefsRequest>,
    ) -> Result<Response<AdvertiseRefsResponse>, Status> {
        self.run_bounded("advertise_refs", async move {
            let req = request.into_inner();
            let store = self.store.read().await;
            let refs = store
                .list_refs(&req.prefix)
                .map_err(|e| Status::internal(e.to_string()))?;

            let entries = refs
                .into_iter()
                .map(|(name, id)| RefEntry {
                    name,
                    target: Some(crate::proto::common::ObjectId {
                        hash: id.as_bytes().to_vec(),
                    }),
                })
                .collect();

            Ok(Response::new(AdvertiseRefsResponse { refs: entries }))
        })
        .await
    }

    type FetchObjectsStream = tokio_stream::wrappers::ReceiverStream<Result<ObjectChunk, Status>>;

    async fn fetch_objects(
        &self,
        request: Request<FetchObjectsRequest>,
    ) -> Result<Response<Self::FetchObjectsStream>, Status> {
        let req = request.into_inner();
        let store = self.store.clone();
        let filter = req.filter.as_ref().map(convert_filter);
        let timeout = self.limits.options.io_timeout;
        let queue_permit = self.acquire_queue_permit()?;
        let worker_permit = self
            .limits
            .workers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Status::unavailable("server workers are unavailable"))?;

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            let _queue_permit = queue_permit;
            let _worker_permit = worker_permit;

            let run_result = tokio::time::timeout(timeout, async {
                let store = store.read().await;

                // Compute want_set = reachable from want_ids
                let want_ids: Vec<ObjectId> = req
                    .want
                    .iter()
                    .filter_map(|msg| {
                        let bytes: [u8; 32] = msg.hash.as_slice().try_into().ok()?;
                        Some(ObjectId::from_bytes(bytes))
                    })
                    .collect();

                let have_ids: Vec<ObjectId> = req
                    .have
                    .iter()
                    .filter_map(|msg| {
                        let bytes: [u8; 32] = msg.hash.as_slice().try_into().ok()?;
                        Some(ObjectId::from_bytes(bytes))
                    })
                    .collect();

                let want_set = find_reachable_objects_with_depth(
                    &store,
                    &want_ids,
                    filter.as_ref().and_then(|f| f.max_depth),
                );
                let have_set = find_reachable_objects(&store, &have_ids);

                let candidate_set: std::collections::HashSet<ObjectId> =
                    want_set.difference(&have_set).copied().collect();

                // Apply filter only to root candidates, then include all in-set dependencies
                // for the selected roots to keep the streamed graph self-contained.
                let mut root_ids: Vec<ObjectId> = candidate_set
                    .iter()
                    .copied()
                    .filter(|id| filter.as_ref().is_none_or(|f| f.matches_object(&store, id)))
                    .collect();
                root_ids.sort_by_key(|id| id.to_hex());

                let mut selected_ids = std::collections::HashSet::new();
                let mut stack = root_ids;
                while let Some(id) = stack.pop() {
                    if !selected_ids.insert(id) {
                        continue;
                    }

                    let Ok(obj) = store.load_object(&id) else {
                        continue;
                    };
                    for dep in obj.dependencies() {
                        if candidate_set.contains(&dep) && !selected_ids.contains(&dep) {
                            stack.push(dep);
                        }
                    }
                }

                let mut candidate_ids: Vec<ObjectId> = selected_ids.into_iter().collect();
                candidate_ids.sort_by_key(|id| id.to_hex());
                let mut sent_bytes: u64 = 0;

                // Send candidates in deterministic order.
                for id in candidate_ids {
                    if let Ok(obj) = store.load_object(&id) {
                        let payload = obj.serialize_payload().unwrap_or_default();
                        let type_tag = obj.type_tag();
                        let cof_data = cof_encode(type_tag, &payload).unwrap_or_default();

                        if let Some(limit) = filter.as_ref().and_then(|f| f.max_bytes) {
                            let chunk_len = cof_data.len() as u64;
                            if sent_bytes.saturating_add(chunk_len) > limit {
                                break;
                            }
                            sent_bytes += chunk_len;
                        }

                        let chunk = ObjectChunk {
                            id: Some(crate::proto::common::ObjectId {
                                hash: id.as_bytes().to_vec(),
                            }),
                            object_type: type_tag as i32,
                            data: cof_data,
                            is_last: false,
                        };
                        if tx.send(Ok(chunk)).await.is_err() {
                            return;
                        }
                    }
                }

                let _ = tx
                    .send(Ok(ObjectChunk {
                        id: None,
                        object_type: 0,
                        data: vec![],
                        is_last: true,
                    }))
                    .await;
            })
            .await;

            if run_result.is_err() {
                let _ = tx
                    .send(Err(Status::deadline_exceeded("fetch_objects timed out")))
                    .await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn push_objects(
        &self,
        request: Request<tonic::Streaming<ObjectChunk>>,
    ) -> Result<Response<PushObjectsResponse>, Status> {
        self.run_bounded("push_objects", async move {
            let mut stream = request.into_inner();
            let store = self.store.write().await;
            let mut accepted = Vec::new();

            while let Some(chunk) = stream.message().await? {
                if chunk.is_last {
                    break;
                }

                if let Some(_id_msg) = &chunk.id {
                    let (type_tag, payload) = claw_core::cof::cof_decode(&chunk.data)
                        .map_err(|e| Status::internal(e.to_string()))?;
                    let obj = claw_core::object::Object::deserialize_payload(type_tag, &payload)
                        .map_err(|e| Status::internal(e.to_string()))?;
                    let id = store
                        .store_object(&obj)
                        .map_err(|e| Status::internal(e.to_string()))?;
                    accepted.push(crate::proto::common::ObjectId {
                        hash: id.as_bytes().to_vec(),
                    });
                }
            }

            Ok(Response::new(PushObjectsResponse {
                success: true,
                message: format!("accepted {} objects", accepted.len()),
                accepted,
            }))
        })
        .await
    }

    async fn update_refs(
        &self,
        request: Request<UpdateRefsRequest>,
    ) -> Result<Response<UpdateRefsResponse>, Status> {
        self.run_bounded("update_refs", async move {
            let req = request.into_inner();
            let store = self.store.write().await;

            // Two-pass: first verify all CAS conditions, then apply
            // Pass 1: verify
            for update in &req.updates {
                let current = store
                    .get_ref(&update.name)
                    .map_err(|e| Status::internal(e.to_string()))?;

                let expected_old = update
                    .old_target
                    .as_ref()
                    .map(decode_object_id)
                    .transpose()?;

                match (&expected_old, &current) {
                    (None, None) => {} // Creating new ref
                    (Some(expected), Some(actual)) if expected == actual => {}
                    (None, Some(_)) if update.force => {} // Force override existing ref
                    _ => {
                        return Ok(Response::new(UpdateRefsResponse {
                            success: false,
                            message: format!(
                                "CAS conflict on ref '{}': expected {:?}, actual {:?}",
                                update.name,
                                expected_old.map(|id| id.to_hex()),
                                current.map(|id| id.to_hex()),
                            ),
                        }));
                    }
                }

                // FF check: verify new is descendant of old (unless force)
                if let Some(new_target) = &update.new_target {
                    let new_id = decode_object_id(new_target)?;
                    if let Some(ref old_id) = current {
                        if !update.force && !is_ancestor(&store, old_id, &new_id) {
                            return Ok(Response::new(UpdateRefsResponse {
                                success: false,
                                message: format!(
                                    "non-fast-forward update on ref '{}'; use force to override",
                                    update.name
                                ),
                            }));
                        }
                    }
                }
            }

            // Pass 2: apply all updates
            for update in &req.updates {
                if let Some(new_target) = &update.new_target {
                    let id = decode_object_id(new_target)?;
                    store
                        .set_ref(&update.name, &id)
                        .map_err(|e| Status::internal(e.to_string()))?;
                }
            }

            Ok(Response::new(UpdateRefsResponse {
                success: true,
                message: "refs updated".to_string(),
            }))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::id::{ChangeId, IntentId};
    use claw_core::object::Object;
    use claw_core::types::{Blob, Capsule, CapsulePublic, Change, ChangeStatus, Patch, Revision};
    use std::time::Duration;
    use tokio_stream::StreamExt;

    async fn collect_streamed_ids(
        response: Response<<SyncServer as SyncService>::FetchObjectsStream>,
    ) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        let mut stream = response.into_inner();
        while let Some(item) = stream.next().await {
            let chunk = item.unwrap();
            if chunk.is_last {
                break;
            }

            let id = chunk.id.expect("chunk id");
            let bytes: [u8; 32] = id.hash.as_slice().try_into().unwrap();
            ids.push(ObjectId::from_bytes(bytes));
        }
        ids
    }

    #[test]
    fn convert_filter_maps_new_fields() {
        let proto = crate::proto::sync::PartialCloneFilter {
            intent_ids: vec!["01aryz6s41tsv4rrffq69g5fav".to_string()],
            path_prefixes: vec!["src/".to_string()],
            time_range_start: 100,
            time_range_end: 200,
            codec_ids: vec!["text/line".to_string()],
            capsule_visibility: "PRIVATE".to_string(),
            max_bytes: 1234,
            max_depth: 4,
        };

        let converted = convert_filter(&proto);
        assert_eq!(converted.intent_ids, vec!["01ARYZ6S41TSV4RRFFQ69G5FAV"]);
        assert_eq!(converted.path_prefixes, vec!["src/"]);
        assert_eq!(converted.codec_ids, vec!["text/line"]);
        assert_eq!(converted.time_range, Some((100, 200)));
        assert_eq!(
            converted.capsule_visibility,
            Some(CapsuleVisibilityFilter::Private)
        );
        assert_eq!(converted.max_bytes, Some(1234));
        assert_eq!(converted.max_depth, Some(4));
    }

    #[tokio::test]
    async fn fetch_objects_enforces_max_depth() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();

        let rev0 = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 1,
                summary: "r0".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let rev1 = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![rev0],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 2,
                summary: "r1".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let rev2 = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![rev1],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 3,
                summary: "r2".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let server = SyncServer::new(store);
        let response = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: vec![crate::proto::common::ObjectId {
                    hash: rev2.as_bytes().to_vec(),
                }],
                have: vec![],
                filter: Some(crate::proto::sync::PartialCloneFilter {
                    intent_ids: vec![],
                    path_prefixes: vec![],
                    time_range_start: 0,
                    time_range_end: 0,
                    codec_ids: vec![],
                    capsule_visibility: String::new(),
                    max_bytes: 0,
                    max_depth: 1,
                }),
            }))
            .await
            .unwrap();

        let ids = collect_streamed_ids(response).await;
        assert!(ids.contains(&rev2));
        assert!(ids.contains(&rev1));
        assert!(!ids.contains(&rev0));
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn fetch_objects_enforces_intent_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();

        let intent_keep = IntentId::new();
        let intent_drop = IntentId::new();

        let change_keep = Change {
            id: ChangeId::new(),
            intent_id: intent_keep,
            head_revision: None,
            workstream_id: None,
            status: ChangeStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let change_drop = Change {
            id: ChangeId::new(),
            intent_id: intent_drop,
            head_revision: None,
            workstream_id: None,
            status: ChangeStatus::Open,
            created_at_ms: 1,
            updated_at_ms: 1,
        };

        let change_keep_oid = store
            .store_object(&Object::Change(change_keep.clone()))
            .unwrap();
        let change_drop_oid = store
            .store_object(&Object::Change(change_drop.clone()))
            .unwrap();

        store
            .set_ref(&format!("changes/{}", change_keep.id), &change_keep_oid)
            .unwrap();
        store
            .set_ref(&format!("changes/{}", change_drop.id), &change_drop_oid)
            .unwrap();

        let rev_keep = store
            .store_object(&Object::Revision(Revision {
                change_id: Some(change_keep.id),
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 10,
                summary: "keep".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let rev_drop = store
            .store_object(&Object::Revision(Revision {
                change_id: Some(change_drop.id),
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 11,
                summary: "drop".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let server = SyncServer::new(store);
        let response = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: vec![
                    crate::proto::common::ObjectId {
                        hash: rev_keep.as_bytes().to_vec(),
                    },
                    crate::proto::common::ObjectId {
                        hash: rev_drop.as_bytes().to_vec(),
                    },
                ],
                have: vec![],
                filter: Some(crate::proto::sync::PartialCloneFilter {
                    intent_ids: vec![intent_keep.to_string().to_ascii_lowercase()],
                    path_prefixes: vec![],
                    time_range_start: 0,
                    time_range_end: 0,
                    codec_ids: vec![],
                    capsule_visibility: String::new(),
                    max_bytes: 0,
                    max_depth: 0,
                }),
            }))
            .await
            .unwrap();

        let ids = collect_streamed_ids(response).await;
        assert!(ids.contains(&rev_keep));
        assert!(!ids.contains(&rev_drop));
    }

    #[tokio::test]
    async fn fetch_objects_enforces_capsule_visibility() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();

        let rev_public = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 1,
                summary: "public".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let rev_private = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 2,
                summary: "private".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let capsule_public = store
            .store_object(&Object::Capsule(Capsule {
                revision_id: rev_public,
                public_fields: CapsulePublic {
                    agent_id: "agent".to_string(),
                    agent_version: None,
                    toolchain_digest: None,
                    env_fingerprint: None,
                    evidence: vec![],
                },
                encrypted_private: None,
                encryption: String::new(),
                key_id: None,
                signatures: vec![],
            }))
            .unwrap();

        let capsule_private = store
            .store_object(&Object::Capsule(Capsule {
                revision_id: rev_private,
                public_fields: CapsulePublic {
                    agent_id: "agent".to_string(),
                    agent_version: None,
                    toolchain_digest: None,
                    env_fingerprint: None,
                    evidence: vec![],
                },
                encrypted_private: Some(vec![1, 2, 3]),
                encryption: "xchacha20poly1305".to_string(),
                key_id: None,
                signatures: vec![],
            }))
            .unwrap();

        let rev_public_with_capsule = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: Some(capsule_public),
                author: "test".to_string(),
                created_at_ms: 3,
                summary: "public-with-capsule".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let rev_private_with_capsule = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![],
                snapshot_base: None,
                tree: None,
                capsule_id: Some(capsule_private),
                author: "test".to_string(),
                created_at_ms: 4,
                summary: "private-with-capsule".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let server = SyncServer::new(store);
        let response = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: vec![
                    crate::proto::common::ObjectId {
                        hash: rev_public_with_capsule.as_bytes().to_vec(),
                    },
                    crate::proto::common::ObjectId {
                        hash: rev_private_with_capsule.as_bytes().to_vec(),
                    },
                ],
                have: vec![],
                filter: Some(crate::proto::sync::PartialCloneFilter {
                    intent_ids: vec![],
                    path_prefixes: vec![],
                    time_range_start: 0,
                    time_range_end: 0,
                    codec_ids: vec![],
                    capsule_visibility: "public".to_string(),
                    max_bytes: 0,
                    max_depth: 0,
                }),
            }))
            .await
            .unwrap();

        let ids = collect_streamed_ids(response).await;
        assert!(ids.contains(&rev_public_with_capsule));
        assert!(ids.contains(&capsule_public));
        assert!(!ids.contains(&rev_private_with_capsule));
        assert!(!ids.contains(&capsule_private));
    }

    #[tokio::test]
    async fn fetch_objects_enforces_max_bytes_budget() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();

        let blob_small = store
            .store_object(&Object::Blob(Blob {
                data: b"small".to_vec(),
                media_type: None,
            }))
            .unwrap();
        let blob_large = store
            .store_object(&Object::Blob(Blob {
                data: vec![42u8; 128],
                media_type: None,
            }))
            .unwrap();

        let mut ordered = [blob_small, blob_large];
        ordered.sort_by_key(|id| id.to_hex());

        let first_obj = store.load_object(&ordered[0]).unwrap();
        let first_payload = first_obj.serialize_payload().unwrap();
        let first_cof = cof_encode(first_obj.type_tag(), &first_payload).unwrap();
        let max_bytes = first_cof.len() as u64;

        let server = SyncServer::new(store);
        let response = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: vec![
                    crate::proto::common::ObjectId {
                        hash: blob_small.as_bytes().to_vec(),
                    },
                    crate::proto::common::ObjectId {
                        hash: blob_large.as_bytes().to_vec(),
                    },
                ],
                have: vec![],
                filter: Some(crate::proto::sync::PartialCloneFilter {
                    intent_ids: vec![],
                    path_prefixes: vec![],
                    time_range_start: 0,
                    time_range_end: 0,
                    codec_ids: vec![],
                    capsule_visibility: String::new(),
                    max_bytes,
                    max_depth: 0,
                }),
            }))
            .await
            .unwrap();

        let ids = collect_streamed_ids(response).await;
        assert_eq!(ids, vec![ordered[0]]);
    }

    #[tokio::test]
    async fn fetch_objects_keeps_dependencies_of_filtered_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();

        let patch_src = store
            .store_object(&Object::Patch(Patch {
                target_path: "src/main.rs".to_string(),
                codec_id: "text/line".to_string(),
                base_object: None,
                result_object: None,
                ops: vec![],
                codec_payload: None,
            }))
            .unwrap();
        let patch_docs = store
            .store_object(&Object::Patch(Patch {
                target_path: "docs/readme.md".to_string(),
                codec_id: "text/line".to_string(),
                base_object: None,
                result_object: None,
                ops: vec![],
                codec_payload: None,
            }))
            .unwrap();
        let revision = store
            .store_object(&Object::Revision(Revision {
                change_id: None,
                parents: vec![],
                patches: vec![patch_src, patch_docs],
                snapshot_base: None,
                tree: None,
                capsule_id: None,
                author: "test".to_string(),
                created_at_ms: 1,
                summary: "rev".to_string(),
                policy_evidence: vec![],
            }))
            .unwrap();

        let server = SyncServer::new(store);
        let response = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: vec![crate::proto::common::ObjectId {
                    hash: revision.as_bytes().to_vec(),
                }],
                have: vec![],
                filter: Some(crate::proto::sync::PartialCloneFilter {
                    intent_ids: vec![],
                    path_prefixes: vec!["src/".to_string()],
                    time_range_start: 0,
                    time_range_end: 0,
                    codec_ids: vec![],
                    capsule_visibility: String::new(),
                    max_bytes: 0,
                    max_depth: 0,
                }),
            }))
            .await
            .unwrap();

        let ids = collect_streamed_ids(response).await;
        assert!(ids.contains(&revision));
        assert!(ids.contains(&patch_src));
        assert!(ids.contains(&patch_docs));
    }

    fn make_many_blobs(store: &ClawStore, count: usize) -> Vec<ObjectId> {
        (0..count)
            .map(|idx| {
                store
                    .store_object(&Object::Blob(Blob {
                        data: format!("blob-{idx:04}").into_bytes(),
                        media_type: None,
                    }))
                    .unwrap()
            })
            .collect()
    }

    #[tokio::test]
    async fn fetch_objects_applies_backpressure_when_overloaded() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let want_ids = make_many_blobs(&store, 200);

        let server = SyncServer::new_with_options(
            store,
            SyncServerOptions {
                worker_pool_size: 1,
                queue_capacity: 0,
                backpressure: true,
                io_timeout: Duration::from_millis(1_000),
            },
        );

        let first = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: want_ids
                    .iter()
                    .map(|id| crate::proto::common::ObjectId {
                        hash: id.as_bytes().to_vec(),
                    })
                    .collect(),
                have: vec![],
                filter: None,
            }))
            .await
            .unwrap();

        let _held_stream = first.into_inner();
        tokio::time::sleep(Duration::from_millis(20)).await;

        let overloaded = server
            .advertise_refs(Request::new(AdvertiseRefsRequest {
                prefix: String::new(),
            }))
            .await
            .expect_err("second request should hit backpressure");

        assert_eq!(overloaded.code(), tonic::Code::ResourceExhausted);
    }

    #[tokio::test]
    async fn waiting_for_worker_honors_io_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let want_ids = make_many_blobs(&store, 200);

        let server = SyncServer::new_with_options(
            store,
            SyncServerOptions {
                worker_pool_size: 1,
                queue_capacity: 1,
                backpressure: true,
                io_timeout: Duration::from_millis(50),
            },
        );

        let first = server
            .fetch_objects(Request::new(FetchObjectsRequest {
                want: want_ids
                    .iter()
                    .map(|id| crate::proto::common::ObjectId {
                        hash: id.as_bytes().to_vec(),
                    })
                    .collect(),
                have: vec![],
                filter: None,
            }))
            .await
            .unwrap();
        let _held_stream = first.into_inner();

        tokio::time::sleep(Duration::from_millis(20)).await;

        let timed_out = server
            .advertise_refs(Request::new(AdvertiseRefsRequest {
                prefix: String::new(),
            }))
            .await
            .expect_err("request should time out while waiting for worker");

        assert_eq!(timed_out.code(), tonic::Code::DeadlineExceeded);
    }
}

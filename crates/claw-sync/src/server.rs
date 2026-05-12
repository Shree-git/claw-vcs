use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use tonic::{Request, Response, Status};

use claw_core::cof::cof_encode;
use claw_core::id::ObjectId;
use claw_store::ClawStore;

use crate::ancestry::is_ancestor;
use crate::event_service::EventBus;
use crate::negotiation::{find_reachable_objects, find_reachable_objects_with_depth};
use crate::partial_clone::{CapsuleVisibilityFilter, PartialCloneFilter};
use crate::proto::sync::sync_service_server::SyncService;
use crate::proto::sync::*;
use crate::protocol::negotiate_capabilities;
use crate::security::{
    AllowAllAuthorizer, AuditEvent, AuditSink, AuthorizationAction, AuthorizationDecision,
    AuthorizationRequest, AuthorizationSubject, Authorizer, RateLimiter, ReplayProtectionConfig,
    ReplayProtector, TracingAuditSink, PRINCIPAL_METADATA_KEY, REPLAY_NONCE_METADATA_KEY,
    REQUEST_ID_METADATA_KEY, TOKEN_ID_METADATA_KEY,
};

static SYNC_REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub struct SyncServer {
    store: Arc<RwLock<ClawStore>>,
    limits: Arc<ServerLimits>,
    event_bus: Option<EventBus>,
    security: Arc<SyncSecurity>,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncServerOptions {
    pub worker_pool_size: usize,
    pub queue_capacity: usize,
    pub backpressure: bool,
    pub io_timeout: Duration,
    pub max_push_chunk_bytes: Option<usize>,
    pub max_push_request_bytes: Option<usize>,
    pub rate_limit_per_minute: Option<u32>,
}

impl Default for SyncServerOptions {
    fn default() -> Self {
        Self {
            worker_pool_size: 8,
            queue_capacity: 1_024,
            backpressure: true,
            io_timeout: Duration::from_secs(10),
            max_push_chunk_bytes: Some(8 * 1024 * 1024),
            max_push_request_bytes: Some(128 * 1024 * 1024),
            rate_limit_per_minute: None,
        }
    }
}

struct ServerLimits {
    queue: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    rate_limiter: Option<Mutex<RateLimiter>>,
    options: SyncServerOptions,
}

struct SyncSecurity {
    authorizer: Arc<dyn Authorizer>,
    audit_sink: Arc<dyn AuditSink>,
    replay_protector: Option<Arc<Mutex<ReplayProtector>>>,
    require_replay_nonce: bool,
}

impl Default for SyncSecurity {
    fn default() -> Self {
        Self {
            authorizer: Arc::new(AllowAllAuthorizer),
            audit_sink: Arc::new(TracingAuditSink),
            replay_protector: None,
            require_replay_nonce: false,
        }
    }
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
        Self::from_shared_with_options_and_events(store, options, None)
    }

    pub fn from_shared_with_options_and_events(
        store: Arc<RwLock<ClawStore>>,
        options: SyncServerOptions,
        event_bus: Option<EventBus>,
    ) -> Self {
        let worker_pool_size = options.worker_pool_size.max(1);
        let total_capacity = worker_pool_size
            .saturating_add(options.queue_capacity)
            .max(1);
        let rate_limiter = options
            .rate_limit_per_minute
            .map(|limit| Mutex::new(RateLimiter::per_minute(limit, Instant::now())));

        Self {
            store,
            limits: Arc::new(ServerLimits {
                queue: Arc::new(Semaphore::new(total_capacity)),
                workers: Arc::new(Semaphore::new(worker_pool_size)),
                rate_limiter,
                options,
            }),
            event_bus,
            security: Arc::new(SyncSecurity::default()),
        }
    }

    pub fn with_authorizer(mut self, authorizer: Arc<dyn Authorizer>) -> Self {
        self.security = Arc::new(SyncSecurity {
            authorizer,
            audit_sink: self.security.audit_sink.clone(),
            replay_protector: self.security.replay_protector.clone(),
            require_replay_nonce: self.security.require_replay_nonce,
        });
        self
    }

    pub fn with_audit_sink(mut self, audit_sink: Arc<dyn AuditSink>) -> Self {
        self.security = Arc::new(SyncSecurity {
            authorizer: self.security.authorizer.clone(),
            audit_sink,
            replay_protector: self.security.replay_protector.clone(),
            require_replay_nonce: self.security.require_replay_nonce,
        });
        self
    }

    pub fn with_replay_protection(
        mut self,
        config: ReplayProtectionConfig,
        require_replay_nonce: bool,
    ) -> Self {
        self.security = Arc::new(SyncSecurity {
            authorizer: self.security.authorizer.clone(),
            audit_sink: self.security.audit_sink.clone(),
            replay_protector: Some(Arc::new(Mutex::new(ReplayProtector::new(config)))),
            require_replay_nonce,
        });
        self
    }

    #[allow(clippy::result_large_err)]
    fn acquire_queue_permit(&self) -> Result<Option<OwnedSemaphorePermit>, Status> {
        if let Some(limiter) = &self.limits.rate_limiter {
            let mut limiter = limiter
                .lock()
                .map_err(|_| Status::internal("rate limiter lock poisoned"))?;
            if !limiter.try_acquire(Instant::now()) {
                return Err(Status::resource_exhausted("server rate limit exceeded"));
            }
        }

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

    #[allow(clippy::result_large_err)]
    fn authorize_request<T>(
        &self,
        request: &Request<T>,
        action: AuthorizationAction,
        resource: Option<String>,
    ) -> Result<(), Status> {
        let subject = subject_from_request(request);
        let request_id = request_id_from_request(request);
        let auth_request = AuthorizationRequest {
            subject: subject.clone(),
            action: action.clone(),
            resource: resource.clone(),
        };

        match self.security.authorizer.authorize(&auth_request) {
            AuthorizationDecision::Allow => {
                self.security.audit_sink.record(AuditEvent::allowed(
                    now_ms(),
                    request_id,
                    subject,
                    action,
                    resource,
                ));
                Ok(())
            }
            AuthorizationDecision::Deny { reason } => {
                self.security.audit_sink.record(AuditEvent::denied(
                    now_ms(),
                    request_id,
                    subject,
                    action,
                    resource,
                    reason.clone(),
                ));
                Err(Status::permission_denied(reason))
            }
        }
    }

    #[allow(clippy::result_large_err)]
    fn authorize_and_enforce_replay_nonce<T>(
        &self,
        request: &Request<T>,
        action: AuthorizationAction,
        resource: Option<String>,
    ) -> Result<(), Status> {
        let subject = subject_from_request(request);
        let request_id = request_id_from_request(request);
        let auth_request = AuthorizationRequest {
            subject: subject.clone(),
            action: action.clone(),
            resource: resource.clone(),
        };

        match self.security.authorizer.authorize(&auth_request) {
            AuthorizationDecision::Deny { reason } => {
                self.security.audit_sink.record(AuditEvent::denied(
                    now_ms(),
                    request_id,
                    subject,
                    action,
                    resource,
                    reason.clone(),
                ));
                return Err(Status::permission_denied(reason));
            }
            AuthorizationDecision::Allow => {}
        }

        self.enforce_replay_nonce_after_authorization(
            request,
            action.clone(),
            resource.clone(),
            subject.clone(),
            request_id.clone(),
        )?;

        self.security.audit_sink.record(AuditEvent::allowed(
            now_ms(),
            request_id,
            subject,
            action,
            resource,
        ));
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn enforce_replay_nonce_after_authorization<T>(
        &self,
        request: &Request<T>,
        action: AuthorizationAction,
        resource: Option<String>,
        subject: AuthorizationSubject,
        request_id: String,
    ) -> Result<(), Status> {
        let Some(protector) = &self.security.replay_protector else {
            return Ok(());
        };

        let nonce = metadata_value(request, REPLAY_NONCE_METADATA_KEY);
        if self.security.require_replay_nonce && nonce.is_none() {
            let reason = "missing replay nonce".to_string();
            self.security.audit_sink.record(AuditEvent::denied(
                now_ms(),
                request_id,
                subject,
                action,
                resource,
                reason.clone(),
            ));
            return Err(Status::permission_denied(reason));
        }

        let Some(nonce) = nonce else {
            return Ok(());
        };

        let mut protector = protector
            .lock()
            .map_err(|_| Status::internal("replay protector lock poisoned"))?;
        let replay_scope = format!(
            "principal={};token={};action={action:?};resource={}",
            subject.principal.as_deref().unwrap_or(""),
            subject.token_id.as_deref().unwrap_or(""),
            resource.as_deref().unwrap_or("")
        );
        protector
            .accept_scoped(&nonce, replay_scope, Instant::now())
            .map_err(|err| {
                self.security.audit_sink.record(AuditEvent::denied(
                    now_ms(),
                    request_id,
                    subject,
                    action,
                    resource,
                    err.to_string(),
                ));
                match err {
                    crate::security::ReplayError::EmptyNonce => {
                        Status::invalid_argument("replay nonce cannot be empty")
                    }
                    crate::security::ReplayError::Replay => {
                        Status::already_exists("replayed request nonce")
                    }
                }
            })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn new_sync_request_id() -> String {
    let seq = SYNC_REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("sync-{:x}-{seq:x}", now_ms())
}

fn metadata_value<T>(request: &Request<T>, key: &str) -> Option<String> {
    request
        .metadata()
        .get(key)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn request_id_from_request<T>(request: &Request<T>) -> String {
    metadata_value(request, REQUEST_ID_METADATA_KEY).unwrap_or_else(new_sync_request_id)
}

fn subject_from_request<T>(request: &Request<T>) -> AuthorizationSubject {
    AuthorizationSubject {
        principal: metadata_value(request, PRINCIPAL_METADATA_KEY),
        token_id: metadata_value(request, TOKEN_ID_METADATA_KEY),
        peer_addr: request.remote_addr().map(|addr| addr.to_string()),
    }
}

fn refs_resource(prefix: &str) -> String {
    if prefix.is_empty() {
        "refs:*".to_string()
    } else {
        format!("refs:{prefix}")
    }
}

fn object_fetch_resource(req: &FetchObjectsRequest) -> String {
    format!(
        "objects:fetch want={} have={}",
        req.want.len(),
        req.have.len()
    )
}

fn ref_update_resource(req: &UpdateRefsRequest) -> String {
    let mut names = req
        .updates
        .iter()
        .map(|update| update.name.as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    if names.is_empty() {
        "refs:update empty".to_string()
    } else {
        format!("refs:update {}", names.join(","))
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

#[allow(clippy::result_large_err)]
fn enforce_push_chunk_limits(
    options: &SyncServerOptions,
    chunk_len: usize,
    received_bytes: &mut usize,
) -> Result<(), Status> {
    if options
        .max_push_chunk_bytes
        .is_some_and(|limit| chunk_len > limit)
    {
        return Err(Status::resource_exhausted(
            "push object chunk exceeds configured byte limit",
        ));
    }

    *received_bytes = received_bytes
        .checked_add(chunk_len)
        .ok_or_else(|| Status::resource_exhausted("push object request too large"))?;
    if options
        .max_push_request_bytes
        .is_some_and(|limit| *received_bytes > limit)
    {
        return Err(Status::resource_exhausted(
            "push object request exceeds configured byte limit",
        ));
    }

    Ok(())
}

#[tonic::async_trait]
impl SyncService for SyncServer {
    async fn hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        self.authorize_request(&request, AuthorizationAction::Hello, None)?;
        self.run_bounded("hello", async move {
            let req = request.into_inner();
            Ok(Response::new(HelloResponse {
                server_version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: negotiate_capabilities(&req.capabilities),
            }))
        })
        .await
    }

    async fn advertise_refs(
        &self,
        request: Request<AdvertiseRefsRequest>,
    ) -> Result<Response<AdvertiseRefsResponse>, Status> {
        self.authorize_request(
            &request,
            AuthorizationAction::AdvertiseRefs,
            Some(refs_resource(&request.get_ref().prefix)),
        )?;
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
        self.authorize_request(
            &request,
            AuthorizationAction::FetchObjects,
            Some(object_fetch_resource(request.get_ref())),
        )?;
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
        let resource = Some("objects:push".to_string());
        self.authorize_and_enforce_replay_nonce(
            &request,
            AuthorizationAction::PushObjects,
            resource,
        )?;
        self.run_bounded("push_objects", async move {
            let mut stream = request.into_inner();
            let store = self.store.write().await;
            let mut accepted = Vec::new();
            let mut received_bytes = 0usize;

            while let Some(chunk) = stream.message().await? {
                if chunk.is_last {
                    break;
                }

                enforce_push_chunk_limits(
                    &self.limits.options,
                    chunk.data.len(),
                    &mut received_bytes,
                )?;

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
        let resource = Some(ref_update_resource(request.get_ref()));
        self.authorize_and_enforce_replay_nonce(
            &request,
            AuthorizationAction::UpdateRefs,
            resource,
        )?;
        let event_bus = self.event_bus.clone();
        self.run_bounded("update_refs", async move {
            let req = request.into_inner();
            let mut seen_refs = HashSet::with_capacity(req.updates.len());
            for update in &req.updates {
                if !seen_refs.insert(update.name.as_str()) {
                    return Err(Status::invalid_argument(format!(
                        "duplicate ref name in batch: {}",
                        update.name
                    )));
                }
            }

            let store = self.store.write().await;
            let mut planned_updates = Vec::with_capacity(req.updates.len());

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
                let new_id = update
                    .new_target
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
                if let Some(new_id) = new_id {
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

                planned_updates.push((update.name.clone(), current, new_id));
            }

            // Pass 2: apply all updates
            for (name, _old_target, new_target) in &planned_updates {
                match new_target {
                    Some(id) => store
                        .set_ref(name, id)
                        .map_err(|e| Status::internal(e.to_string()))?,
                    None => store
                        .delete_ref(name)
                        .map_err(|e| Status::internal(e.to_string()))?,
                }
            }

            if let Some(bus) = &event_bus {
                for (name, old_target, new_target) in &planned_updates {
                    match (old_target, new_target) {
                        (_, Some(id)) => {
                            let event_type = if old_target.is_some() {
                                "ref_updated"
                            } else {
                                "ref_created"
                            };
                            bus.publish_ref_event(
                                event_type,
                                name,
                                Some(crate::proto::common::ObjectId {
                                    hash: id.as_bytes().to_vec(),
                                }),
                            );
                        }
                        (Some(_), None) => {
                            bus.publish_ref_event("ref_deleted", name, None);
                        }
                        (None, None) => {}
                    }
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
    use crate::event_service::EventBus;
    use crate::security::{
        AuditOutcome, AuditSink, AuthorizationDecision, AuthorizationRequest, AuthorizationRole,
        Authorizer, RoleBasedAuthorizer, PRINCIPAL_METADATA_KEY, REPLAY_NONCE_METADATA_KEY,
    };
    use claw_core::hash::content_hash;
    use claw_core::id::{ChangeId, IntentId};
    use claw_core::object::Object;
    use claw_core::object::TypeTag;
    use claw_core::types::{Blob, Capsule, CapsulePublic, Change, ChangeStatus, Patch, Revision};
    use std::sync::Mutex as StdMutex;
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

    #[derive(Clone, Default)]
    struct TestAuditSink {
        events: Arc<StdMutex<Vec<AuditEvent>>>,
    }

    impl AuditSink for TestAuditSink {
        fn record(&self, event: AuditEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[derive(Default)]
    struct DenyOnceAuthorizer {
        has_denied: StdMutex<bool>,
    }

    impl Authorizer for DenyOnceAuthorizer {
        fn authorize(&self, _request: &AuthorizationRequest) -> AuthorizationDecision {
            let mut has_denied = self.has_denied.lock().unwrap();
            if !*has_denied {
                *has_denied = true;
                return AuthorizationDecision::Deny {
                    reason: "temporary authorization denial".to_string(),
                };
            }

            AuthorizationDecision::Allow
        }
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
                recipients: vec![],
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
                recipients: vec![],
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
                ..SyncServerOptions::default()
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
                ..SyncServerOptions::default()
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

    #[tokio::test]
    async fn update_refs_publishes_ref_events_to_bus() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let shared = Arc::new(RwLock::new(store));
        let event_bus = EventBus::new(8);
        let mut events = event_bus.subscribe();
        let target = content_hash(TypeTag::Blob, b"target");

        let server = SyncServer::from_shared_with_options_and_events(
            shared.clone(),
            SyncServerOptions::default(),
            Some(event_bus),
        );

        let response = server
            .update_refs(Request::new(UpdateRefsRequest {
                updates: vec![RefUpdate {
                    name: "heads/main".to_string(),
                    old_target: None,
                    new_target: Some(crate::proto::common::ObjectId {
                        hash: target.as_bytes().to_vec(),
                    }),
                    force: false,
                }],
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(response.success);
        let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.event_type, "ref_created");
        assert_eq!(event.ref_name, "heads/main");
        assert_eq!(event.object_id.unwrap().hash, target.as_bytes().to_vec());

        let response = server
            .update_refs(Request::new(UpdateRefsRequest {
                updates: vec![RefUpdate {
                    name: "heads/main".to_string(),
                    old_target: Some(crate::proto::common::ObjectId {
                        hash: target.as_bytes().to_vec(),
                    }),
                    new_target: None,
                    force: false,
                }],
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(response.success);
        assert!(shared.read().await.get_ref("heads/main").unwrap().is_none());
        let event = tokio::time::timeout(Duration::from_secs(1), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.event_type, "ref_deleted");
        assert_eq!(event.ref_name, "heads/main");
        assert!(event.object_id.is_none());
    }

    #[tokio::test]
    async fn update_refs_rejects_duplicate_names_in_batch() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let server = SyncServer::new(store);
        let first = content_hash(TypeTag::Blob, b"first");
        let second = content_hash(TypeTag::Blob, b"second");

        let err = server
            .update_refs(Request::new(UpdateRefsRequest {
                updates: vec![
                    RefUpdate {
                        name: "heads/main".to_string(),
                        old_target: None,
                        new_target: Some(crate::proto::common::ObjectId {
                            hash: first.as_bytes().to_vec(),
                        }),
                        force: false,
                    },
                    RefUpdate {
                        name: "heads/main".to_string(),
                        old_target: None,
                        new_target: Some(crate::proto::common::ObjectId {
                            hash: second.as_bytes().to_vec(),
                        }),
                        force: false,
                    },
                ],
            }))
            .await
            .expect_err("duplicate ref names should be rejected");

        assert_eq!(err.code(), tonic::Code::InvalidArgument);
        assert!(err.message().contains("duplicate ref name in batch"));
    }

    #[tokio::test]
    async fn role_authorizer_denies_ref_update_and_records_audit_event() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let audit = TestAuditSink::default();
        let target = content_hash(TypeTag::Blob, b"target");

        let server = SyncServer::new(store)
            .with_authorizer(Arc::new(
                RoleBasedAuthorizer::new().grant_role("reader", AuthorizationRole::Reader),
            ))
            .with_audit_sink(Arc::new(audit.clone()));

        let mut request = Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: None,
                new_target: Some(crate::proto::common::ObjectId {
                    hash: target.as_bytes().to_vec(),
                }),
                force: false,
            }],
        });
        request
            .metadata_mut()
            .insert(PRINCIPAL_METADATA_KEY, "reader".parse().unwrap());

        let denied = server
            .update_refs(request)
            .await
            .expect_err("reader role cannot update refs");
        assert_eq!(denied.code(), tonic::Code::PermissionDenied);

        let events = audit.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].outcome, AuditOutcome::Denied);
        assert_eq!(events[0].action, AuthorizationAction::UpdateRefs);
        assert_eq!(events[0].subject.principal.as_deref(), Some("reader"));
    }

    #[tokio::test]
    async fn replay_protection_requires_nonce_for_ref_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let server = SyncServer::new(store).with_replay_protection(
            ReplayProtectionConfig {
                window: Duration::from_secs(60),
                max_entries: 16,
            },
            true,
        );
        let target = content_hash(TypeTag::Blob, b"target");

        let denied = server
            .update_refs(Request::new(UpdateRefsRequest {
                updates: vec![RefUpdate {
                    name: "heads/main".to_string(),
                    old_target: None,
                    new_target: Some(crate::proto::common::ObjectId {
                        hash: target.as_bytes().to_vec(),
                    }),
                    force: false,
                }],
            }))
            .await
            .expect_err("missing replay nonce should be rejected");

        assert_eq!(denied.code(), tonic::Code::PermissionDenied);
    }

    #[tokio::test]
    async fn replay_protection_rejects_duplicate_nonce_for_ref_updates() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let server = SyncServer::new(store).with_replay_protection(
            ReplayProtectionConfig {
                window: Duration::from_secs(60),
                max_entries: 16,
            },
            true,
        );
        let target = content_hash(TypeTag::Blob, b"target");

        let mut first = Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: None,
                new_target: Some(crate::proto::common::ObjectId {
                    hash: target.as_bytes().to_vec(),
                }),
                force: false,
            }],
        });
        first
            .metadata_mut()
            .insert(REPLAY_NONCE_METADATA_KEY, "nonce-1".parse().unwrap());
        assert!(
            server
                .update_refs(first)
                .await
                .unwrap()
                .into_inner()
                .success
        );

        let mut second = Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: None,
                new_target: Some(crate::proto::common::ObjectId {
                    hash: target.as_bytes().to_vec(),
                }),
                force: false,
            }],
        });
        second
            .metadata_mut()
            .insert(REPLAY_NONCE_METADATA_KEY, "nonce-1".parse().unwrap());

        let replayed = server
            .update_refs(second)
            .await
            .expect_err("duplicate nonce should be rejected");
        assert_eq!(replayed.code(), tonic::Code::AlreadyExists);
    }

    #[tokio::test]
    async fn authorization_denial_does_not_consume_replay_nonce() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let server = SyncServer::new(store)
            .with_authorizer(Arc::new(DenyOnceAuthorizer::default()))
            .with_replay_protection(
                ReplayProtectionConfig {
                    window: Duration::from_secs(60),
                    max_entries: 16,
                },
                true,
            );
        let target = content_hash(TypeTag::Blob, b"target");

        let mut denied_request = Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: None,
                new_target: Some(crate::proto::common::ObjectId {
                    hash: target.as_bytes().to_vec(),
                }),
                force: false,
            }],
        });
        denied_request
            .metadata_mut()
            .insert(PRINCIPAL_METADATA_KEY, "operator".parse().unwrap());
        denied_request
            .metadata_mut()
            .insert(REPLAY_NONCE_METADATA_KEY, "shared-nonce".parse().unwrap());
        let denied = server
            .update_refs(denied_request)
            .await
            .expect_err("first call should be denied before replay is recorded");
        assert_eq!(denied.code(), tonic::Code::PermissionDenied);

        let mut allowed_retry = Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: None,
                new_target: Some(crate::proto::common::ObjectId {
                    hash: target.as_bytes().to_vec(),
                }),
                force: false,
            }],
        });
        allowed_retry
            .metadata_mut()
            .insert(PRINCIPAL_METADATA_KEY, "operator".parse().unwrap());
        allowed_retry
            .metadata_mut()
            .insert(REPLAY_NONCE_METADATA_KEY, "shared-nonce".parse().unwrap());

        let accepted = server
            .update_refs(allowed_retry)
            .await
            .expect("authorized retry should be allowed to use the nonce")
            .into_inner();
        assert!(accepted.success);
    }

    #[tokio::test]
    async fn rate_limit_rejects_excess_requests() {
        let tmp = tempfile::tempdir().unwrap();
        let store = ClawStore::init(tmp.path()).unwrap();
        let server = SyncServer::new_with_options(
            store,
            SyncServerOptions {
                backpressure: false,
                rate_limit_per_minute: Some(1),
                ..SyncServerOptions::default()
            },
        );

        server
            .hello(Request::new(HelloRequest {
                client_version: "0.1.0".to_string(),
                capabilities: vec![],
            }))
            .await
            .unwrap();

        let limited = server
            .hello(Request::new(HelloRequest {
                client_version: "0.1.0".to_string(),
                capabilities: vec![],
            }))
            .await
            .expect_err("second request should hit rate limit");

        assert_eq!(limited.code(), tonic::Code::ResourceExhausted);
    }

    #[test]
    fn push_chunk_limits_reject_large_chunk_and_total() {
        let options = SyncServerOptions {
            max_push_chunk_bytes: Some(4),
            max_push_request_bytes: Some(6),
            ..SyncServerOptions::default()
        };
        let mut received = 0;

        let too_large_chunk = enforce_push_chunk_limits(&options, 5, &mut received).unwrap_err();
        assert_eq!(too_large_chunk.code(), tonic::Code::ResourceExhausted);

        enforce_push_chunk_limits(&options, 3, &mut received).unwrap();
        enforce_push_chunk_limits(&options, 3, &mut received).unwrap();
        let too_large_request = enforce_push_chunk_limits(&options, 1, &mut received).unwrap_err();
        assert_eq!(too_large_request.code(), tonic::Code::ResourceExhausted);
    }
}

use async_trait::async_trait;
use claw_core::cof::{cof_decode, cof_encode};
use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_store::ClawStore;
use rand::Rng;
use tokio::time::{sleep, Duration};
use tonic::Code;

use crate::http_client::HttpSyncClient;
use crate::proto::sync::sync_service_client::SyncServiceClient;
use crate::proto::sync::*;
use crate::transport::{RemoteTransportConfig, SyncTransport};
use crate::SyncError;

pub struct SyncClient {
    inner: Box<dyn SyncTransport>,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub idempotent_only: bool,
    pub max_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            idempotent_only: true,
            max_attempts: 4,
            base_backoff_ms: 100,
            max_backoff_ms: 2_000,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    fn attempts(self) -> u32 {
        self.max_attempts.max(1)
    }

    fn backoff_delay(self, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1).min(31);
        let factor = 1u64 << exponent;
        let base = self.base_backoff_ms.saturating_mul(factor);
        let capped = base.min(self.max_backoff_ms.max(self.base_backoff_ms));

        if !self.jitter {
            return Duration::from_millis(capped);
        }

        let lower = capped / 2;
        let upper = capped.saturating_mul(3).saturating_div(2).max(lower + 1);
        let sampled = rand::thread_rng().gen_range(lower..upper);
        Duration::from_millis(sampled)
    }
}

impl SyncClient {
    pub async fn connect(addr: &str) -> Result<Self, SyncError> {
        Self::connect_with_transport(RemoteTransportConfig::Grpc {
            addr: addr.to_string(),
            bearer_token: None,
        })
        .await
    }

    pub async fn connect_with_transport(config: RemoteTransportConfig) -> Result<Self, SyncError> {
        Self::connect_with_transport_and_retry(config, RetryPolicy::default()).await
    }

    pub async fn connect_with_transport_and_retry(
        config: RemoteTransportConfig,
        retry_policy: RetryPolicy,
    ) -> Result<Self, SyncError> {
        let inner: Box<dyn SyncTransport> = match config {
            RemoteTransportConfig::Grpc { addr, bearer_token } => {
                Box::new(GrpcSyncClient::connect(&addr, bearer_token).await?)
            }
            RemoteTransportConfig::Http {
                base_url,
                repo,
                bearer_token,
            } => Box::new(HttpSyncClient::new(base_url, repo, bearer_token)),
        };

        Ok(Self {
            inner,
            retry_policy,
        })
    }

    fn should_retry(&self, err: &SyncError) -> bool {
        match err {
            SyncError::Grpc(status) => matches!(
                status.code(),
                Code::Unavailable
                    | Code::DeadlineExceeded
                    | Code::ResourceExhausted
                    | Code::Aborted
                    | Code::Internal
            ),
            SyncError::Transport(_) => true,
            SyncError::Http(err) => {
                if err.is_timeout() || err.is_connect() {
                    return true;
                }

                matches!(
                    err.status(),
                    Some(
                        reqwest::StatusCode::TOO_MANY_REQUESTS
                            | reqwest::StatusCode::BAD_GATEWAY
                            | reqwest::StatusCode::SERVICE_UNAVAILABLE
                            | reqwest::StatusCode::GATEWAY_TIMEOUT
                    )
                )
            }
            _ => false,
        }
    }

    async fn retry_wait(&self, attempt: u32) {
        let delay = self.retry_policy.backoff_delay(attempt);
        sleep(delay).await;
    }

    async fn retry_hello(&mut self) -> Result<HelloResponse, SyncError> {
        let attempts = self.retry_policy.attempts();
        let mut last_err = None;

        for attempt in 1..=attempts {
            match self.inner.hello().await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let retryable = self.should_retry(&err);
                    last_err = Some(err);
                    if !retryable || attempt == attempts {
                        break;
                    }
                    self.retry_wait(attempt).await;
                }
            }
        }

        Err(last_err.expect("retry loop always records error before exit"))
    }

    async fn retry_advertise_refs(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<(String, ObjectId)>, SyncError> {
        let attempts = self.retry_policy.attempts();
        let mut last_err = None;

        for attempt in 1..=attempts {
            match self.inner.advertise_refs(prefix).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let retryable = self.should_retry(&err);
                    last_err = Some(err);
                    if !retryable || attempt == attempts {
                        break;
                    }
                    self.retry_wait(attempt).await;
                }
            }
        }

        Err(last_err.expect("retry loop always records error before exit"))
    }

    async fn retry_fetch_objects(
        &mut self,
        store: &ClawStore,
        want: &[ObjectId],
        have: &[ObjectId],
    ) -> Result<Vec<ObjectId>, SyncError> {
        let attempts = self.retry_policy.attempts();
        let mut last_err = None;

        for attempt in 1..=attempts {
            match self.inner.fetch_objects(store, want, have).await {
                Ok(resp) => return Ok(resp),
                Err(err) => {
                    let retryable = self.should_retry(&err);
                    last_err = Some(err);
                    if !retryable || attempt == attempts {
                        break;
                    }
                    self.retry_wait(attempt).await;
                }
            }
        }

        Err(last_err.expect("retry loop always records error before exit"))
    }

    pub async fn hello(&mut self) -> Result<HelloResponse, SyncError> {
        self.retry_hello().await
    }

    pub async fn advertise_refs(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<(String, ObjectId)>, SyncError> {
        self.retry_advertise_refs(prefix).await
    }

    pub async fn fetch_objects(
        &mut self,
        store: &ClawStore,
        want: &[ObjectId],
        have: &[ObjectId],
    ) -> Result<Vec<ObjectId>, SyncError> {
        self.retry_fetch_objects(store, want, have).await
    }

    pub async fn update_refs(
        &mut self,
        updates: &[(String, Option<ObjectId>, ObjectId)],
        force: bool,
    ) -> Result<UpdateRefsResponse, SyncError> {
        self.inner.update_refs(updates, force).await
    }

    pub async fn push_objects(
        &mut self,
        store: &ClawStore,
        ids: &[ObjectId],
    ) -> Result<PushObjectsResponse, SyncError> {
        self.inner.push_objects(store, ids).await
    }
}

#[cfg(test)]
impl SyncClient {
    fn from_transport_for_test(inner: Box<dyn SyncTransport>, retry_policy: RetryPolicy) -> Self {
        Self {
            inner,
            retry_policy,
        }
    }
}

pub struct GrpcSyncClient {
    client: SyncServiceClient<tonic::transport::Channel>,
    bearer_token: Option<String>,
}

impl GrpcSyncClient {
    pub async fn connect(addr: &str, bearer_token: Option<String>) -> Result<Self, SyncError> {
        let client = SyncServiceClient::connect(addr.to_string()).await?;
        Ok(Self {
            client,
            bearer_token,
        })
    }

    #[allow(clippy::result_large_err)]
    fn with_auth<T>(&self, mut request: tonic::Request<T>) -> Result<tonic::Request<T>, SyncError> {
        if let Some(token) = &self.bearer_token {
            let auth_value = format!("Bearer {token}");
            let metadata_value = auth_value.parse().map_err(|e| {
                SyncError::TransferFailed(format!("invalid bearer token metadata: {e}"))
            })?;
            request
                .metadata_mut()
                .insert("authorization", metadata_value);
        }
        Ok(request)
    }
}

#[async_trait]
impl SyncTransport for GrpcSyncClient {
    async fn hello(&mut self) -> Result<HelloResponse, SyncError> {
        let request = self.with_auth(tonic::Request::new(HelloRequest {
            client_version: "0.1.0".to_string(),
            capabilities: vec!["partial-clone".to_string()],
        }))?;
        let resp = self.client.hello(request).await?;
        Ok(resp.into_inner())
    }

    async fn advertise_refs(&mut self, prefix: &str) -> Result<Vec<(String, ObjectId)>, SyncError> {
        let request = self.with_auth(tonic::Request::new(AdvertiseRefsRequest {
            prefix: prefix.to_string(),
        }))?;
        let resp = self.client.advertise_refs(request).await?;
        let inner = resp.into_inner();

        let mut refs = Vec::new();
        for entry in inner.refs {
            if let Some(id_msg) = entry.target {
                let id_bytes: [u8; 32] = id_msg
                    .hash
                    .as_slice()
                    .try_into()
                    .map_err(|_| SyncError::NegotiationFailed("invalid object id".into()))?;
                refs.push((entry.name, ObjectId::from_bytes(id_bytes)));
            }
        }
        Ok(refs)
    }

    async fn fetch_objects(
        &mut self,
        store: &ClawStore,
        want: &[ObjectId],
        have: &[ObjectId],
    ) -> Result<Vec<ObjectId>, SyncError> {
        let want_msgs: Vec<_> = want
            .iter()
            .map(|id| crate::proto::common::ObjectId {
                hash: id.as_bytes().to_vec(),
            })
            .collect();
        let have_msgs: Vec<_> = have
            .iter()
            .map(|id| crate::proto::common::ObjectId {
                hash: id.as_bytes().to_vec(),
            })
            .collect();

        let resp = self
            .client
            .fetch_objects(self.with_auth(tonic::Request::new(FetchObjectsRequest {
                want: want_msgs,
                have: have_msgs,
                filter: None,
            }))?)
            .await?;

        let mut stream = resp.into_inner();
        let mut fetched = Vec::new();

        while let Some(chunk) = stream.message().await? {
            if chunk.is_last {
                break;
            }
            let (type_tag, payload) = cof_decode(&chunk.data)?;
            let obj = Object::deserialize_payload(type_tag, &payload)?;
            let id = store.store_object(&obj)?;
            fetched.push(id);
        }

        Ok(fetched)
    }

    async fn update_refs(
        &mut self,
        updates: &[(String, Option<ObjectId>, ObjectId)],
        force: bool,
    ) -> Result<UpdateRefsResponse, SyncError> {
        let proto_updates: Vec<RefUpdate> = updates
            .iter()
            .map(|(name, old, new)| RefUpdate {
                name: name.clone(),
                old_target: old.map(|id| crate::proto::common::ObjectId {
                    hash: id.as_bytes().to_vec(),
                }),
                new_target: Some(crate::proto::common::ObjectId {
                    hash: new.as_bytes().to_vec(),
                }),
                force,
            })
            .collect();

        let resp = self
            .client
            .update_refs(self.with_auth(tonic::Request::new(UpdateRefsRequest {
                updates: proto_updates,
            }))?)
            .await?;
        Ok(resp.into_inner())
    }

    async fn push_objects(
        &mut self,
        store: &ClawStore,
        ids: &[ObjectId],
    ) -> Result<PushObjectsResponse, SyncError> {
        let mut chunks = Vec::new();

        for id in ids {
            let obj = store.load_object(id)?;
            let payload = obj.serialize_payload()?;
            let type_tag = obj.type_tag();
            let cof_data = cof_encode(type_tag, &payload)?;

            chunks.push(ObjectChunk {
                id: Some(crate::proto::common::ObjectId {
                    hash: id.as_bytes().to_vec(),
                }),
                object_type: type_tag as i32,
                data: cof_data,
                is_last: false,
            });
        }

        chunks.push(ObjectChunk {
            id: None,
            object_type: 0,
            data: vec![],
            is_last: true,
        });

        let stream = tokio_stream::iter(chunks);
        let request = self.with_auth(tonic::Request::new(stream))?;
        let resp = self.client.push_objects(request).await?;
        Ok(resp.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockTransport {
        hello_remaining_failures: Arc<Mutex<u32>>,
        update_refs_calls: Arc<Mutex<u32>>,
    }

    impl MockTransport {
        fn new(hello_remaining_failures: u32) -> Self {
            Self {
                hello_remaining_failures: Arc::new(Mutex::new(hello_remaining_failures)),
                update_refs_calls: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl SyncTransport for MockTransport {
        async fn hello(&mut self) -> Result<HelloResponse, SyncError> {
            let mut remaining = self.hello_remaining_failures.lock().await;
            if *remaining > 0 {
                *remaining -= 1;
                return Err(SyncError::Grpc(tonic::Status::unavailable("temporary")));
            }

            Ok(HelloResponse {
                server_version: "0.1.0".to_string(),
                capabilities: vec![],
            })
        }

        async fn advertise_refs(
            &mut self,
            _prefix: &str,
        ) -> Result<Vec<(String, ObjectId)>, SyncError> {
            Ok(Vec::new())
        }

        async fn fetch_objects(
            &mut self,
            _store: &ClawStore,
            _want: &[ObjectId],
            _have: &[ObjectId],
        ) -> Result<Vec<ObjectId>, SyncError> {
            Ok(Vec::new())
        }

        async fn update_refs(
            &mut self,
            _updates: &[(String, Option<ObjectId>, ObjectId)],
            _force: bool,
        ) -> Result<UpdateRefsResponse, SyncError> {
            let mut calls = self.update_refs_calls.lock().await;
            *calls += 1;
            Err(SyncError::Grpc(tonic::Status::unavailable(
                "non-idempotent should not retry",
            )))
        }

        async fn push_objects(
            &mut self,
            _store: &ClawStore,
            _ids: &[ObjectId],
        ) -> Result<PushObjectsResponse, SyncError> {
            Ok(PushObjectsResponse {
                success: true,
                message: "ok".to_string(),
                accepted: vec![],
            })
        }
    }

    #[tokio::test]
    async fn retries_idempotent_hello_until_success() {
        let transport = Box::new(MockTransport::new(2));
        let mut client = SyncClient::from_transport_for_test(
            transport,
            RetryPolicy {
                idempotent_only: true,
                max_attempts: 4,
                base_backoff_ms: 1,
                max_backoff_ms: 4,
                jitter: false,
            },
        );

        let hello = client.hello().await.expect("hello should eventually succeed");
        assert_eq!(hello.server_version, "0.1.0");
    }

    #[tokio::test]
    async fn does_not_retry_non_idempotent_update_refs() {
        let transport = Box::new(MockTransport::new(0));
        let mut client = SyncClient::from_transport_for_test(
            transport,
            RetryPolicy {
                idempotent_only: true,
                max_attempts: 5,
                base_backoff_ms: 1,
                max_backoff_ms: 4,
                jitter: false,
            },
        );

        let err = client
            .update_refs(&[], false)
            .await
            .expect_err("update_refs should fail immediately");
        match err {
            SyncError::Grpc(status) => assert_eq!(status.code(), Code::Unavailable),
            other => panic!("unexpected error: {other}"),
        }
    }
}

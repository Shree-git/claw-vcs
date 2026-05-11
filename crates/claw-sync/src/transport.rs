use async_trait::async_trait;

use claw_core::id::ObjectId;
use claw_store::ClawStore;

use crate::proto::sync::{HelloResponse, PushObjectsResponse, UpdateRefsResponse};
use crate::security::redacted_secret_marker;
use crate::SyncError;

#[derive(Clone)]
pub struct GrpcTlsConfig {
    pub ca_cert_pem: Option<Vec<u8>>,
    pub client_cert_pem: Option<Vec<u8>>,
    pub client_key_pem: Option<Vec<u8>>,
    pub domain_name: Option<String>,
}

impl std::fmt::Debug for GrpcTlsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcTlsConfig")
            .field(
                "ca_cert_pem",
                &redacted_secret_marker(self.ca_cert_pem.is_some()),
            )
            .field(
                "client_cert_pem",
                &redacted_secret_marker(self.client_cert_pem.is_some()),
            )
            .field(
                "client_key_pem",
                &redacted_secret_marker(self.client_key_pem.is_some()),
            )
            .field("domain_name", &self.domain_name)
            .finish()
    }
}

#[derive(Clone)]
pub enum RemoteTransportConfig {
    Grpc {
        addr: String,
        bearer_token: Option<String>,
        tls: Option<GrpcTlsConfig>,
    },
    Http {
        base_url: String,
        repo: String,
        bearer_token: Option<String>,
    },
}

impl std::fmt::Debug for RemoteTransportConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grpc {
                addr,
                bearer_token,
                tls,
            } => f
                .debug_struct("Grpc")
                .field("addr", addr)
                .field(
                    "bearer_token",
                    &redacted_secret_marker(bearer_token.is_some()),
                )
                .field("tls", tls)
                .finish(),
            Self::Http {
                base_url,
                repo,
                bearer_token,
            } => f
                .debug_struct("Http")
                .field("base_url", base_url)
                .field("repo", repo)
                .field(
                    "bearer_token",
                    &redacted_secret_marker(bearer_token.is_some()),
                )
                .finish(),
        }
    }
}

#[async_trait]
pub trait SyncTransport: Send {
    async fn hello(&mut self) -> Result<HelloResponse, SyncError>;

    async fn advertise_refs(&mut self, prefix: &str) -> Result<Vec<(String, ObjectId)>, SyncError>;

    async fn fetch_objects(
        &mut self,
        store: &ClawStore,
        want: &[ObjectId],
        have: &[ObjectId],
    ) -> Result<Vec<ObjectId>, SyncError>;

    async fn update_refs(
        &mut self,
        updates: &[(String, Option<ObjectId>, ObjectId)],
        force: bool,
    ) -> Result<UpdateRefsResponse, SyncError>;

    async fn push_objects(
        &mut self,
        store: &ClawStore,
        ids: &[ObjectId],
    ) -> Result<PushObjectsResponse, SyncError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_grpc_bearer_token() {
        let config = RemoteTransportConfig::Grpc {
            addr: "http://127.0.0.1:50051".to_string(),
            bearer_token: Some("super-secret-token".to_string()),
            tls: Some(GrpcTlsConfig {
                ca_cert_pem: Some(b"secret-ca".to_vec()),
                client_cert_pem: Some(b"secret-cert".to_vec()),
                client_key_pem: Some(b"secret-key".to_vec()),
                domain_name: Some("localhost".to_string()),
            }),
        };

        let rendered = format!("{config:?}");
        assert!(!rendered.contains("super-secret-token"));
        assert!(!rendered.contains("secret-ca"));
        assert!(!rendered.contains("secret-cert"));
        assert!(!rendered.contains("secret-key"));
        assert!(rendered.contains("[REDACTED]"));
    }
}

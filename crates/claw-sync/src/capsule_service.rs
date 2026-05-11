use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Capsule, CapsulePublic, CapsuleRecipient, Evidence};
use claw_crypto::capsule::verify_capsule;
use claw_store::ClawStore;

use crate::proto::capsule::capsule_service_server::CapsuleService;
use crate::proto::capsule::*;
use crate::security::{
    metadata_value, AuthorizationAction, Authorizer, ServiceSecurity, PRINCIPAL_METADATA_KEY,
};

pub struct CapsuleServer {
    store: Arc<RwLock<ClawStore>>,
    security: ServiceSecurity,
}

impl CapsuleServer {
    pub fn new(store: Arc<RwLock<ClawStore>>) -> Self {
        Self {
            store,
            security: ServiceSecurity::default(),
        }
    }

    pub fn with_authorizer(mut self, authorizer: Arc<dyn Authorizer>) -> Self {
        self.security = self.security.with_authorizer(authorizer);
        self
    }
}

fn capsule_to_proto(c: &Capsule) -> crate::proto::objects::Capsule {
    crate::proto::objects::Capsule {
        revision_id: Some(crate::proto::common::ObjectId {
            hash: c.revision_id.as_bytes().to_vec(),
        }),
        public_fields: Some(crate::proto::objects::CapsulePublic {
            agent_id: c.public_fields.agent_id.clone(),
            agent_version: c.public_fields.agent_version.clone().unwrap_or_default(),
            toolchain_digest: c.public_fields.toolchain_digest.clone().unwrap_or_default(),
            env_fingerprint: c.public_fields.env_fingerprint.clone().unwrap_or_default(),
            evidence: c
                .public_fields
                .evidence
                .iter()
                .map(|e| crate::proto::objects::Evidence {
                    name: e.name.clone(),
                    status: e.status.clone(),
                    duration_ms: e.duration_ms,
                    artifact_refs: e.artifact_refs.clone(),
                    summary: e.summary.clone().unwrap_or_default(),
                    revision_id: e.revision_id.map(|id| crate::proto::common::ObjectId {
                        hash: id.as_bytes().to_vec(),
                    }),
                    command: e.command.clone(),
                    exit_code: e.exit_code,
                    started_at_ms: e.started_at_ms,
                    ended_at_ms: e.ended_at_ms,
                    environment_digest: e.environment_digest.clone(),
                    runner_identity: e.runner_identity.clone(),
                    log_digest: e.log_digest.clone(),
                    artifact_digest: e.artifact_digest.clone(),
                    expires_at_ms: e.expires_at_ms,
                    trust_domain: e.trust_domain.clone(),
                    signature: e.signature.clone(),
                })
                .collect(),
        }),
        encrypted_private: c.encrypted_private.clone().unwrap_or_default(),
        encryption: c.encryption.clone(),
        key_id: c.key_id.clone().unwrap_or_default(),
        signatures: c
            .signatures
            .iter()
            .map(|s| crate::proto::objects::CapsuleSignature {
                signer_id: s.signer_id.clone(),
                signature: s.signature.clone(),
            })
            .collect(),
        recipients: c
            .recipients
            .iter()
            .map(|r| crate::proto::objects::CapsuleRecipient {
                recipient_id: r.recipient_id.clone(),
                key_id: r.key_id.clone(),
                algorithm: r.algorithm.clone(),
                ephemeral_public_key: r.ephemeral_public_key.clone(),
                encrypted_content_key: r.encrypted_content_key.clone(),
            })
            .collect(),
    }
}

fn public_from_proto(
    p: &crate::proto::objects::CapsulePublic,
) -> Result<CapsulePublic, &'static str> {
    Ok(CapsulePublic {
        agent_id: p.agent_id.clone(),
        agent_version: if p.agent_version.is_empty() {
            None
        } else {
            Some(p.agent_version.clone())
        },
        toolchain_digest: if p.toolchain_digest.is_empty() {
            None
        } else {
            Some(p.toolchain_digest.clone())
        },
        env_fingerprint: if p.env_fingerprint.is_empty() {
            None
        } else {
            Some(p.env_fingerprint.clone())
        },
        evidence: p
            .evidence
            .iter()
            .map(|e| {
                let revision_id = match &e.revision_id {
                    Some(id) if !id.hash.is_empty() => {
                        let bytes: [u8; 32] = id
                            .hash
                            .as_slice()
                            .try_into()
                            .map_err(|_| "invalid evidence revision_id")?;
                        Some(ObjectId::from_bytes(bytes))
                    }
                    _ => None,
                };
                Ok(Evidence {
                    name: e.name.clone(),
                    status: e.status.clone(),
                    duration_ms: e.duration_ms,
                    artifact_refs: e.artifact_refs.clone(),
                    summary: if e.summary.is_empty() {
                        None
                    } else {
                        Some(e.summary.clone())
                    },
                    revision_id,
                    command: e.command.clone(),
                    exit_code: e.exit_code,
                    started_at_ms: e.started_at_ms,
                    ended_at_ms: e.ended_at_ms,
                    environment_digest: e.environment_digest.clone(),
                    runner_identity: e.runner_identity.clone(),
                    log_digest: e.log_digest.clone(),
                    artifact_digest: e.artifact_digest.clone(),
                    expires_at_ms: e.expires_at_ms,
                    trust_domain: e.trust_domain.clone(),
                    signature: e.signature.clone(),
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?,
    })
}

fn recipient_from_proto(r: &crate::proto::objects::CapsuleRecipient) -> CapsuleRecipient {
    CapsuleRecipient {
        recipient_id: r.recipient_id.clone(),
        key_id: r.key_id.clone(),
        algorithm: r.algorithm.clone(),
        ephemeral_public_key: r.ephemeral_public_key.clone(),
        encrypted_content_key: r.encrypted_content_key.clone(),
    }
}

fn verify_capsule_for_revision(capsule: &Capsule, revision_id: &ObjectId) -> (bool, String) {
    if capsule.revision_id != *revision_id {
        return (
            false,
            "capsule revision mismatch with requested revision".into(),
        );
    }

    if capsule.signatures.is_empty() {
        return (false, "no signatures".into());
    }

    let mut verified = 0usize;
    let mut parse_errors = 0usize;

    for sig in &capsule.signatures {
        let Ok(signer_bytes) = hex::decode(&sig.signer_id) else {
            parse_errors += 1;
            continue;
        };
        let Ok(public_key) = <[u8; 32]>::try_from(signer_bytes.as_slice()) else {
            parse_errors += 1;
            continue;
        };

        let mut candidate = capsule.clone();
        candidate.signatures = vec![sig.clone()];
        if matches!(verify_capsule(&candidate, &public_key), Ok(true)) {
            verified += 1;
        }
    }

    if verified > 0 {
        (
            true,
            format!(
                "{verified}/{} signature(s) verified cryptographically",
                capsule.signatures.len()
            ),
        )
    } else if parse_errors > 0 {
        (
            false,
            format!(
                "no valid signatures ({} malformed signer id(s))",
                parse_errors
            ),
        )
    } else {
        (false, "no valid signatures".into())
    }
}

fn principal_from_request<T>(request: &Request<T>) -> Option<String> {
    metadata_value(request, PRINCIPAL_METADATA_KEY)
}

fn capsule_for_principal(
    capsule: &Capsule,
    principal: Option<&str>,
    can_read_private: bool,
) -> Capsule {
    if capsule.encrypted_private.is_none() {
        return capsule.clone();
    }

    let authorized = principal.is_some_and(|principal| {
        capsule
            .recipients
            .iter()
            .any(|recipient| recipient.recipient_id == principal)
    });

    if authorized && can_read_private {
        return capsule.clone();
    }

    let mut redacted = capsule.clone();
    redacted.encrypted_private = None;
    redacted.recipients.clear();
    redacted
}

#[tonic::async_trait]
impl CapsuleService for CapsuleServer {
    async fn create(
        &self,
        request: Request<CreateCapsuleRequest>,
    ) -> Result<Response<CreateCapsuleResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::CreateCapsule, None)?;
        let req = request.into_inner();

        let rev_id_msg = req
            .revision_id
            .ok_or_else(|| Status::invalid_argument("missing revision_id"))?;
        let arr: [u8; 32] = rev_id_msg
            .hash
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("invalid ObjectId"))?;
        let revision_id = ObjectId::from_bytes(arr);

        let public_fields = req
            .public_fields
            .as_ref()
            .map(public_from_proto)
            .transpose()
            .map_err(Status::invalid_argument)?
            .unwrap_or(CapsulePublic {
                agent_id: String::new(),
                agent_version: None,
                toolchain_digest: None,
                env_fingerprint: None,
                evidence: vec![],
            });
        let recipients = req
            .recipients
            .iter()
            .map(recipient_from_proto)
            .collect::<Vec<_>>();
        if !req.private_data.is_empty() && recipients.is_empty() {
            return Err(Status::invalid_argument(
                "private_data requires at least one capsule recipient envelope",
            ));
        }

        let capsule = Capsule {
            revision_id,
            public_fields,
            encrypted_private: if req.private_data.is_empty() {
                None
            } else {
                Some(req.private_data)
            },
            encryption: req.encryption,
            key_id: if req.key_id.is_empty() {
                None
            } else {
                Some(req.key_id)
            },
            recipients,
            signatures: vec![],
        };

        let store = self.store.write().await;
        let obj_id = store
            .store_object(&Object::Capsule(capsule.clone()))
            .map_err(|e| Status::internal(e.to_string()))?;
        store
            .set_ref(&format!("capsules/{}", revision_id.to_hex()), &obj_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        store
            .set_ref(
                &format!("capsules/by-revision/{}", revision_id.to_hex()),
                &obj_id,
            )
            .map_err(|e| Status::internal(e.to_string()))?;
        store
            .set_ref(
                &format!("capsules/by-revision/{}", &revision_id.to_hex()[..16]),
                &obj_id,
            )
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateCapsuleResponse {
            capsule: Some(capsule_to_proto(&capsule)),
        }))
    }

    async fn get(
        &self,
        request: Request<GetCapsuleRequest>,
    ) -> Result<Response<GetCapsuleResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::ReadCapsule, None)?;
        let can_read_private =
            self.security
                .allows(&request, AuthorizationAction::ReadPrivateCapsule, None);
        let principal = principal_from_request(&request);
        let req = request.into_inner();
        let rev_id_msg = req
            .revision_id
            .ok_or_else(|| Status::invalid_argument("missing revision_id"))?;
        let arr: [u8; 32] = rev_id_msg
            .hash
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("invalid ObjectId"))?;
        let revision_id = ObjectId::from_bytes(arr);

        let store = self.store.read().await;
        let obj_id = store
            .get_ref(&format!("capsules/{}", revision_id.to_hex()))
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("capsule not found"))?;

        match store
            .load_object(&obj_id)
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Object::Capsule(c) => Ok(Response::new(GetCapsuleResponse {
                capsule: Some(capsule_to_proto(&capsule_for_principal(
                    &c,
                    principal.as_deref(),
                    can_read_private,
                ))),
            })),
            _ => Err(Status::internal("object is not a capsule")),
        }
    }

    async fn verify(
        &self,
        request: Request<VerifyCapsuleRequest>,
    ) -> Result<Response<VerifyCapsuleResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::VerifyCapsule, None)?;
        let req = request.into_inner();
        let rev_id_msg = req
            .revision_id
            .ok_or_else(|| Status::invalid_argument("missing revision_id"))?;
        let arr: [u8; 32] = rev_id_msg
            .hash
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("invalid ObjectId"))?;
        let revision_id = ObjectId::from_bytes(arr);

        let store = self.store.read().await;
        let obj_id = match store
            .get_ref(&format!("capsules/{}", revision_id.to_hex()))
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Some(id) => id,
            None => {
                return Ok(Response::new(VerifyCapsuleResponse {
                    valid: false,
                    message: "capsule not found".into(),
                }));
            }
        };

        match store
            .load_object(&obj_id)
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Object::Capsule(c) => {
                let (valid, message) = verify_capsule_for_revision(&c, &revision_id);
                Ok(Response::new(VerifyCapsuleResponse { valid, message }))
            }
            _ => Ok(Response::new(VerifyCapsuleResponse {
                valid: false,
                message: "object is not a capsule".into(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{capsule_for_principal, verify_capsule_for_revision};
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;
    use claw_core::types::{Capsule, CapsulePublic, CapsuleRecipient};
    use claw_crypto::capsule::build_capsule;
    use claw_crypto::keypair::KeyPair;

    fn public_fields() -> CapsulePublic {
        CapsulePublic {
            agent_id: "agent-a".to_string(),
            agent_version: Some("1.0.0".to_string()),
            toolchain_digest: None,
            env_fingerprint: None,
            evidence: vec![],
        }
    }

    #[test]
    fn verify_capsule_for_revision_accepts_valid_signature() {
        let keypair = KeyPair::generate();
        let revision_id = content_hash(TypeTag::Revision, b"revision-a");
        let capsule = build_capsule(&revision_id, public_fields(), None, None, &keypair).unwrap();

        let (valid, _) = verify_capsule_for_revision(&capsule, &revision_id);
        assert!(valid);
    }

    #[test]
    fn verify_capsule_for_revision_rejects_tampering() {
        let keypair = KeyPair::generate();
        let revision_id = content_hash(TypeTag::Revision, b"revision-b");
        let mut capsule =
            build_capsule(&revision_id, public_fields(), None, None, &keypair).unwrap();
        capsule.public_fields.agent_id = "tampered".to_string();

        let (valid, _) = verify_capsule_for_revision(&capsule, &revision_id);
        assert!(!valid);
    }

    #[test]
    fn verify_capsule_for_revision_rejects_revision_mismatch() {
        let keypair = KeyPair::generate();
        let revision_id = content_hash(TypeTag::Revision, b"revision-c");
        let other_revision_id = content_hash(TypeTag::Revision, b"revision-d");
        let capsule = build_capsule(&revision_id, public_fields(), None, None, &keypair).unwrap();

        let (valid, message) = verify_capsule_for_revision(&capsule, &other_revision_id);
        assert!(!valid);
        assert!(message.contains("mismatch"));
    }

    fn recipient_capsule() -> Capsule {
        Capsule {
            revision_id: content_hash(TypeTag::Revision, b"revision-recipient"),
            public_fields: public_fields(),
            encrypted_private: Some(b"ciphertext".to_vec()),
            encryption: "xchacha20poly1305+recipient-envelope-v1".to_string(),
            key_id: None,
            recipients: vec![CapsuleRecipient {
                recipient_id: "runner-a".to_string(),
                key_id: "key-1".to_string(),
                algorithm: "x25519-blake3-xchacha20poly1305".to_string(),
                ephemeral_public_key: vec![1, 2, 3],
                encrypted_content_key: vec![4, 5, 6],
            }],
            signatures: vec![],
        }
    }

    #[test]
    fn capsule_private_fields_are_redacted_for_non_recipient_principals() {
        let capsule = recipient_capsule();

        let redacted = capsule_for_principal(&capsule, Some("runner-b"), true);

        assert!(redacted.encrypted_private.is_none());
        assert!(redacted.recipients.is_empty());
        assert_eq!(
            redacted.public_fields.agent_id,
            capsule.public_fields.agent_id
        );
    }

    #[test]
    fn capsule_private_fields_are_redacted_for_recipient_without_private_read_scope() {
        let capsule = recipient_capsule();

        let redacted = capsule_for_principal(&capsule, Some("runner-a"), false);

        assert!(redacted.encrypted_private.is_none());
        assert!(redacted.recipients.is_empty());
    }

    #[test]
    fn capsule_private_fields_are_kept_for_authorized_recipient_principal() {
        let capsule = recipient_capsule();

        let visible = capsule_for_principal(&capsule, Some("runner-a"), true);

        assert_eq!(visible.encrypted_private, capsule.encrypted_private);
        assert_eq!(visible.recipients.len(), 1);
    }

    #[test]
    fn capsule_private_fields_without_recipients_are_redacted() {
        let mut capsule = recipient_capsule();
        capsule.recipients.clear();

        let redacted = capsule_for_principal(&capsule, Some("runner-a"), true);

        assert!(redacted.encrypted_private.is_none());
        assert!(redacted.recipients.is_empty());
        assert_eq!(
            redacted.public_fields.agent_id,
            capsule.public_fields.agent_id
        );
    }
}

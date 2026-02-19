use std::sync::Arc;

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Capsule, CapsulePublic, Evidence};
use claw_crypto::capsule::verify_capsule;
use claw_store::ClawStore;

use crate::proto::capsule::capsule_service_server::CapsuleService;
use crate::proto::capsule::*;

pub struct CapsuleServer {
    store: Arc<RwLock<ClawStore>>,
}

impl CapsuleServer {
    pub fn new(store: Arc<RwLock<ClawStore>>) -> Self {
        Self { store }
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
    }
}

fn public_from_proto(p: &crate::proto::objects::CapsulePublic) -> CapsulePublic {
    CapsulePublic {
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
            .map(|e| Evidence {
                name: e.name.clone(),
                status: e.status.clone(),
                duration_ms: e.duration_ms,
                artifact_refs: e.artifact_refs.clone(),
                summary: if e.summary.is_empty() {
                    None
                } else {
                    Some(e.summary.clone())
                },
            })
            .collect(),
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

#[tonic::async_trait]
impl CapsuleService for CapsuleServer {
    async fn create(
        &self,
        request: Request<CreateCapsuleRequest>,
    ) -> Result<Response<CreateCapsuleResponse>, Status> {
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

        let public_fields =
            req.public_fields
                .as_ref()
                .map(public_from_proto)
                .unwrap_or(CapsulePublic {
                    agent_id: String::new(),
                    agent_version: None,
                    toolchain_digest: None,
                    env_fingerprint: None,
                    evidence: vec![],
                });

        let capsule = Capsule {
            revision_id,
            public_fields,
            encrypted_private: if req.private_data.is_empty() {
                None
            } else {
                Some(req.private_data)
            },
            encryption: String::new(),
            key_id: None,
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
                capsule: Some(capsule_to_proto(&c)),
            })),
            _ => Err(Status::internal("object is not a capsule")),
        }
    }

    async fn verify(
        &self,
        request: Request<VerifyCapsuleRequest>,
    ) -> Result<Response<VerifyCapsuleResponse>, Status> {
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
    use super::verify_capsule_for_revision;
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;
    use claw_core::types::CapsulePublic;
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
}

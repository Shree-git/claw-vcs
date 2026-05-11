use claw_core::types::{Capsule, Policy, Visibility};

use crate::context::PolicyContext;
use crate::PolicyError;

/// Enforce capsule visibility requirements declared by a policy.
pub fn check_visibility(
    policy: &Policy,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    match policy.visibility {
        Visibility::Public => Ok(()),
        Visibility::Private => {
            require_encrypted_private("private", capsule)?;
            Ok(())
        }
        Visibility::EncryptedMetadataRequired => {
            require_encrypted_private("encrypted-metadata-required", capsule)?;

            let key_id = capsule
                .key_id
                .as_deref()
                .map(str::trim)
                .filter(|key_id| !key_id.is_empty())
                .ok_or_else(|| {
                    PolicyError::Violation(
                        "encrypted-metadata-required policy requires capsule key_id".into(),
                    )
                })?;

            if !context
                .signer_key_ids
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(key_id))
            {
                return Err(PolicyError::VisibilityDenied);
            }

            Ok(())
        }
    }
}

fn require_encrypted_private(visibility: &str, capsule: &Capsule) -> Result<(), PolicyError> {
    match capsule.encrypted_private.as_deref() {
        Some(bytes) if !bytes.is_empty() => {
            if capsule.encryption.trim().is_empty() {
                return Err(PolicyError::Violation(format!(
                    "{visibility} policy requires encryption metadata"
                )));
            }
            Ok(())
        }
        _ => Err(PolicyError::Violation(format!(
            "{visibility} policy requires encrypted private fields"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;
    use claw_core::types::{CapsulePublic, EvidencePolicy};

    use super::*;

    fn policy(visibility: Visibility) -> Policy {
        Policy {
            policy_id: "default".to_string(),
            required_checks: vec![],
            required_reviewers: vec![],
            sensitive_paths: vec![],
            quarantine_lane: false,
            min_trust_score: None,
            visibility,
            authorized_recipients: vec![],
            revoked_recipients: vec![],
            evidence_policy: EvidencePolicy::default(),
        }
    }

    fn capsule() -> Capsule {
        Capsule {
            revision_id: content_hash(TypeTag::Revision, b"rev"),
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
        }
    }

    #[test]
    fn encrypted_metadata_required_requires_encrypted_private_key_id_and_authorized_key() {
        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = "xchacha20poly1305".to_string();
        c.key_id = Some("TEAM-KMS-1".to_string());
        let context = PolicyContext {
            signer_key_ids: vec!["team-kms-1".to_string()],
            ..PolicyContext::default()
        };

        assert!(
            check_visibility(&policy(Visibility::EncryptedMetadataRequired), &c, &context).is_ok()
        );
    }

    #[test]
    fn encrypted_metadata_required_rejects_missing_authorized_key() {
        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = "xchacha20poly1305".to_string();
        c.key_id = Some("team-kms-1".to_string());

        let err = check_visibility(
            &policy(Visibility::EncryptedMetadataRequired),
            &c,
            &PolicyContext::default(),
        )
        .unwrap_err();
        assert!(matches!(err, PolicyError::VisibilityDenied));
    }

    #[test]
    fn private_rejects_empty_encrypted_private_payload() {
        let mut c = capsule();
        c.encrypted_private = Some(vec![]);
        c.encryption = "xchacha20poly1305".to_string();

        let err = check_visibility(&policy(Visibility::Private), &c, &PolicyContext::default())
            .unwrap_err();
        assert!(matches!(err, PolicyError::Violation(_)));
    }
}

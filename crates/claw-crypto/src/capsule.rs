use claw_core::id::ObjectId;
use claw_core::types::{
    Capsule, CapsulePublic, CapsuleRecipient, CapsuleSignature, CAPSULE_PRIVATE_ENCRYPTION,
    CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION,
};

use crate::encrypt;
use crate::keypair::KeyPair;
use crate::recipient::{random_content_key, wrap_content_key_for_recipients, RecipientPublicKey};
use crate::sign;
use crate::CryptoError;

/// Builds and signs a capsule, optionally encrypting private fields with a shared key.
pub fn build_capsule(
    revision_id: &ObjectId,
    public_fields: CapsulePublic,
    private_data: Option<&[u8]>,
    encryption_key: Option<&[u8; 32]>,
    signing_keypair: &KeyPair,
) -> Result<Capsule, CryptoError> {
    // Encrypt private data if provided
    let encrypted_private = match (private_data, encryption_key) {
        (Some(data), Some(key)) => Some(encrypt::encrypt(key, data)?),
        _ => None,
    };

    let encryption = if encryption_key.is_some() {
        CAPSULE_PRIVATE_ENCRYPTION.to_string()
    } else {
        String::new()
    };

    let mut capsule = Capsule {
        revision_id: *revision_id,
        public_fields,
        encrypted_private,
        encryption,
        key_id: None,
        recipients: vec![],
        signatures: vec![],
    };
    append_capsule_signature(&mut capsule, signing_keypair)?;
    Ok(capsule)
}

/// Builds and signs a capsule whose private fields are encrypted for named recipients.
pub fn build_capsule_for_recipients(
    revision_id: &ObjectId,
    public_fields: CapsulePublic,
    private_data: &[u8],
    recipients: &[RecipientPublicKey],
    signing_keypair: &KeyPair,
) -> Result<Capsule, CryptoError> {
    if recipients.is_empty() {
        return Err(CryptoError::EncryptionFailed(
            "recipient capsule requires at least one recipient".into(),
        ));
    }

    let content_key = random_content_key();
    let encrypted_private = Some(encrypt::encrypt(&content_key, private_data)?);
    let recipient_envelopes = wrap_content_key_for_recipients(&content_key, recipients)?;

    let mut capsule = Capsule {
        revision_id: *revision_id,
        public_fields,
        encrypted_private,
        encryption: CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION.to_string(),
        key_id: None,
        recipients: recipient_envelopes,
        signatures: vec![],
    };
    append_capsule_signature(&mut capsule, signing_keypair)?;
    Ok(capsule)
}

/// Verifies the first signature on a capsule against an Ed25519 public key.
pub fn verify_capsule(capsule: &Capsule, public_key: &[u8; 32]) -> Result<bool, CryptoError> {
    let sig = capsule
        .signatures
        .first()
        .ok_or_else(|| CryptoError::VerificationFailed("no signature".into()))?;

    let sign_payload = capsule_signing_payload(capsule)?;

    crate::verify::verify(public_key, &sign_payload, &sig.signature)
}

/// Appends a signature for `signing_keypair` unless that signer is already present.
pub fn append_capsule_signature(
    capsule: &mut Capsule,
    signing_keypair: &KeyPair,
) -> Result<(), CryptoError> {
    let sign_payload = capsule_signing_payload(capsule)?;
    let sig = sign::sign(signing_keypair, &sign_payload);
    let signer_id = hex::encode(sig.signer_id);

    if capsule
        .signatures
        .iter()
        .any(|existing| existing.signer_id.eq_ignore_ascii_case(&signer_id))
    {
        return Ok(());
    }

    capsule.signatures.push(CapsuleSignature {
        signer_id,
        signature: sig.signature,
    });
    Ok(())
}

/// Returns the canonical bytes signed by capsule signatures.
pub fn capsule_signing_payload(capsule: &Capsule) -> Result<Vec<u8>, CryptoError> {
    signing_payload(
        &capsule.revision_id,
        &capsule.public_fields,
        capsule.encrypted_private.as_deref(),
        &capsule.recipients,
    )
    .map_err(|e| CryptoError::VerificationFailed(e.to_string()))
}

fn signing_payload(
    revision_id: &ObjectId,
    public_fields: &CapsulePublic,
    encrypted_private: Option<&[u8]>,
    recipients: &[CapsuleRecipient],
) -> Result<Vec<u8>, serde_json::Error> {
    let public_bytes = serde_json::to_vec(public_fields)?;
    let public_hash = blake3::hash(&public_bytes);

    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(revision_id.as_bytes());
    sign_payload.extend_from_slice(public_hash.as_bytes());
    if let Some(enc) = encrypted_private {
        let enc_hash = blake3::hash(enc);
        sign_payload.extend_from_slice(enc_hash.as_bytes());
    }
    if !recipients.is_empty() {
        let recipients_bytes = serde_json::to_vec(recipients)?;
        let recipients_hash = blake3::hash(&recipients_bytes);
        sign_payload.extend_from_slice(recipients_hash.as_bytes());
    }
    Ok(sign_payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;

    fn test_public() -> CapsulePublic {
        CapsulePublic {
            agent_id: "test-agent".to_string(),
            agent_version: None,
            toolchain_digest: None,
            env_fingerprint: None,
            evidence: vec![],
        }
    }

    #[test]
    fn capsule_sign_and_verify() {
        let kp = KeyPair::generate();
        let rev_id = content_hash(TypeTag::Revision, b"test revision");

        let capsule = build_capsule(&rev_id, test_public(), None, None, &kp).unwrap();
        let pk = kp.public_key_bytes();
        assert!(verify_capsule(&capsule, &pk).unwrap());
    }

    #[test]
    fn capsule_tamper_detection() {
        let kp = KeyPair::generate();
        let rev_id = content_hash(TypeTag::Revision, b"test revision");

        let mut capsule = build_capsule(&rev_id, test_public(), None, None, &kp).unwrap();
        // Tamper with the public fields
        capsule.public_fields.agent_id = "TAMPERED".to_string();
        let pk = kp.public_key_bytes();
        assert!(!verify_capsule(&capsule, &pk).unwrap());
    }

    #[test]
    fn capsule_with_encrypted_private() {
        let kp = KeyPair::generate();
        let rev_id = content_hash(TypeTag::Revision, b"test");
        let enc_key = [99u8; 32];

        let private_data = b"secret private data";
        let capsule = build_capsule(
            &rev_id,
            test_public(),
            Some(private_data),
            Some(&enc_key),
            &kp,
        )
        .unwrap();

        assert!(capsule.encrypted_private.is_some());
        assert_eq!(capsule.encryption, "xchacha20poly1305");
        let pk = kp.public_key_bytes();
        assert!(verify_capsule(&capsule, &pk).unwrap());

        // Can decrypt
        let decrypted =
            encrypt::decrypt(&enc_key, capsule.encrypted_private.as_ref().unwrap()).unwrap();
        assert_eq!(decrypted, private_data);

        // Wrong key can't decrypt
        let wrong_key = [100u8; 32];
        assert!(encrypt::decrypt(&wrong_key, capsule.encrypted_private.as_ref().unwrap()).is_err());
    }

    #[test]
    fn append_capsule_signature_adds_second_signer() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let rev_id = content_hash(TypeTag::Revision, b"test revision");

        let mut capsule = build_capsule(&rev_id, test_public(), None, None, &kp1).unwrap();
        append_capsule_signature(&mut capsule, &kp2).unwrap();

        assert_eq!(capsule.signatures.len(), 2);
    }
}

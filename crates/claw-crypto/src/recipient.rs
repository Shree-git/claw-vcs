/// Algorithm identifier stored on recipient envelopes.
pub use claw_core::types::RECIPIENT_ENVELOPE_ALGORITHM;

use claw_core::types::{Capsule, CapsuleRecipient};
use rand::RngCore;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

use crate::encrypt;
use crate::CryptoError;

/// Public recipient key metadata used to wrap capsule private-field keys.
#[derive(Debug, Clone)]
pub struct RecipientPublicKey {
    /// Stable recipient identity used by policy and CLI lookups.
    pub recipient_id: String,
    /// Key identifier for this recipient key.
    pub key_id: String,
    /// X25519 public key bytes.
    pub public_key: [u8; 32],
}

/// Derives the X25519 public key for a recipient secret key.
pub fn recipient_public_key(secret_key: &[u8; 32]) -> [u8; 32] {
    let secret = StaticSecret::from(*secret_key);
    PublicKey::from(&secret).to_bytes()
}

/// Generates a random 256-bit content encryption key.
pub fn random_content_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Wraps a content key for each recipient using ephemeral X25519 envelopes.
pub fn wrap_content_key_for_recipients(
    content_key: &[u8; 32],
    recipients: &[RecipientPublicKey],
) -> Result<Vec<CapsuleRecipient>, CryptoError> {
    recipients
        .iter()
        .map(|recipient| {
            let ephemeral = EphemeralSecret::random_from_rng(rand::thread_rng());
            let ephemeral_public = PublicKey::from(&ephemeral);
            let recipient_public = PublicKey::from(recipient.public_key);
            let shared = ephemeral.diffie_hellman(&recipient_public);
            let wrapping_key = derive_wrapping_key(shared.as_bytes());
            let encrypted_content_key = encrypt::encrypt(&wrapping_key, content_key)?;

            Ok(CapsuleRecipient {
                recipient_id: recipient.recipient_id.clone(),
                key_id: recipient.key_id.clone(),
                algorithm: RECIPIENT_ENVELOPE_ALGORITHM.to_string(),
                ephemeral_public_key: ephemeral_public.to_bytes().to_vec(),
                encrypted_content_key,
            })
        })
        .collect()
}

/// Decrypts a recipient envelope into the original content key.
pub fn unwrap_content_key(
    recipient_secret_key: &[u8; 32],
    envelope: &CapsuleRecipient,
) -> Result<[u8; 32], CryptoError> {
    if envelope.algorithm != RECIPIENT_ENVELOPE_ALGORITHM {
        return Err(CryptoError::DecryptionFailed(format!(
            "unsupported recipient envelope algorithm: {}",
            envelope.algorithm
        )));
    }
    let ephemeral_public: [u8; 32] = envelope
        .ephemeral_public_key
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::DecryptionFailed("invalid ephemeral public key".into()))?;
    let secret = StaticSecret::from(*recipient_secret_key);
    let shared = secret.diffie_hellman(&PublicKey::from(ephemeral_public));
    let wrapping_key = derive_wrapping_key(shared.as_bytes());
    let content_key = encrypt::decrypt(&wrapping_key, &envelope.encrypted_content_key)?;
    content_key
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::DecryptionFailed("invalid content key length".into()))
}

/// Decrypts capsule private fields for a matching recipient identity and secret key.
pub fn decrypt_capsule_private_for_recipient(
    capsule: &Capsule,
    recipient_id: &str,
    recipient_secret_key: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    let encrypted_private = capsule.encrypted_private.as_deref().ok_or_else(|| {
        CryptoError::DecryptionFailed("capsule has no encrypted private fields".into())
    })?;
    let envelope = capsule
        .recipients
        .iter()
        .find(|recipient| recipient.recipient_id.eq_ignore_ascii_case(recipient_id))
        .ok_or_else(|| CryptoError::DecryptionFailed("recipient envelope not found".into()))?;
    let content_key = unwrap_content_key(recipient_secret_key, envelope)?;
    encrypt::decrypt(&content_key, encrypted_private)
}

fn derive_wrapping_key(shared_secret: &[u8; 32]) -> [u8; 32] {
    blake3::derive_key(
        "claw-vcs recipient envelope content key wrap v1",
        shared_secret,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_unwraps_content_key() {
        let recipient_secret = [9u8; 32];
        let recipient_public = recipient_public_key(&recipient_secret);
        let content_key = [42u8; 32];

        let envelopes = wrap_content_key_for_recipients(
            &content_key,
            &[RecipientPublicKey {
                recipient_id: "security".to_string(),
                key_id: "security-key".to_string(),
                public_key: recipient_public,
            }],
        )
        .unwrap();

        let unwrapped = unwrap_content_key(&recipient_secret, &envelopes[0]).unwrap();
        assert_eq!(unwrapped, content_key);
    }
}

use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// Legacy symmetric encryption marker for capsule private fields.
pub const CAPSULE_PRIVATE_ENCRYPTION: &str = "xchacha20poly1305";
/// Recipient-envelope encryption marker for capsule private fields.
pub const CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION: &str = "xchacha20poly1305+recipient-envelope-v1";
/// Recipient envelope key agreement and content encryption algorithm.
pub const RECIPIENT_ENVELOPE_ALGORITHM: &str = "x25519-blake3-xchacha20poly1305";

/// Evidence claim attached to a capsule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Check or evidence name, such as `test` or `lint`.
    pub name: String,
    /// Producer-reported result status.
    pub status: String,
    /// Producer-reported duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Referenced artifacts or logs for this evidence.
    #[serde(default)]
    pub artifact_refs: Vec<String>,
    /// Optional human summary of the evidence result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Revision ID the evidence was produced for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<ObjectId>,
    /// Command that produced the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Process exit code for command-based evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Evidence start time in Unix epoch milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<u64>,
    /// Evidence end time in Unix epoch milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at_ms: Option<u64>,
    /// Digest of the environment or toolchain used to produce evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_digest: Option<String>,
    /// Identity of the runner that produced the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_identity: Option<String>,
    /// Digest of the evidence log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_digest: Option<String>,
    /// Digest of an evidence artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_digest: Option<String>,
    /// Expiration time in Unix epoch milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
    /// Trust domain in which the evidence was produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_domain: Option<String>,
    /// Optional detached signature over the evidence claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

/// Public, policy-visible capsule fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsulePublic {
    /// Agent identity that produced the capsule.
    pub agent_id: String,
    /// Optional producer version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
    /// Optional digest of the producer toolchain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain_digest: Option<String>,
    /// Optional fingerprint of the producer environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_fingerprint: Option<String>,
    /// Evidence claims included in the capsule.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

/// Signature over the canonical capsule claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleSignature {
    /// Key or identity that produced the signature.
    pub signer_id: String,
    /// Raw signature bytes.
    pub signature: Vec<u8>,
}

/// Per-recipient envelope used to decrypt private capsule fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleRecipient {
    /// Authorized recipient identity.
    pub recipient_id: String,
    /// Recipient public key identifier.
    pub key_id: String,
    /// Envelope algorithm identifier.
    pub algorithm: String,
    /// Ephemeral public key used for key agreement.
    pub ephemeral_public_key: Vec<u8>,
    /// Content key encrypted for this recipient.
    pub encrypted_content_key: Vec<u8>,
}

/// Signed provenance capsule for a specific revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capsule {
    /// Revision this capsule claims evidence for.
    pub revision_id: ObjectId,
    /// Public fields visible to policy evaluation.
    pub public_fields: CapsulePublic,
    /// Encrypted private capsule payload, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_private: Option<Vec<u8>>,
    /// Encryption algorithm marker for `encrypted_private`.
    #[serde(default)]
    pub encryption: String,
    /// Optional symmetric key identifier for legacy encrypted fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    /// Recipient envelopes that can decrypt the private payload.
    #[serde(default)]
    pub recipients: Vec<CapsuleRecipient>,
    /// Detached signatures over the capsule claim.
    #[serde(default)]
    pub signatures: Vec<CapsuleSignature>,
}

//! Cryptographic helpers for Claw VCS capsules and agent identities.
//!
//! This crate provides Ed25519 signing, capsule signature verification,
//! XChaCha20-Poly1305 private-field encryption, and key derivation helpers.
//! It does not decide policy trust; callers must still decide which keys and
//! evidence are acceptable.
//!
//! # Example
//!
//! ```rust
//! use claw_core::hash::content_hash;
//! use claw_core::object::TypeTag;
//! use claw_core::types::{CapsulePublic, Evidence};
//! use claw_crypto::capsule::{build_capsule, verify_capsule};
//! use claw_crypto::keypair::KeyPair;
//!
//! let keypair = KeyPair::from_bytes(&[7; 32])?;
//! let revision_id = content_hash(TypeTag::Revision, b"revision payload");
//! let encryption_key = [42; 32];
//!
//! let public_fields = CapsulePublic {
//!     agent_id: "agent-1".to_string(),
//!     agent_version: Some("1.0.0".to_string()),
//!     toolchain_digest: None,
//!     env_fingerprint: None,
//!     evidence: vec![Evidence {
//!         name: "unit-tests".to_string(),
//!         status: "pass".to_string(),
//!         duration_ms: 120,
//!         artifact_refs: vec![],
//!         summary: None,
//!         revision_id: Some(revision_id),
//!         command: Some("cargo test".to_string()),
//!         exit_code: Some(0),
//!         started_at_ms: Some(1_000),
//!         ended_at_ms: Some(1_100),
//!         environment_digest: Some("sha256:env".to_string()),
//!         runner_identity: Some("runner-a".to_string()),
//!         log_digest: Some("sha256:log".to_string()),
//!         artifact_digest: None,
//!         expires_at_ms: Some(2_000),
//!         trust_domain: Some("ci".to_string()),
//!         signature: None,
//!     }],
//! };
//!
//! let capsule = build_capsule(
//!     &revision_id,
//!     public_fields,
//!     Some(b"private build metadata"),
//!     Some(&encryption_key),
//!     &keypair,
//! )?;
//!
//! assert!(verify_capsule(&capsule, &keypair.public_key_bytes())?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
#![deny(missing_docs)]

/// Capsule construction, signing, and signature verification.
pub mod capsule;
/// Symmetric private-field encryption helpers.
pub mod encrypt;
/// Error types returned by crypto operations.
pub mod error;
/// BLAKE3-based key derivation helpers.
pub mod kdf;
/// Ed25519 keypair creation and serialization.
pub mod keypair;
/// Recipient envelope encryption helpers for private capsule fields.
pub mod recipient;
/// Ed25519 signing helpers.
pub mod sign;
/// Ed25519 signature verification helpers.
pub mod verify;

pub use error::CryptoError;

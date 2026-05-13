//! Policy evaluation for Claw VCS revisions and capsules.
//!
//! `claw-policy` verifies required checks, reviewer/signature requirements,
//! sensitive path behavior, visibility semantics, trust-score thresholds, and
//! optional external policy plugins. Evaluation is fail-closed for missing
//! required evidence.
//!
#![deny(missing_docs)]

//! # Example
//!
//! ```rust
//! use claw_core::hash::content_hash;
//! use claw_core::object::TypeTag;
//! use claw_core::types::{Capsule, CapsulePublic, Evidence, EvidencePolicy, Policy, Visibility};
//! use claw_policy::checks::verify_required_checks;
//! use claw_policy::PolicyContext;
//!
//! let policy = Policy {
//!     policy_id: "release".to_string(),
//!     required_checks: vec!["unit-tests".to_string()],
//!     required_reviewers: vec![],
//!     sensitive_paths: vec!["src/secrets/".to_string()],
//!     quarantine_lane: false,
//!     min_trust_score: None,
//!     visibility: Visibility::Public,
//!     authorized_recipients: vec![],
//!     revoked_recipients: vec![],
//!     evidence_policy: EvidencePolicy::default(),
//! };
//!
//! let revision_id = content_hash(TypeTag::Revision, b"revision");
//! let capsule = Capsule {
//!     revision_id,
//!     public_fields: CapsulePublic {
//!         agent_id: "agent-1".to_string(),
//!         agent_version: None,
//!         toolchain_digest: None,
//!         env_fingerprint: None,
//!         evidence: vec![Evidence {
//!             name: "unit-tests".to_string(),
//!             status: "pass".to_string(),
//!             duration_ms: 31,
//!             artifact_refs: vec![],
//!             summary: None,
//!             revision_id: Some(revision_id),
//!             command: Some("cargo test".to_string()),
//!             exit_code: Some(0),
//!             started_at_ms: Some(1_000),
//!             ended_at_ms: Some(1_100),
//!             environment_digest: Some("sha256:env".to_string()),
//!             runner_identity: Some("runner-a".to_string()),
//!             log_digest: Some("sha256:log".to_string()),
//!             artifact_digest: None,
//!             expires_at_ms: Some(2_000),
//!             trust_domain: Some("ci".to_string()),
//!             signature: None,
//!         }],
//!     },
//!     encrypted_private: None,
//!     encryption: String::new(),
//!     key_id: None,
//!     recipients: vec![],
//!     signatures: vec![],
//! };
//!
//! verify_required_checks(&policy, &capsule)?;
//!
//! let context = PolicyContext {
//!     touched_paths: vec!["src/secrets/token.txt".to_string()],
//!     ..PolicyContext::default()
//! };
//! assert_eq!(
//!     context.touched_sensitive_path(&policy.sensitive_paths).as_deref(),
//!     Some("src/secrets/token.txt")
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
/// Check-level policy validation helpers.
pub mod checks;
/// Evaluation inputs that are derived outside the capsule.
pub mod context;
/// Typed policy evaluation errors.
pub mod error;
/// End-to-end policy evaluator.
pub mod evaluator;
/// External policy plugin runtime.
pub mod plugin;
/// Capsule visibility enforcement.
pub mod visibility;

/// Policy evaluation context.
pub use context::PolicyContext;
/// Policy evaluation error.
pub use error::PolicyError;

use serde::{Deserialize, Serialize};

/// Visibility behavior enforced for capsule private metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    /// No private-field encryption requirement.
    Public,
    /// Capsule private fields must be encrypted.
    Private,
    /// Capsule private fields must be encrypted.
    EncryptedMetadataRequired,
}

/// Repository-stored policy that gates evidence, visibility, and integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Stable policy identifier.
    pub policy_id: String,
    /// Evidence check names that must be present and passing.
    #[serde(default)]
    pub required_checks: Vec<String>,
    /// Reviewer or signer identities required by policy.
    #[serde(default)]
    pub required_reviewers: Vec<String>,
    /// Paths that trigger sensitive-path behavior.
    #[serde(default)]
    pub sensitive_paths: Vec<String>,
    /// Whether matching changes are routed to quarantine.
    #[serde(default)]
    pub quarantine_lane: bool,
    /// Minimum trust score required by this policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_trust_score: Option<String>,
    /// Visibility is used by the policy evaluator for capsule enforcement.
    pub visibility: Visibility,
    /// Recipient identities allowed to decrypt private capsule fields.
    #[serde(default)]
    pub authorized_recipients: Vec<String>,
    /// Recipient identities that must not appear on capsule envelopes.
    #[serde(default)]
    pub revoked_recipients: Vec<String>,
    /// Evidence freshness and integrity requirements.
    #[serde(default)]
    pub evidence_policy: EvidencePolicy,
}

/// Fine-grained requirements for accepting capsule evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidencePolicy {
    /// Require all default freshness checks.
    pub require_fresh_evidence: bool,
    /// Require evidence to reference the evaluated revision exactly.
    pub require_revision_match: bool,
    /// Require evidence timestamps to be newer than the revision timestamp.
    pub require_evidence_after_revision: bool,
    /// Require evidence to carry an expiration timestamp.
    pub require_expires_at: bool,
    /// Require a runner identity.
    pub require_runner_identity: bool,
    /// Require the producing command.
    pub require_command: bool,
    /// Require the producing process exit code.
    pub require_exit_code: bool,
    /// Require at least one log or artifact digest.
    pub require_log_or_artifact_digest: bool,
    /// Require an environment or toolchain digest.
    pub require_environment_digest: bool,
    /// Maximum acceptable evidence age in milliseconds.
    pub max_age_ms: Option<u64>,
    /// Runner identities trusted by this policy.
    pub trusted_runner_identities: Vec<String>,
}

impl Default for EvidencePolicy {
    fn default() -> Self {
        Self {
            require_fresh_evidence: false,
            require_revision_match: true,
            require_evidence_after_revision: true,
            require_expires_at: true,
            require_runner_identity: true,
            require_command: true,
            require_exit_code: true,
            require_log_or_artifact_digest: true,
            require_environment_digest: true,
            max_age_ms: Some(24 * 60 * 60 * 1_000),
            trusted_runner_identities: Vec::new(),
        }
    }
}

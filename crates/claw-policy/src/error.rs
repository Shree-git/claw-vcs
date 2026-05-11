use thiserror::Error;

/// Errors returned while evaluating policy gates or external plugins.
#[derive(Debug, Error)]
pub enum PolicyError {
    /// A general policy violation.
    #[error("policy violation: {0}")]
    Violation(String),
    /// A required check was missing or did not pass.
    #[error("missing required check: {0}")]
    MissingCheck(String),
    /// A required reviewer was not present in signer evidence.
    #[error("missing required reviewer: {0}")]
    MissingReviewer(String),
    /// Sensitive paths were touched without encrypted private capsule fields.
    #[error("sensitive path requires encrypted capsule data: {0}")]
    SensitivePathRequiresPrivate(String),
    /// Quarantine lane policy blocked automated integration.
    #[error("quarantine lane required: {0}")]
    QuarantineLane(String),
    /// A policy requires trust score evidence but none was supplied.
    #[error("missing trust score evidence")]
    MissingTrustScore,
    /// A configured `min_trust_score` value could not be parsed.
    #[error("invalid min_trust_score value: {0}")]
    InvalidTrustScore(String),
    /// The evaluated trust score is below the policy threshold.
    #[error("trust score {actual:.2} is below required threshold {required:.2}")]
    MinTrustScoreNotMet {
        /// Minimum trust score required by the policy.
        required: f32,
        /// Trust score supplied by the evaluation context.
        actual: f32,
    },
    /// Evidence freshness policy rejected a capsule evidence item.
    #[error("stale or incomplete evidence for {check}: {reason}")]
    StaleEvidence {
        /// Evidence check name.
        check: String,
        /// Freshness failure reason.
        reason: String,
    },
    /// A recipient envelope is required but missing or unauthorized.
    #[error("recipient authorization failed: {0}")]
    RecipientAuthorization(String),
    /// Visibility policy rejected the capsule.
    #[error("visibility denied")]
    VisibilityDenied,
    /// External plugin configuration is invalid.
    #[error("plugin config error: {0}")]
    PluginConfig(String),
    /// An external policy plugin could not be started.
    #[error("plugin spawn error ({plugin}): {reason}")]
    PluginSpawn {
        /// Plugin executable path or display name.
        plugin: String,
        /// Spawn failure reason.
        reason: String,
    },
    /// An external policy plugin violated the JSON protocol.
    #[error("plugin protocol error ({plugin}): {reason}")]
    PluginProtocol {
        /// Plugin executable path or display name.
        plugin: String,
        /// Protocol failure reason.
        reason: String,
    },
    /// An external policy plugin did not respond before the timeout.
    #[error("plugin timeout ({plugin}) during {phase} after {timeout_ms}ms")]
    PluginTimeout {
        /// Plugin executable path or display name.
        plugin: String,
        /// Protocol phase that timed out.
        phase: &'static str,
        /// Timeout threshold in milliseconds.
        timeout_ms: u64,
    },
    /// An external policy plugin explicitly denied the request.
    #[error("plugin denied policy ({plugin}): {reason}")]
    PluginDenied {
        /// Plugin executable path or display name.
        plugin: String,
        /// Denial reason supplied by the plugin.
        reason: String,
    },
}

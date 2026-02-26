use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("policy violation: {0}")]
    Violation(String),
    #[error("missing required check: {0}")]
    MissingCheck(String),
    #[error("missing required reviewer: {0}")]
    MissingReviewer(String),
    #[error("sensitive path requires encrypted capsule data: {0}")]
    SensitivePathRequiresPrivate(String),
    #[error("quarantine lane required: {0}")]
    QuarantineLane(String),
    #[error("missing trust score evidence")]
    MissingTrustScore,
    #[error("invalid min_trust_score value: {0}")]
    InvalidTrustScore(String),
    #[error("trust score {actual:.2} is below required threshold {required:.2}")]
    MinTrustScoreNotMet { required: f32, actual: f32 },
    #[error("visibility denied")]
    VisibilityDenied,
    #[error("plugin config error: {0}")]
    PluginConfig(String),
    #[error("plugin spawn error ({plugin}): {reason}")]
    PluginSpawn { plugin: String, reason: String },
    #[error("plugin protocol error ({plugin}): {reason}")]
    PluginProtocol { plugin: String, reason: String },
    #[error("plugin timeout ({plugin}) during {phase} after {timeout_ms}ms")]
    PluginTimeout {
        plugin: String,
        phase: &'static str,
        timeout_ms: u64,
    },
    #[error("plugin denied policy ({plugin}): {reason}")]
    PluginDenied { plugin: String, reason: String },
}

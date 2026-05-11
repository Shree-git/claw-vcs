use thiserror::Error;

/// Errors returned by patch codecs and codec lookup.
#[derive(Debug, Error)]
pub enum PatchError {
    /// A requested codec id is not registered.
    #[error("codec not found: {0}")]
    CodecNotFound(String),
    /// Patch application failed.
    #[error("apply failed: {0}")]
    ApplyFailed(String),
    /// Patch inversion failed.
    #[error("invert failed: {0}")]
    InvertFailed(String),
    /// Two patch streams overlap and cannot be commuted.
    #[error("commute failed: patches overlap")]
    CommuteFailed,
    /// Three-way merge could not produce a conflict-free result.
    #[error("merge3 failed: {0}")]
    Merge3Failed(String),
    /// A patch address could not be resolved for the target content.
    #[error("address resolution failed: {0}")]
    AddressResolutionFailed(String),
    /// JSON input could not be parsed by the structural JSON codec.
    #[error("invalid json: {0}")]
    InvalidJson(String),
}

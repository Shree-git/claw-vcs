use thiserror::Error;

/// Errors returned by Claw VCS cryptographic helpers.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Key generation failed.
    #[error("keypair generation failed: {0}")]
    KeyGeneration(String),
    /// Signing failed.
    #[error("signing failed: {0}")]
    SigningFailed(String),
    /// Signature verification failed or could not be evaluated.
    #[error("verification failed: {0}")]
    VerificationFailed(String),
    /// Private-field encryption failed.
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    /// Private-field decryption failed.
    #[error("decryption failed: {0}")]
    DecryptionFailed(String),
    /// Key material was malformed or unsupported.
    #[error("invalid key: {0}")]
    InvalidKey(String),
    /// File I/O failed while loading or storing key material.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Core object or serialization logic failed.
    #[error("core error: {0}")]
    Core(#[from] claw_core::CoreError),
}

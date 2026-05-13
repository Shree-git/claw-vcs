use thiserror::Error;

/// Errors returned by core object encoding, IDs, and schema conversion.
#[derive(Debug, Error)]
pub enum CoreError {
    /// COF bytes did not start with the expected magic prefix.
    #[error("invalid magic bytes")]
    InvalidMagic,
    /// COF bytes use a version this crate cannot read.
    #[error("unsupported COF version: {0}")]
    UnsupportedVersion(u8),
    /// COF bytes used an unknown object type tag.
    #[error("unknown type tag: {0}")]
    UnknownTypeTag(u8),
    /// CRC32 in the COF trailer did not match the decoded payload.
    #[error("CRC32 mismatch: expected {expected:#010x}, got {actual:#010x}")]
    Crc32Mismatch {
        /// CRC32 value recorded in the object.
        expected: u32,
        /// CRC32 computed from the decoded payload.
        actual: u32,
    },
    /// Decompression failed while decoding object bytes.
    #[error("decompression error: {0}")]
    Decompression(String),
    /// Compression failed while encoding object bytes.
    #[error("compression error: {0}")]
    Compression(String),
    /// Serialization to a stable wire representation failed.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// Deserialization from a stable wire representation failed.
    #[error("deserialization error: {0}")]
    Deserialization(String),
    /// Object ID text or bytes were malformed.
    #[error("invalid object ID: {0}")]
    InvalidObjectId(String),
    /// Tree entry name failed canonical path validation.
    #[error("invalid tree entry name: {0}")]
    InvalidTreeEntryName(String),
    /// Payload exceeded a configured object-size limit.
    #[error("payload too large: {size} bytes (max {max})")]
    PayloadTooLarge {
        /// Actual payload size in bytes.
        size: usize,
        /// Configured maximum payload size in bytes.
        max: usize,
    },
    /// Filesystem or stream I/O failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

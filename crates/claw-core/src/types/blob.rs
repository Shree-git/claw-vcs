use serde::{Deserialize, Serialize};

/// Raw file content stored as a Claw object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    /// Opaque file bytes.
    pub data: Vec<u8>,
    /// Optional media type for consumers that need display hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

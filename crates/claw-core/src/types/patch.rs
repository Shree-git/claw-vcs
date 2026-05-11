use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// A single codec-specific patch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOp {
    /// Codec-specific address within the target.
    pub address: String,
    /// Codec-specific operation name.
    pub op_type: String,
    /// Optional bytes expected before applying the operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_data: Option<Vec<u8>>,
    /// Optional bytes written by the operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_data: Option<Vec<u8>>,
    /// Optional context hash used by the codec to detect drift.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_hash: Option<u64>,
}

/// Patch object that transforms one object into another for a target path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Repository path this patch targets.
    pub target_path: String,
    /// Patch codec identifier.
    pub codec_id: String,
    /// Optional base object the patch applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_object: Option<ObjectId>,
    /// Optional result object produced by the patch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_object: Option<ObjectId>,
    /// Codec-specific operations.
    pub ops: Vec<PatchOp>,
    /// Optional opaque codec payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec_payload: Option<Vec<u8>>,
}

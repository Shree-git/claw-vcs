use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// Resolution state for a merge or patch conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictStatus {
    /// Conflict is unresolved.
    Open,
    /// Conflict has a recorded resolution.
    Resolved,
}

/// A recorded conflict between two revisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Optional common base revision.
    pub base_revision: Option<ObjectId>,
    /// Left-side revision involved in the conflict.
    pub left_revision: ObjectId,
    /// Right-side revision involved in the conflict.
    pub right_revision: ObjectId,
    /// Path whose patches or contents conflict.
    pub file_path: String,
    /// Patch codec that identified the conflict.
    pub codec_id: String,
    /// Patch IDs from the left side.
    pub left_patch_ids: Vec<ObjectId>,
    /// Patch IDs from the right side.
    pub right_patch_ids: Vec<ObjectId>,
    /// Patch IDs that resolve the conflict.
    #[serde(default)]
    pub resolution_patch_ids: Vec<ObjectId>,
    /// Current conflict state.
    pub status: ConflictStatus,
    /// Creation time in Unix epoch milliseconds.
    pub created_at_ms: u64,
}

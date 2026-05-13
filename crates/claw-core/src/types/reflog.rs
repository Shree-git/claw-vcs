use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// Append-only history for a repository reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefLog {
    /// Reference name whose movement is recorded.
    pub ref_name: String,
    /// Ordered reference updates.
    pub entries: Vec<RefLogEntry>,
}

/// A single reference movement record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefLogEntry {
    /// Previous reference target, if the ref existed.
    pub old_target: Option<ObjectId>,
    /// New reference target.
    pub new_target: ObjectId,
    /// Actor that moved the reference.
    pub author: String,
    /// Human-readable reason for the reference movement.
    pub message: String,
    /// Update time in Unix epoch milliseconds.
    pub timestamp: u64,
}

use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// Snapshot object binding a tree root to a revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Root tree object for the snapshot.
    pub tree_root: ObjectId,
    /// Revision represented by this snapshot.
    pub revision_id: ObjectId,
    /// Creation time in Unix epoch milliseconds.
    pub created_at_ms: u64,
}

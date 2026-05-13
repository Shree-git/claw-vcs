use serde::{Deserialize, Serialize};

use crate::id::ObjectId;

/// Revision object describing a repository state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    /// Change that produced this revision, when known.
    pub change_id: Option<crate::id::ChangeId>,
    /// Parent revisions.
    pub parents: Vec<ObjectId>,
    /// Patch objects included in this revision.
    pub patches: Vec<ObjectId>,
    /// Optional base snapshot object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_base: Option<ObjectId>,
    /// Optional tree object for the resulting state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<ObjectId>,
    /// Optional capsule object attached to this revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule_id: Option<ObjectId>,
    /// Author or agent that created the revision.
    pub author: String,
    /// Creation time in Unix epoch milliseconds.
    pub created_at_ms: u64,
    /// Short revision summary.
    pub summary: String,
    /// Policy evidence references associated with this revision.
    #[serde(default)]
    pub policy_evidence: Vec<String>,
}

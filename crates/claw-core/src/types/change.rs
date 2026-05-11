use serde::{Deserialize, Serialize};

use crate::id::{ChangeId, IntentId, ObjectId};

/// Lifecycle state for an intent-scoped change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeStatus {
    /// Change is still being worked.
    Open,
    /// Change has the evidence needed for integration.
    Ready,
    /// Change has been integrated.
    Integrated,
    /// Change was intentionally abandoned.
    Abandoned,
}

/// A unit of implementation work linked to an intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Stable change identifier.
    pub id: ChangeId,
    /// Intent this change serves.
    pub intent_id: IntentId,
    /// Current head revision for the change, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_revision: Option<ObjectId>,
    /// Optional workstream this change belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workstream_id: Option<String>,
    /// Current lifecycle state.
    pub status: ChangeStatus,
    /// Creation time in Unix epoch milliseconds.
    pub created_at_ms: u64,
    /// Last update time in Unix epoch milliseconds.
    pub updated_at_ms: u64,
}

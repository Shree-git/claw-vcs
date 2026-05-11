use serde::{Deserialize, Serialize};

use crate::id::ChangeId;

/// Ordered stack of related changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workstream {
    /// Stable workstream identifier.
    pub workstream_id: String,
    /// Ordered change stack for this workstream.
    #[serde(default)]
    pub change_stack: Vec<ChangeId>,
}

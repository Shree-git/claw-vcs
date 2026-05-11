use serde::{Deserialize, Serialize};

use crate::id::IntentId;

/// Lifecycle state for an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    /// Intent is active and accepting changes.
    Open,
    /// Intent is blocked by policy, dependency, or external work.
    Blocked,
    /// Intent has been completed.
    Done,
    /// Intent was replaced by another intent.
    Superseded,
}

/// User or agent intent that explains why changes are being made.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Stable intent identifier.
    pub id: IntentId,
    /// Short title for humans and tools.
    pub title: String,
    /// Goal statement describing desired outcome.
    pub goal: String,
    /// Constraints that shape acceptable solutions.
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Acceptance tests or checks associated with the intent.
    #[serde(default)]
    pub acceptance_tests: Vec<String>,
    /// External links related to the intent.
    #[serde(default)]
    pub links: Vec<String>,
    /// Policy IDs that apply to this intent.
    #[serde(default)]
    pub policy_refs: Vec<String>,
    /// Agents assigned to or associated with the intent.
    #[serde(default)]
    pub agents: Vec<String>,
    /// Changes created under this intent.
    #[serde(default)]
    pub change_ids: Vec<String>,
    /// Intent IDs this intent depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Intent IDs replaced by this intent.
    #[serde(default)]
    pub supersedes: Vec<String>,
    /// Current lifecycle state.
    pub status: IntentStatus,
    /// Creation time in Unix epoch milliseconds.
    pub created_at_ms: u64,
    /// Last update time in Unix epoch milliseconds.
    pub updated_at_ms: u64,
}

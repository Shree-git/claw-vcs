use claw_core::id::ObjectId;

/// Runtime evidence used during policy evaluation.
#[derive(Debug, Clone, Default)]
pub struct PolicyContext {
    /// Object ID of the revision currently being evaluated.
    pub revision_id: Option<ObjectId>,
    /// Agent identities associated with verified capsule signatures.
    pub signer_agent_ids: Vec<String>,
    /// Key identifiers associated with verified capsule signatures.
    pub signer_key_ids: Vec<String>,
    /// Repository paths touched by the evaluated revision.
    pub touched_paths: Vec<String>,
    /// Optional trust score in the inclusive range `0.0..=1.0`.
    pub trust_score: Option<f32>,
    /// Optional wall-clock timestamp for freshness evaluation.
    pub now_ms: Option<u64>,
}

impl PolicyContext {
    /// Return the first touched path that matches one of the sensitive path prefixes.
    pub fn touched_sensitive_path(&self, sensitive_paths: &[String]) -> Option<String> {
        if sensitive_paths.is_empty() || self.touched_paths.is_empty() {
            return None;
        }

        for path in &self.touched_paths {
            let normalized_path = normalize_path(path);
            for prefix in sensitive_paths {
                let normalized_prefix = normalize_path(prefix);
                if normalized_path.starts_with(normalized_prefix) {
                    return Some(path.clone());
                }
            }
        }

        None
    }
}

fn normalize_path(value: &str) -> &str {
    value
        .strip_prefix("./")
        .or_else(|| value.strip_prefix('/'))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::PolicyContext;

    #[test]
    fn matches_sensitive_prefixes() {
        let context = PolicyContext {
            touched_paths: vec!["src/secrets/token.txt".to_string()],
            ..PolicyContext::default()
        };

        let hit = context.touched_sensitive_path(&["src/secrets/".to_string()]);
        assert_eq!(hit.as_deref(), Some("src/secrets/token.txt"));
    }

    #[test]
    fn normalizes_dot_slash_prefix() {
        let context = PolicyContext {
            touched_paths: vec!["./admin/config.toml".to_string()],
            ..PolicyContext::default()
        };

        let hit = context.touched_sensitive_path(&["admin/".to_string()]);
        assert_eq!(hit.as_deref(), Some("./admin/config.toml"));
    }
}

use claw_core::types::{Capsule, Policy, Revision};

use crate::PolicyError;

/// Check that revisions touching sensitive paths have adequate capsule evidence.
///
/// If a policy declares `sensitive_paths` globs and the revision's patches touch
/// any matching path, the capsule must carry a passing "sensitive-path-review"
/// evidence item (or the agent must have encrypted private fields, proving
/// elevated trust).
pub fn check_sensitive_paths(
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
) -> Result<(), PolicyError> {
    if policy.sensitive_paths.is_empty() {
        return Ok(());
    }

    // Build glob matchers for each sensitive path pattern
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in &policy.sensitive_paths {
        let glob = globset::Glob::new(pattern).map_err(|e| {
            PolicyError::Violation(format!("invalid sensitive_paths glob '{}': {}", pattern, e))
        })?;
        builder.add(glob);
    }
    let glob_set = builder.build().map_err(|e| {
        PolicyError::Violation(format!("failed to compile sensitive_paths globs: {}", e))
    })?;

    // Extract touched paths from revision's policy_evidence field
    // (policy_evidence carries "touched:<path>" entries set by the snapshot command)
    let touched_paths: Vec<&str> = revision
        .policy_evidence
        .iter()
        .filter_map(|e| e.strip_prefix("touched:"))
        .collect();

    // If no touched paths recorded, we can't enforce — skip
    if touched_paths.is_empty() {
        return Ok(());
    }

    let any_match = touched_paths.iter().any(|path| glob_set.is_match(path));
    if !any_match {
        return Ok(());
    }

    // A sensitive path was touched — require evidence
    let has_review = capsule
        .public_fields
        .evidence
        .iter()
        .any(|e| e.name == "sensitive-path-review" && e.status == "pass");

    let has_encrypted_private = capsule.encrypted_private.is_some();

    if !has_review && !has_encrypted_private {
        let matched: Vec<&str> = touched_paths
            .iter()
            .filter(|p| glob_set.is_match(*p))
            .copied()
            .collect();
        return Err(PolicyError::Violation(format!(
            "revision touches sensitive path(s) [{}] but capsule lacks 'sensitive-path-review' evidence",
            matched.join(", ")
        )));
    }

    Ok(())
}

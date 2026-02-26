use claw_core::types::{Capsule, Policy, Revision};

use crate::checks::{
    verify_min_trust_score, verify_quarantine_lane, verify_required_checks,
    verify_required_reviewers, verify_sensitive_paths,
};
use crate::context::PolicyContext;
use crate::plugin::evaluate_plugins;
use crate::visibility::check_visibility;
use crate::PolicyError;

pub fn evaluate_policy(
    policy: &Policy,
    _revision: &Revision,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    // Check visibility constraints
    check_visibility(policy, capsule)?;

    // Check required checks
    verify_required_checks(policy, capsule)?;

    // Check required reviewers against verified signers.
    verify_required_reviewers(policy, context)?;

    // If sensitive paths were touched, enforce encrypted private capsule fields.
    verify_sensitive_paths(policy, capsule, context)?;

    // Quarantine lane blocks auto-integration when sensitive paths are touched.
    verify_quarantine_lane(policy, context)?;

    // Enforce trust score floor when configured.
    verify_min_trust_score(policy, context)?;

    // External plugins run in separate processes and must allow the policy.
    evaluate_plugins(policy, _revision, capsule, context)?;

    Ok(())
}

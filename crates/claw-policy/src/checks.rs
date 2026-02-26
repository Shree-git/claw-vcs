use std::collections::HashSet;

use claw_core::types::{Capsule, Policy};

use crate::context::PolicyContext;
use crate::PolicyError;

pub fn verify_required_checks(policy: &Policy, capsule: &Capsule) -> Result<(), PolicyError> {
    for check in &policy.required_checks {
        let found = capsule
            .public_fields
            .evidence
            .iter()
            .any(|e| &e.name == check && e.status.eq_ignore_ascii_case("pass"));
        if !found {
            return Err(PolicyError::MissingCheck(check.clone()));
        }
    }
    Ok(())
}

pub fn verify_required_reviewers(
    policy: &Policy,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    if policy.required_reviewers.is_empty() {
        return Ok(());
    }

    let available_reviewers: HashSet<String> = context
        .signer_agent_ids
        .iter()
        .chain(context.signer_key_ids.iter())
        .map(|id| id.to_ascii_lowercase())
        .collect();

    for reviewer in &policy.required_reviewers {
        if !available_reviewers.contains(&reviewer.to_ascii_lowercase()) {
            return Err(PolicyError::MissingReviewer(reviewer.clone()));
        }
    }

    Ok(())
}

pub fn verify_sensitive_paths(
    policy: &Policy,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    if let Some(path) = context.touched_sensitive_path(&policy.sensitive_paths) {
        if capsule.encrypted_private.is_none() {
            return Err(PolicyError::SensitivePathRequiresPrivate(path));
        }
    }

    Ok(())
}

pub fn verify_quarantine_lane(policy: &Policy, context: &PolicyContext) -> Result<(), PolicyError> {
    if !policy.quarantine_lane {
        return Ok(());
    }

    if policy.sensitive_paths.is_empty() {
        return Err(PolicyError::QuarantineLane(
            "quarantine lane blocks automated integration".to_string(),
        ));
    }

    if let Some(path) = context.touched_sensitive_path(&policy.sensitive_paths) {
        return Err(PolicyError::QuarantineLane(format!(
            "sensitive path '{}' requires quarantine lane",
            path
        )));
    }

    Ok(())
}

pub fn verify_min_trust_score(policy: &Policy, context: &PolicyContext) -> Result<(), PolicyError> {
    let Some(raw_threshold) = policy.min_trust_score.as_deref() else {
        return Ok(());
    };

    let threshold = parse_trust_score(raw_threshold)
        .map_err(|_| PolicyError::InvalidTrustScore(raw_threshold.to_string()))?;
    let actual = context.trust_score.ok_or(PolicyError::MissingTrustScore)?;

    if actual < threshold {
        return Err(PolicyError::MinTrustScoreNotMet {
            required: threshold,
            actual,
        });
    }

    Ok(())
}

fn parse_trust_score(value: &str) -> Result<f32, ()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(());
    }

    let parsed = if let Some(percent) = trimmed.strip_suffix('%') {
        percent.trim().parse::<f32>().map_err(|_| ())? / 100.0
    } else {
        trimmed.parse::<f32>().map_err(|_| ())?
    };

    if !(0.0..=1.0).contains(&parsed) {
        return Err(());
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;
    use claw_core::types::{CapsulePublic, Evidence, Visibility};

    use super::*;

    fn policy() -> Policy {
        Policy {
            policy_id: "default".to_string(),
            required_checks: vec![],
            required_reviewers: vec![],
            sensitive_paths: vec![],
            quarantine_lane: false,
            min_trust_score: None,
            visibility: Visibility::Public,
        }
    }

    fn capsule() -> Capsule {
        Capsule {
            revision_id: content_hash(TypeTag::Revision, b"rev"),
            public_fields: CapsulePublic {
                agent_id: "agent".to_string(),
                agent_version: None,
                toolchain_digest: None,
                env_fingerprint: None,
                evidence: vec![],
            },
            encrypted_private: None,
            encryption: String::new(),
            key_id: None,
            signatures: vec![],
        }
    }

    #[test]
    fn reviewer_check_accepts_agent_id_or_key() {
        let mut p = policy();
        p.required_reviewers = vec!["agent-a".to_string(), "ABC123".to_string()];

        let context = PolicyContext {
            signer_agent_ids: vec!["agent-a".to_string()],
            signer_key_ids: vec!["abc123".to_string()],
            ..PolicyContext::default()
        };

        assert!(verify_required_reviewers(&p, &context).is_ok());
    }

    #[test]
    fn reviewer_check_rejects_missing_reviewer() {
        let mut p = policy();
        p.required_reviewers = vec!["reviewer-1".to_string()];

        let context = PolicyContext {
            signer_agent_ids: vec!["agent-a".to_string()],
            ..PolicyContext::default()
        };

        let err = verify_required_reviewers(&p, &context).unwrap_err();
        assert!(matches!(err, PolicyError::MissingReviewer(_)));
    }

    #[test]
    fn sensitive_paths_require_private_capsule_data() {
        let mut p = policy();
        p.sensitive_paths = vec!["secrets/".to_string()];

        let context = PolicyContext {
            touched_paths: vec!["secrets/token.txt".to_string()],
            ..PolicyContext::default()
        };

        let err = verify_sensitive_paths(&p, &capsule(), &context).unwrap_err();
        assert!(matches!(err, PolicyError::SensitivePathRequiresPrivate(_)));
    }

    #[test]
    fn quarantine_lane_blocks_sensitive_paths() {
        let mut p = policy();
        p.sensitive_paths = vec!["admin/".to_string()];
        p.quarantine_lane = true;
        let context = PolicyContext {
            touched_paths: vec!["admin/settings.toml".to_string()],
            ..PolicyContext::default()
        };

        let err = verify_quarantine_lane(&p, &context).unwrap_err();
        assert!(matches!(err, PolicyError::QuarantineLane(_)));
    }

    #[test]
    fn trust_score_threshold_accepts_percentage_format() {
        let mut p = policy();
        p.min_trust_score = Some("80%".to_string());

        let context = PolicyContext {
            trust_score: Some(0.81),
            ..PolicyContext::default()
        };

        assert!(verify_min_trust_score(&p, &context).is_ok());
    }

    #[test]
    fn trust_score_threshold_rejects_low_score() {
        let mut p = policy();
        p.min_trust_score = Some("0.9".to_string());

        let context = PolicyContext {
            trust_score: Some(0.8),
            ..PolicyContext::default()
        };

        let err = verify_min_trust_score(&p, &context).unwrap_err();
        assert!(matches!(err, PolicyError::MinTrustScoreNotMet { .. }));
    }

    #[test]
    fn required_checks_ignore_status_case() {
        let mut p = policy();
        p.required_checks = vec!["ci".to_string()];
        let mut c = capsule();
        c.public_fields.evidence.push(Evidence {
            name: "ci".to_string(),
            status: "PASS".to_string(),
            duration_ms: 10,
            artifact_refs: vec![],
            summary: None,
        });

        assert!(verify_required_checks(&p, &c).is_ok());
    }
}

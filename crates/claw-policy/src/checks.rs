use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use claw_core::types::{
    Capsule, Evidence, Policy, Revision, CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION,
    RECIPIENT_ENVELOPE_ALGORITHM,
};

use crate::context::PolicyContext;
use crate::PolicyError;

/// Verify that every required policy check has passing capsule evidence.
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

/// Verify that all required reviewers are present in the evaluated signer context.
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

/// Require encrypted private capsule fields when sensitive paths were touched.
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

/// Enforce quarantine-lane policy behavior for automated integration.
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

/// Verify that the evaluated context meets a configured minimum trust score.
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

/// Verify recipient envelopes against policy-defined authorized recipients.
pub fn verify_authorized_recipients(policy: &Policy, capsule: &Capsule) -> Result<(), PolicyError> {
    let revoked: HashSet<String> = policy
        .revoked_recipients
        .iter()
        .map(|id| id.to_ascii_lowercase())
        .collect();

    for recipient in &capsule.recipients {
        if revoked.contains(&recipient.recipient_id.to_ascii_lowercase()) {
            return Err(PolicyError::RecipientAuthorization(format!(
                "recipient '{}' is revoked by policy",
                recipient.recipient_id
            )));
        }
    }

    if policy.authorized_recipients.is_empty() {
        return Ok(());
    }

    if !matches!(capsule.encrypted_private.as_deref(), Some(bytes) if !bytes.is_empty()) {
        return Err(PolicyError::RecipientAuthorization(
            "authorized recipients require encrypted private capsule fields".to_string(),
        ));
    }
    if capsule.encryption != CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION {
        return Err(PolicyError::RecipientAuthorization(format!(
            "authorized recipients require {} encryption metadata",
            CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION
        )));
    }

    let authorized: HashSet<String> = policy
        .authorized_recipients
        .iter()
        .map(|id| id.to_ascii_lowercase())
        .collect();
    let mut present = HashSet::new();

    for recipient in &capsule.recipients {
        let id = recipient.recipient_id.to_ascii_lowercase();
        if !authorized.contains(&id) {
            return Err(PolicyError::RecipientAuthorization(format!(
                "recipient '{}' is not authorized by policy",
                recipient.recipient_id
            )));
        }
        if recipient.algorithm != RECIPIENT_ENVELOPE_ALGORITHM {
            return Err(PolicyError::RecipientAuthorization(format!(
                "recipient '{}' envelope uses unsupported algorithm '{}'",
                recipient.recipient_id, recipient.algorithm
            )));
        }
        if recipient.key_id.trim().is_empty()
            || recipient.encrypted_content_key.is_empty()
            || recipient.ephemeral_public_key.len() != 32
        {
            return Err(PolicyError::RecipientAuthorization(format!(
                "recipient '{}' envelope is incomplete",
                recipient.recipient_id
            )));
        }
        present.insert(id);
    }

    for required in &authorized {
        if !present.contains(required) {
            return Err(PolicyError::RecipientAuthorization(format!(
                "missing recipient envelope for '{}'",
                required
            )));
        }
    }

    Ok(())
}

/// Verify that required evidence is bound to the evaluated revision and fresh.
pub fn verify_evidence_freshness(
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    let freshness = &policy.evidence_policy;
    if !freshness.require_fresh_evidence {
        return Ok(());
    }

    let mut evidence_items: Vec<&Evidence> = capsule.public_fields.evidence.iter().collect();
    if !policy.required_checks.is_empty() {
        let required: HashSet<String> = policy
            .required_checks
            .iter()
            .map(|check| check.to_ascii_lowercase())
            .collect();
        evidence_items.retain(|e| required.contains(&e.name.to_ascii_lowercase()));
    }

    if evidence_items.is_empty() {
        return Err(PolicyError::StaleEvidence {
            check: "*".to_string(),
            reason: "policy requires fresh evidence but capsule has none".to_string(),
        });
    }

    let now_ms = context.now_ms.unwrap_or_else(current_time_ms);
    for evidence in evidence_items {
        verify_one_evidence(policy, revision, capsule, evidence, now_ms, context)?;
    }

    Ok(())
}

fn verify_one_evidence(
    policy: &Policy,
    revision: &Revision,
    capsule: &Capsule,
    evidence: &Evidence,
    now_ms: u64,
    context: &PolicyContext,
) -> Result<(), PolicyError> {
    let freshness = &policy.evidence_policy;
    let check = evidence.name.clone();

    if freshness.require_revision_match {
        let expected_revision_id = context.revision_id.unwrap_or(capsule.revision_id);
        if capsule.revision_id != expected_revision_id {
            return Err(stale(
                &check,
                "capsule revision_id does not match evaluated revision",
            ));
        }
        let evidence_revision = evidence
            .revision_id
            .ok_or_else(|| stale(&check, "missing revision_id"))?;
        if evidence_revision != expected_revision_id {
            return Err(stale(
                &check,
                "revision_id does not match evaluated revision",
            ));
        }
    }

    if freshness.require_evidence_after_revision {
        let evidence_time = evidence
            .ended_at_ms
            .or(evidence.started_at_ms)
            .ok_or_else(|| stale(&check, "missing evidence timestamp"))?;
        if evidence_time < revision.created_at_ms {
            return Err(stale(&check, "evidence timestamp is older than revision"));
        }
    }

    if freshness.require_expires_at && evidence.expires_at_ms.is_none() {
        return Err(stale(&check, "missing expires_at_ms"));
    }
    if evidence
        .expires_at_ms
        .is_some_and(|expires_at| expires_at <= now_ms)
    {
        return Err(stale(&check, "evidence has expired"));
    }

    if let Some(max_age_ms) = freshness.max_age_ms {
        let evidence_time = evidence
            .ended_at_ms
            .or(evidence.started_at_ms)
            .ok_or_else(|| stale(&check, "missing evidence timestamp"))?;
        if now_ms.saturating_sub(evidence_time) > max_age_ms {
            return Err(stale(&check, "evidence exceeds max_age_ms"));
        }
    }

    if freshness.require_runner_identity
        && evidence
            .runner_identity
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(stale(&check, "missing runner_identity"));
    }
    if let Some(runner) = evidence.runner_identity.as_deref() {
        if !freshness.trusted_runner_identities.is_empty()
            && !freshness
                .trusted_runner_identities
                .iter()
                .any(|trusted| trusted.eq_ignore_ascii_case(runner))
        {
            return Err(stale(&check, "runner_identity is not trusted"));
        }
    }

    if freshness.require_command
        && evidence
            .command
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(stale(&check, "missing command"));
    }
    if freshness.require_exit_code && evidence.exit_code.is_none() {
        return Err(stale(&check, "missing exit_code"));
    }
    if evidence.status.eq_ignore_ascii_case("pass")
        && evidence.exit_code.is_some_and(|code| code != 0)
    {
        return Err(stale(&check, "passing evidence must have exit_code 0"));
    }
    if freshness.require_log_or_artifact_digest
        && evidence
            .log_digest
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        && evidence
            .artifact_digest
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(stale(&check, "missing log_digest or artifact_digest"));
    }
    if freshness.require_environment_digest
        && evidence
            .environment_digest
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(stale(&check, "missing environment_digest"));
    }

    Ok(())
}

fn stale(check: &str, reason: &str) -> PolicyError {
    PolicyError::StaleEvidence {
        check: check.to_string(),
        reason: reason.to_string(),
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
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
    use claw_core::types::{CapsulePublic, CapsuleRecipient, Evidence, EvidencePolicy, Visibility};

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
            authorized_recipients: vec![],
            revoked_recipients: vec![],
            evidence_policy: EvidencePolicy::default(),
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
            recipients: vec![],
            signatures: vec![],
        }
    }

    fn fresh_evidence(revision_id: claw_core::id::ObjectId) -> Evidence {
        Evidence {
            name: "ci".to_string(),
            status: "PASS".to_string(),
            duration_ms: 10,
            artifact_refs: vec![],
            summary: None,
            revision_id: Some(revision_id),
            command: Some("cargo test".to_string()),
            exit_code: Some(0),
            started_at_ms: Some(1_100),
            ended_at_ms: Some(1_200),
            environment_digest: Some("sha256:env".to_string()),
            runner_identity: Some("runner-a".to_string()),
            log_digest: Some("sha256:log".to_string()),
            artifact_digest: None,
            expires_at_ms: Some(2_000),
            trust_domain: Some("ci".to_string()),
            signature: Some(vec![1, 2, 3]),
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
            revision_id: None,
            command: None,
            exit_code: None,
            started_at_ms: None,
            ended_at_ms: None,
            environment_digest: None,
            runner_identity: None,
            log_digest: None,
            artifact_digest: None,
            expires_at_ms: None,
            trust_domain: None,
            signature: None,
        });

        assert!(verify_required_checks(&p, &c).is_ok());
    }

    #[test]
    fn freshness_policy_accepts_complete_evidence() {
        let revision_id = content_hash(TypeTag::Revision, b"rev");
        let revision = Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: None,
            capsule_id: None,
            author: "agent".to_string(),
            created_at_ms: 1_000,
            summary: "test".to_string(),
            policy_evidence: vec![],
        };
        let mut p = policy();
        p.required_checks = vec!["ci".to_string()];
        p.evidence_policy.require_fresh_evidence = true;
        p.evidence_policy.trusted_runner_identities = vec!["runner-a".to_string()];

        let mut c = capsule();
        c.revision_id = revision_id;
        c.public_fields.evidence.push(fresh_evidence(revision_id));
        let context = PolicyContext {
            revision_id: Some(revision_id),
            now_ms: Some(1_500),
            ..PolicyContext::default()
        };

        assert!(verify_evidence_freshness(&p, &revision, &c, &context).is_ok());
    }

    #[test]
    fn freshness_policy_rejects_capsule_from_different_revision() {
        let evaluated_revision_id = content_hash(TypeTag::Revision, b"evaluated-rev");
        let capsule_revision_id = content_hash(TypeTag::Revision, b"capsule-rev");
        let revision = Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: None,
            capsule_id: None,
            author: "agent".to_string(),
            created_at_ms: 1_000,
            summary: "test".to_string(),
            policy_evidence: vec![],
        };
        let mut p = policy();
        p.required_checks = vec!["ci".to_string()];
        p.evidence_policy.require_fresh_evidence = true;

        let mut c = capsule();
        c.revision_id = capsule_revision_id;
        c.public_fields
            .evidence
            .push(fresh_evidence(capsule_revision_id));
        let context = PolicyContext {
            revision_id: Some(evaluated_revision_id),
            now_ms: Some(1_500),
            ..PolicyContext::default()
        };

        let err = verify_evidence_freshness(&p, &revision, &c, &context).unwrap_err();
        assert!(matches!(
            err,
            PolicyError::StaleEvidence { reason, .. }
                if reason == "capsule revision_id does not match evaluated revision"
        ));
    }

    #[test]
    fn freshness_policy_rejects_untrusted_runner() {
        let revision_id = content_hash(TypeTag::Revision, b"rev");
        let revision = Revision {
            change_id: None,
            parents: vec![],
            patches: vec![],
            snapshot_base: None,
            tree: None,
            capsule_id: None,
            author: "agent".to_string(),
            created_at_ms: 1_000,
            summary: "test".to_string(),
            policy_evidence: vec![],
        };
        let mut p = policy();
        p.evidence_policy.require_fresh_evidence = true;
        p.evidence_policy.trusted_runner_identities = vec!["runner-b".to_string()];

        let mut c = capsule();
        c.revision_id = revision_id;
        c.public_fields.evidence.push(fresh_evidence(revision_id));
        let context = PolicyContext {
            now_ms: Some(1_500),
            ..PolicyContext::default()
        };

        let err = verify_evidence_freshness(&p, &revision, &c, &context).unwrap_err();
        assert!(matches!(err, PolicyError::StaleEvidence { .. }));
    }

    #[test]
    fn authorized_recipients_rejects_missing_envelope() {
        let mut p = policy();
        p.authorized_recipients = vec!["security".to_string()];

        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION.to_string();

        let err = verify_authorized_recipients(&p, &c).unwrap_err();
        assert!(matches!(err, PolicyError::RecipientAuthorization(_)));
    }

    #[test]
    fn authorized_recipients_accepts_policy_recipient() {
        let mut p = policy();
        p.authorized_recipients = vec!["security".to_string()];

        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION.to_string();
        c.recipients.push(CapsuleRecipient {
            recipient_id: "security".to_string(),
            key_id: "security-key".to_string(),
            algorithm: RECIPIENT_ENVELOPE_ALGORITHM.to_string(),
            ephemeral_public_key: vec![7; 32],
            encrypted_content_key: vec![8; 48],
        });

        assert!(verify_authorized_recipients(&p, &c).is_ok());
    }

    #[test]
    fn revoked_recipients_reject_present_envelope() {
        let mut p = policy();
        p.revoked_recipients = vec!["former-reviewer".to_string()];

        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION.to_string();
        c.recipients.push(CapsuleRecipient {
            recipient_id: "former-reviewer".to_string(),
            key_id: "old-key".to_string(),
            algorithm: RECIPIENT_ENVELOPE_ALGORITHM.to_string(),
            ephemeral_public_key: vec![1; 32],
            encrypted_content_key: vec![2],
        });

        let err = verify_authorized_recipients(&p, &c).unwrap_err();
        assert!(matches!(err, PolicyError::RecipientAuthorization(_)));
        assert!(err.to_string().contains("revoked"));
    }

    #[test]
    fn authorized_recipients_rejects_unusable_envelope_metadata() {
        let mut p = policy();
        p.authorized_recipients = vec!["security".to_string()];

        let mut c = capsule();
        c.encrypted_private = Some(vec![1, 2, 3]);
        c.encryption = CAPSULE_RECIPIENT_PRIVATE_ENCRYPTION.to_string();
        c.recipients.push(CapsuleRecipient {
            recipient_id: "security".to_string(),
            key_id: String::new(),
            algorithm: "plaintext".to_string(),
            ephemeral_public_key: vec![7; 31],
            encrypted_content_key: vec![],
        });

        let err = verify_authorized_recipients(&p, &c).unwrap_err();
        assert!(matches!(err, PolicyError::RecipientAuthorization(_)));
    }
}

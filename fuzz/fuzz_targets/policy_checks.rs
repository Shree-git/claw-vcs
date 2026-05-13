#![no_main]

use claw_core::hash::content_hash;
use claw_core::object::TypeTag;
use claw_core::types::{Capsule, CapsulePublic, Evidence, EvidencePolicy, Policy, Visibility};
use claw_policy::checks::{
    verify_min_trust_score, verify_quarantine_lane, verify_required_checks,
    verify_required_reviewers, verify_sensitive_paths,
};
use claw_policy::context::PolicyContext;
use claw_policy::visibility::check_visibility;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let required_check = data.first().is_some_and(|byte| byte & 0x01 != 0);
    let encrypted_private = data.first().is_some_and(|byte| byte & 0x02 != 0);
    let sensitive = data.first().is_some_and(|byte| byte & 0x04 != 0);
    let quarantine_lane = data.first().is_some_and(|byte| byte & 0x08 != 0);
    let trust_score = data.get(1).map(|byte| (*byte as f32) / 255.0);

    let visibility = match data.get(2).copied().unwrap_or_default() % 3 {
        0 => Visibility::Public,
        1 => Visibility::Private,
        _ => Visibility::EncryptedMetadataRequired,
    };
    let policy = Policy {
        policy_id: "fuzz-policy".to_string(),
        required_checks: required_check
            .then(|| "ci".to_string())
            .into_iter()
            .collect(),
        required_reviewers: vec!["reviewer".to_string()],
        sensitive_paths: sensitive
            .then(|| "secrets/".to_string())
            .into_iter()
            .collect(),
        quarantine_lane,
        min_trust_score: Some("0.5".to_string()),
        visibility,
        authorized_recipients: vec![],
        revoked_recipients: vec![],
        evidence_policy: EvidencePolicy::default(),
    };
    let revision_id = content_hash(TypeTag::Revision, data);
    let capsule = Capsule {
        revision_id,
        public_fields: CapsulePublic {
            agent_id: "fuzzer".to_string(),
            agent_version: None,
            toolchain_digest: None,
            env_fingerprint: None,
            evidence: vec![Evidence {
                name: "ci".to_string(),
                status: if data.get(3).is_some_and(|byte| byte & 0x01 != 0) {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                duration_ms: 1,
                artifact_refs: vec![],
                summary: None,
                revision_id: Some(revision_id),
                command: Some("fuzz".to_string()),
                exit_code: Some(0),
                started_at_ms: Some(1),
                ended_at_ms: Some(2),
                environment_digest: Some("sha256:fuzz".to_string()),
                runner_identity: Some("fuzzer".to_string()),
                log_digest: Some("sha256:log".to_string()),
                artifact_digest: None,
                expires_at_ms: Some(3),
                trust_domain: Some("fuzz".to_string()),
                signature: None,
            }],
        },
        encrypted_private: encrypted_private.then(|| data.to_vec()),
        encryption: String::new(),
        key_id: None,
        recipients: vec![],
        signatures: vec![],
    };
    let context = PolicyContext {
        revision_id: Some(revision_id),
        signer_agent_ids: vec!["reviewer".to_string()],
        signer_key_ids: vec![],
        touched_paths: sensitive
            .then(|| "secrets/fuzz.txt".to_string())
            .into_iter()
            .collect(),
        trust_score,
        now_ms: Some(2),
    };

    let _ = check_visibility(&policy, &capsule, &context);
    let _ = verify_required_checks(&policy, &capsule);
    let _ = verify_required_reviewers(&policy, &context);
    let _ = verify_sensitive_paths(&policy, &capsule, &context);
    let _ = verify_quarantine_lane(&policy, &context);
    let _ = verify_min_trust_score(&policy, &context);
});

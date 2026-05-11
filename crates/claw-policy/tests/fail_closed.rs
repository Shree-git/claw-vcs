use std::sync::Mutex;

use claw_core::hash::content_hash;
use claw_core::object::TypeTag;
use claw_core::types::{
    Capsule, CapsulePublic, Evidence, EvidencePolicy, Policy, Revision, Visibility,
};
use claw_policy::checks::verify_sensitive_paths;
use claw_policy::context::PolicyContext;
use claw_policy::evaluator::evaluate_policy;
use claw_policy::PolicyError;
use proptest::prelude::*;
use serde::Deserialize;

static POLICY_ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Deserialize)]
struct FailClosedVector {
    name: String,
    visibility: String,
    required_checks: Vec<String>,
    evidence: Vec<EvidenceVector>,
    required_reviewers: Vec<String>,
    signer_agent_ids: Vec<String>,
    signer_key_ids: Vec<String>,
    sensitive_paths: Vec<String>,
    touched_paths: Vec<String>,
    encrypted_private: bool,
    quarantine_lane: bool,
    min_trust_score: Option<String>,
    trust_score: Option<f32>,
    expected_error: String,
}

#[derive(Debug, Deserialize)]
struct EvidenceVector {
    name: String,
    status: String,
}

#[test]
fn fail_closed_vectors_reject_without_optional_allow_evidence() {
    with_clean_policy_env(|| {
        let vectors: Vec<FailClosedVector> = serde_json::from_str(include_str!(
            "../../../tests/vectors/policy_fail_closed_vectors.json"
        ))
        .unwrap();

        for vector in vectors {
            let policy = policy_from_vector(&vector);
            let capsule = capsule_from_vector(&vector);
            let context = context_from_vector(&vector);
            let err = evaluate_policy(&policy, &revision(), &capsule, &context).unwrap_err();
            let message = err.to_string();

            assert!(
                message.contains(&vector.expected_error),
                "{}: expected error containing {:?}, got {:?}",
                vector.name,
                vector.expected_error,
                message
            );
        }
    });
}

#[test]
fn missing_policy_plugin_executable_denies_by_default() {
    with_clean_policy_env(|| {
        std::env::set_var(
            "CLAW_POLICY_PLUGINS",
            "/tmp/claw-policy-plugin-that-should-not-exist",
        );

        let mut vector = passing_vector();
        vector.evidence.push(EvidenceVector {
            name: "ci".to_string(),
            status: "pass".to_string(),
        });
        vector.required_checks.push("ci".to_string());
        let err = evaluate_policy(
            &policy_from_vector(&vector),
            &revision(),
            &capsule_from_vector(&vector),
            &context_from_vector(&vector),
        )
        .unwrap_err();

        assert!(matches!(err, PolicyError::PluginSpawn { .. }));
    });
}

proptest! {
    #[test]
    fn sensitive_path_without_encrypted_private_fails_closed(
        prefix in "[a-z][a-z0-9_-]{0,8}",
        leaf in "[a-z][a-z0-9_-]{0,8}"
    ) {
        let policy = Policy {
            policy_id: "sensitive".to_string(),
            required_checks: vec![],
            required_reviewers: vec![],
            sensitive_paths: vec![format!("{prefix}/")],
            quarantine_lane: false,
            min_trust_score: None,
            visibility: Visibility::Public,
            authorized_recipients: vec![],
            revoked_recipients: vec![],
            evidence_policy: EvidencePolicy::default(),
        };
        let capsule = capsule(false, vec![]);
        let context = PolicyContext {
            touched_paths: vec![format!("./{prefix}/{leaf}.txt")],
            ..PolicyContext::default()
        };

        let err = verify_sensitive_paths(&policy, &capsule, &context).unwrap_err();
        prop_assert!(matches!(err, PolicyError::SensitivePathRequiresPrivate(_)));
    }
}

fn with_clean_policy_env<R>(f: impl FnOnce() -> R) -> R {
    let _lock = POLICY_ENV_LOCK.lock().unwrap();
    let _reset = PolicyEnvReset;
    std::env::remove_var("CLAW_POLICY_PLUGINS");
    std::env::remove_var("CLAW_POLICY_PLUGIN_TIMEOUT_MS");
    f()
}

struct PolicyEnvReset;

impl Drop for PolicyEnvReset {
    fn drop(&mut self) {
        std::env::remove_var("CLAW_POLICY_PLUGINS");
        std::env::remove_var("CLAW_POLICY_PLUGIN_TIMEOUT_MS");
    }
}

fn passing_vector() -> FailClosedVector {
    FailClosedVector {
        name: "passing".to_string(),
        visibility: "Public".to_string(),
        required_checks: vec![],
        evidence: vec![],
        required_reviewers: vec![],
        signer_agent_ids: vec![],
        signer_key_ids: vec![],
        sensitive_paths: vec![],
        touched_paths: vec![],
        encrypted_private: false,
        quarantine_lane: false,
        min_trust_score: None,
        trust_score: None,
        expected_error: String::new(),
    }
}

fn policy_from_vector(vector: &FailClosedVector) -> Policy {
    Policy {
        policy_id: vector.name.clone(),
        required_checks: vector.required_checks.clone(),
        required_reviewers: vector.required_reviewers.clone(),
        sensitive_paths: vector.sensitive_paths.clone(),
        quarantine_lane: vector.quarantine_lane,
        min_trust_score: vector.min_trust_score.clone(),
        visibility: visibility(&vector.visibility),
        authorized_recipients: vec![],
        revoked_recipients: vec![],
        evidence_policy: EvidencePolicy::default(),
    }
}

fn capsule_from_vector(vector: &FailClosedVector) -> Capsule {
    let evidence = vector
        .evidence
        .iter()
        .map(|entry| Evidence {
            name: entry.name.clone(),
            status: entry.status.clone(),
            duration_ms: 1,
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
        })
        .collect();
    capsule(vector.encrypted_private, evidence)
}

fn capsule(encrypted_private: bool, evidence: Vec<Evidence>) -> Capsule {
    Capsule {
        revision_id: content_hash(TypeTag::Revision, b"policy-test-revision"),
        public_fields: CapsulePublic {
            agent_id: "agent".to_string(),
            agent_version: None,
            toolchain_digest: None,
            env_fingerprint: None,
            evidence,
        },
        encrypted_private: encrypted_private.then(|| vec![1, 2, 3, 4]),
        encryption: if encrypted_private {
            "xchacha20poly1305".to_string()
        } else {
            String::new()
        },
        key_id: None,
        recipients: vec![],
        signatures: vec![],
    }
}

fn context_from_vector(vector: &FailClosedVector) -> PolicyContext {
    PolicyContext {
        revision_id: None,
        signer_agent_ids: vector.signer_agent_ids.clone(),
        signer_key_ids: vector.signer_key_ids.clone(),
        touched_paths: vector.touched_paths.clone(),
        trust_score: vector.trust_score,
        now_ms: None,
    }
}

fn revision() -> Revision {
    Revision {
        change_id: None,
        parents: vec![],
        patches: vec![],
        snapshot_base: None,
        tree: None,
        capsule_id: None,
        author: "codex".to_string(),
        created_at_ms: 1_700_000_000_000,
        summary: "policy fail-closed test".to_string(),
        policy_evidence: vec![],
    }
}

fn visibility(raw: &str) -> Visibility {
    match raw {
        "Public" => Visibility::Public,
        "Private" => Visibility::Private,
        "Restricted" | "EncryptedMetadataRequired" => Visibility::EncryptedMetadataRequired,
        other => panic!("unknown visibility in vector: {other}"),
    }
}

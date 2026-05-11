use clap::{Args, Subcommand};

use claw_core::hash::content_hash;
use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Capsule, EvidencePolicy, Policy, Revision, Visibility};
use claw_policy::{evaluator::evaluate_policy, PolicyContext};
use claw_store::ClawStore;

use crate::config::find_repo_root;

#[derive(Args)]
pub struct PolicyArgs {
    #[command(subcommand)]
    command: PolicyCommand,
}

#[derive(Subcommand)]
enum PolicyCommand {
    /// Create or update a policy
    Create {
        /// Policy ID
        #[arg(long)]
        id: String,
        /// Visibility: public|private|encrypted-metadata-required
        #[arg(long, default_value = "public")]
        visibility: String,
        /// Required check (repeat for multiple checks)
        #[arg(long = "check")]
        checks: Vec<String>,
        /// Required reviewer identity (repeatable)
        #[arg(long = "reviewer")]
        reviewers: Vec<String>,
        /// Sensitive path glob (repeatable)
        #[arg(long = "sensitive-path")]
        sensitive_paths: Vec<String>,
        /// Mark policy as quarantine lane
        #[arg(long)]
        quarantine_lane: bool,
        /// Optional minimum trust score threshold (e.g. 0.8 or 80%)
        #[arg(long)]
        min_trust_score: Option<String>,
        /// Authorized recipient ID for encrypted private fields (repeatable)
        #[arg(long = "recipient")]
        recipients: Vec<String>,
        /// Revoked recipient ID that must not appear in capsule envelopes (repeatable)
        #[arg(long = "revoked-recipient")]
        revoked_recipients: Vec<String>,
        /// Require evidence freshness metadata for required checks
        #[arg(long)]
        require_fresh_evidence: bool,
        /// Maximum evidence age in milliseconds when freshness is required
        #[arg(long)]
        evidence_max_age_ms: Option<u64>,
        /// Trusted runner identity for fresh evidence (repeatable)
        #[arg(long = "trusted-runner")]
        trusted_runners: Vec<String>,
    },
    /// Preview or apply a policy definition
    Apply {
        /// Policy ID
        #[arg(long)]
        id: String,
        /// Visibility: public|private|encrypted-metadata-required
        #[arg(long, default_value = "public")]
        visibility: String,
        /// Required check (repeat for multiple checks)
        #[arg(long = "check")]
        checks: Vec<String>,
        /// Required reviewer identity (repeatable)
        #[arg(long = "reviewer")]
        reviewers: Vec<String>,
        /// Sensitive path glob (repeatable)
        #[arg(long = "sensitive-path")]
        sensitive_paths: Vec<String>,
        /// Mark policy as quarantine lane
        #[arg(long)]
        quarantine_lane: bool,
        /// Optional minimum trust score threshold (e.g. 0.8 or 80%)
        #[arg(long)]
        min_trust_score: Option<String>,
        /// Authorized recipient ID for encrypted private fields (repeatable)
        #[arg(long = "recipient")]
        recipients: Vec<String>,
        /// Revoked recipient ID that must not appear in capsule envelopes (repeatable)
        #[arg(long = "revoked-recipient")]
        revoked_recipients: Vec<String>,
        /// Require evidence freshness metadata for required checks
        #[arg(long)]
        require_fresh_evidence: bool,
        /// Maximum evidence age in milliseconds when freshness is required
        #[arg(long)]
        evidence_max_age_ms: Option<u64>,
        /// Trusted runner identity for fresh evidence (repeatable)
        #[arg(long = "trusted-runner")]
        trusted_runners: Vec<String>,
        /// Preview object/ref changes without writing them
        #[arg(long)]
        dry_run: bool,
        /// Output apply result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Evaluate a policy against a revision and capsule
    Eval {
        /// Policy ID or policy ref
        id: String,
        /// Revision ref, hex ID, or clw_ display ID
        #[arg(long)]
        revision: String,
        /// Capsule ref, hex ID, or clw_ display ID. Defaults to the revision capsule.
        #[arg(long)]
        capsule: Option<String>,
        /// Verified signer agent ID (repeatable)
        #[arg(long = "signer-agent")]
        signer_agents: Vec<String>,
        /// Verified signer key ID (repeatable)
        #[arg(long = "signer-key")]
        signer_keys: Vec<String>,
        /// Touched path for sensitive-path evaluation (repeatable)
        #[arg(long = "path")]
        touched_paths: Vec<String>,
        /// Trust score override (e.g. 0.8 or 80%). Defaults to capsule evidence pass ratio.
        #[arg(long)]
        trust_score: Option<String>,
        /// Output evaluation result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a policy
    Show {
        /// Policy ID
        id: String,
    },
    /// List policies
    List,
}

pub fn run(args: PolicyArgs) -> anyhow::Result<()> {
    match args.command {
        PolicyCommand::Create {
            id,
            visibility,
            checks,
            reviewers,
            sensitive_paths,
            quarantine_lane,
            min_trust_score,
            recipients,
            revoked_recipients,
            require_fresh_evidence,
            evidence_max_age_ms,
            trusted_runners,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let policy = build_policy(PolicyBuildOptions {
                id,
                visibility,
                checks,
                reviewers,
                sensitive_paths,
                quarantine_lane,
                min_trust_score,
                recipients,
                revoked_recipients,
                require_fresh_evidence,
                evidence_max_age_ms,
                trusted_runners,
            })?;
            save_policy(&store, &policy, false, false)?;
        }
        PolicyCommand::Apply {
            id,
            visibility,
            checks,
            reviewers,
            sensitive_paths,
            quarantine_lane,
            min_trust_score,
            recipients,
            revoked_recipients,
            require_fresh_evidence,
            evidence_max_age_ms,
            trusted_runners,
            dry_run,
            json,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let policy = build_policy(PolicyBuildOptions {
                id,
                visibility,
                checks,
                reviewers,
                sensitive_paths,
                quarantine_lane,
                min_trust_score,
                recipients,
                revoked_recipients,
                require_fresh_evidence,
                evidence_max_age_ms,
                trusted_runners,
            })?;
            save_policy(&store, &policy, dry_run, json)?;
        }
        PolicyCommand::Eval {
            id,
            revision,
            capsule,
            signer_agents,
            signer_keys,
            touched_paths,
            trust_score,
            json,
        } => {
            run_eval(EvalRequest {
                policy_id: id,
                revision_ref: revision,
                capsule_ref: capsule,
                signer_agents,
                signer_keys,
                touched_paths,
                trust_score,
                json,
            })?;
        }
        PolicyCommand::Show { id } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let ref_name = if id.starts_with("policies/") {
                id
            } else {
                format!("policies/{id}")
            };
            let obj_id = store
                .get_ref(&ref_name)?
                .ok_or_else(|| anyhow::anyhow!("policy not found: {ref_name}"))?;
            let obj = store.load_object(&obj_id)?;

            let policy = match obj {
                Object::Policy(p) => p,
                _ => anyhow::bail!("ref does not point to a policy object: {ref_name}"),
            };

            println!("Policy: {}", policy.policy_id);
            println!("  Ref: {ref_name}");
            println!("  Visibility: {:?}", policy.visibility);
            if !policy.required_checks.is_empty() {
                println!("  Required checks: {}", policy.required_checks.join(", "));
            }
            if !policy.required_reviewers.is_empty() {
                println!(
                    "  Required reviewers: {}",
                    policy.required_reviewers.join(", ")
                );
            }
            if !policy.sensitive_paths.is_empty() {
                println!("  Sensitive paths: {}", policy.sensitive_paths.join(", "));
            }
            if policy.quarantine_lane {
                println!("  Quarantine lane: true");
            }
            if let Some(score) = policy.min_trust_score {
                println!("  Min trust score: {score}");
            }
            if !policy.authorized_recipients.is_empty() {
                println!(
                    "  Authorized recipients: {}",
                    policy.authorized_recipients.join(", ")
                );
            }
            if !policy.revoked_recipients.is_empty() {
                println!(
                    "  Revoked recipients: {}",
                    policy.revoked_recipients.join(", ")
                );
            }
            if policy.evidence_policy.require_fresh_evidence {
                println!("  Fresh evidence: required");
                if let Some(max_age_ms) = policy.evidence_policy.max_age_ms {
                    println!("  Evidence max age ms: {max_age_ms}");
                }
                if !policy.evidence_policy.trusted_runner_identities.is_empty() {
                    println!(
                        "  Trusted runners: {}",
                        policy.evidence_policy.trusted_runner_identities.join(", ")
                    );
                }
            }
        }
        PolicyCommand::List => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let refs = store.list_refs("policies")?;

            if refs.is_empty() {
                println!("No policies found.");
                return Ok(());
            }

            for (name, obj_id) in refs {
                match store.load_object(&obj_id) {
                    Ok(Object::Policy(policy)) => {
                        println!(
                            "{} {:?} checks:{}",
                            policy.policy_id,
                            policy.visibility,
                            policy.required_checks.len()
                        );
                    }
                    _ => {
                        println!("{} (non-policy object)", name);
                    }
                }
            }
        }
    }

    Ok(())
}

struct PolicyBuildOptions {
    id: String,
    visibility: String,
    checks: Vec<String>,
    reviewers: Vec<String>,
    sensitive_paths: Vec<String>,
    quarantine_lane: bool,
    min_trust_score: Option<String>,
    recipients: Vec<String>,
    revoked_recipients: Vec<String>,
    require_fresh_evidence: bool,
    evidence_max_age_ms: Option<u64>,
    trusted_runners: Vec<String>,
}

fn build_policy(options: PolicyBuildOptions) -> anyhow::Result<Policy> {
    let visibility = parse_visibility(&options.visibility)?;
    if let Some(score) = options.min_trust_score.as_deref() {
        validate_min_trust_score(score)?;
    }

    let default_evidence_policy = EvidencePolicy::default();
    let evidence_policy = EvidencePolicy {
        require_fresh_evidence: options.require_fresh_evidence,
        max_age_ms: options
            .evidence_max_age_ms
            .or(default_evidence_policy.max_age_ms),
        trusted_runner_identities: options.trusted_runners,
        ..default_evidence_policy
    };

    Ok(Policy {
        policy_id: options.id,
        required_checks: options.checks,
        required_reviewers: options.reviewers,
        sensitive_paths: options.sensitive_paths,
        quarantine_lane: options.quarantine_lane,
        min_trust_score: options.min_trust_score,
        visibility,
        authorized_recipients: options.recipients,
        revoked_recipients: options.revoked_recipients,
        evidence_policy,
    })
}

fn save_policy(
    store: &ClawStore,
    policy: &Policy,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<()> {
    let ref_name = format!("policies/{}", policy.policy_id);
    let old = store.get_ref(&ref_name)?;
    let planned_obj_id = policy_object_id(policy)?;

    if dry_run {
        if json {
            print_policy_apply_json(policy, &ref_name, old, planned_obj_id, true)?;
        } else {
            println!("Dry run: would save policy {}", policy.policy_id);
            println!("  Ref: {ref_name}");
            if let Some(old) = old {
                println!("  Previous object: {old}");
            }
            println!("  New object: {planned_obj_id}");
            println!("  Object write skipped.");
            println!("  Ref update skipped.");
        }
        return Ok(());
    }

    let obj_id = store.store_object(&Object::Policy(policy.clone()))?;
    store.update_ref_cas(
        &ref_name,
        old.as_ref(),
        &obj_id,
        "policy",
        "policy create/update",
    )?;

    if json {
        print_policy_apply_json(policy, &ref_name, old, obj_id, false)?;
    } else {
        println!("Saved policy: {}", policy.policy_id);
        println!("  Ref: {ref_name}");
        println!("  Object: {obj_id}");
    }

    Ok(())
}

fn print_policy_apply_json(
    policy: &Policy,
    ref_name: &str,
    old: Option<ObjectId>,
    new: ObjectId,
    dry_run: bool,
) -> anyhow::Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "dry_run": dry_run,
            "ref": ref_name,
            "old_object": old.map(|id| id.to_string()),
            "new_object": new.to_string(),
            "policy": policy,
        }))?
    );
    Ok(())
}

fn policy_object_id(policy: &Policy) -> anyhow::Result<ObjectId> {
    let object = Object::Policy(policy.clone());
    let payload = object.serialize_payload()?;
    Ok(content_hash(object.type_tag(), &payload))
}

struct EvalRequest {
    policy_id: String,
    revision_ref: String,
    capsule_ref: Option<String>,
    signer_agents: Vec<String>,
    signer_keys: Vec<String>,
    touched_paths: Vec<String>,
    trust_score: Option<String>,
    json: bool,
}

fn run_eval(request: EvalRequest) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;
    let (policy_ref, policy_obj_id, policy) = load_policy(&store, &request.policy_id)?;
    let (revision_id, revision) = load_revision(&store, &request.revision_ref)?;
    let (capsule_id, capsule) = match request.capsule_ref.as_deref() {
        Some(value) => load_capsule(&store, value)?,
        None => load_default_capsule(&store, &revision_id, &revision)?,
    };
    let context = PolicyContext {
        revision_id: Some(revision_id),
        signer_agent_ids: request.signer_agents,
        signer_key_ids: request.signer_keys,
        touched_paths: request.touched_paths,
        trust_score: request
            .trust_score
            .as_deref()
            .map(parse_trust_score)
            .transpose()?
            .or_else(|| derive_capsule_trust_score(&capsule)),
        now_ms: Some(current_time_ms()),
    };

    let evaluation = evaluate_policy(&policy, &revision, &capsule, &context);
    let error = evaluation.as_ref().err().map(ToString::to_string);
    let allowed = error.is_none();

    if request.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "allowed": allowed,
                "error": error,
                "policy": {
                    "id": &policy.policy_id,
                    "ref": &policy_ref,
                    "object": policy_obj_id.to_string(),
                },
                "revision": {
                    "id": revision_id.to_string(),
                    "hex": revision_id.to_hex(),
                },
                "capsule": {
                    "id": capsule_id.to_string(),
                    "hex": capsule_id.to_hex(),
                },
                "context": {
                    "signer_agent_ids": context.signer_agent_ids,
                    "signer_key_ids": context.signer_key_ids,
                    "touched_paths": context.touched_paths,
                    "trust_score": context.trust_score,
                    "now_ms": context.now_ms,
                }
            }))?
        );
    }

    if let Err(err) = evaluation {
        anyhow::bail!(
            "policy '{}' denied revision {}: {}",
            policy.policy_id,
            revision_id.to_hex(),
            err
        );
    }

    if !request.json {
        println!(
            "Policy '{}' allowed revision {}",
            policy.policy_id,
            revision_id.to_hex()
        );
    }

    Ok(())
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn load_policy(store: &ClawStore, id: &str) -> anyhow::Result<(String, ObjectId, Policy)> {
    let ref_name = if id.starts_with("policies/") {
        id.to_string()
    } else {
        format!("policies/{id}")
    };
    let obj_id = store
        .get_ref(&ref_name)?
        .ok_or_else(|| anyhow::anyhow!("policy not found: {ref_name}"))?;
    let obj = store.load_object(&obj_id)?;
    match obj {
        Object::Policy(policy) => Ok((ref_name, obj_id, policy)),
        _ => anyhow::bail!("ref does not point to a policy object: {ref_name}"),
    }
}

fn load_revision(store: &ClawStore, value: &str) -> anyhow::Result<(ObjectId, Revision)> {
    let id = resolve_object_ref_or_id(store, value)?;
    match store.load_object(&id)? {
        Object::Revision(revision) => Ok((id, revision)),
        _ => anyhow::bail!("not a revision: {value}"),
    }
}

fn load_capsule(store: &ClawStore, value: &str) -> anyhow::Result<(ObjectId, Capsule)> {
    let id = resolve_object_ref_or_id(store, value)?;
    load_capsule_id(store, id, value)
}

fn load_default_capsule(
    store: &ClawStore,
    revision_id: &ObjectId,
    revision: &Revision,
) -> anyhow::Result<(ObjectId, Capsule)> {
    if let Some(capsule_id) = revision.capsule_id {
        return load_capsule_id(store, capsule_id, &capsule_id.to_string());
    }

    for ref_name in [
        format!("capsules/by-revision/{}", revision_id.to_hex()),
        format!("capsules/{}", revision_id.to_hex()),
    ] {
        if let Some(capsule_id) = store.get_ref(&ref_name)? {
            return load_capsule_id(store, capsule_id, &ref_name);
        }
    }

    anyhow::bail!(
        "revision {} has no capsule; pass --capsule",
        revision_id.to_hex()
    )
}

fn load_capsule_id(
    store: &ClawStore,
    capsule_id: ObjectId,
    source: &str,
) -> anyhow::Result<(ObjectId, Capsule)> {
    match store.load_object(&capsule_id)? {
        Object::Capsule(capsule) => Ok((capsule_id, capsule)),
        _ => anyhow::bail!("not a capsule: {source}"),
    }
}

fn resolve_object_ref_or_id(store: &ClawStore, value: &str) -> anyhow::Result<ObjectId> {
    if let Some(id) = store.get_ref(value)? {
        return Ok(id);
    }
    if let Ok(id) = ObjectId::from_hex(value) {
        return Ok(id);
    }
    if let Ok(id) = ObjectId::from_display(value) {
        return Ok(id);
    }
    anyhow::bail!("cannot resolve object or ref: {value}")
}

fn parse_visibility(value: &str) -> anyhow::Result<Visibility> {
    match value.to_ascii_lowercase().as_str() {
        "public" => Ok(Visibility::Public),
        "private" => Ok(Visibility::Private),
        "encrypted-metadata-required" | "encrypted_metadata_required" | "restricted" => {
            Ok(Visibility::EncryptedMetadataRequired)
        }
        _ => anyhow::bail!(
            "unknown visibility '{}'; expected public|private|encrypted-metadata-required",
            value
        ),
    }
}

fn parse_trust_score(value: &str) -> anyhow::Result<f32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("min trust score cannot be empty");
    }

    let parsed = if let Some(percent) = trimmed.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow::anyhow!("invalid percentage trust score '{}'", value))?
            / 100.0
    } else {
        trimmed
            .parse::<f32>()
            .map_err(|_| anyhow::anyhow!("invalid trust score '{}'", value))?
    };

    if !(0.0..=1.0).contains(&parsed) {
        anyhow::bail!("min trust score '{}' must be between 0 and 1", value);
    }

    Ok(parsed)
}

fn validate_min_trust_score(value: &str) -> anyhow::Result<()> {
    parse_trust_score(value)?;
    Ok(())
}

fn derive_capsule_trust_score(capsule: &Capsule) -> Option<f32> {
    let total = capsule.public_fields.evidence.len();
    if total == 0 {
        return None;
    }

    let passed = capsule
        .public_fields
        .evidence
        .iter()
        .filter(|e| e.status.eq_ignore_ascii_case("pass"))
        .count();

    Some(passed as f32 / total as f32)
}

#[cfg(test)]
mod tests {
    use super::{
        build_policy, parse_visibility, validate_min_trust_score, PolicyArgs, PolicyBuildOptions,
        PolicyCommand,
    };
    use clap::Parser;
    use claw_core::types::Visibility;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        args: PolicyArgs,
    }

    #[test]
    fn parses_apply_dry_run() {
        let cli = TestCli::parse_from(["claw", "apply", "--id", "release", "--dry-run"]);

        match cli.args.command {
            PolicyCommand::Apply { id, dry_run, .. } => {
                assert_eq!(id, "release");
                assert!(dry_run);
            }
            _ => panic!("expected apply command"),
        }
    }

    #[test]
    fn parses_create_recipient_and_freshness_flags() {
        let cli = TestCli::parse_from([
            "claw",
            "create",
            "--id",
            "sensitive",
            "--recipient",
            "security",
            "--revoked-recipient",
            "former-reviewer",
            "--require-fresh-evidence",
            "--trusted-runner",
            "github-actions/release",
            "--evidence-max-age-ms",
            "60000",
        ]);

        match cli.args.command {
            PolicyCommand::Create {
                id,
                recipients,
                revoked_recipients,
                require_fresh_evidence,
                evidence_max_age_ms,
                trusted_runners,
                ..
            } => {
                assert_eq!(id, "sensitive");
                assert_eq!(recipients, vec!["security".to_string()]);
                assert_eq!(revoked_recipients, vec!["former-reviewer".to_string()]);
                assert!(require_fresh_evidence);
                assert_eq!(evidence_max_age_ms, Some(60_000));
                assert_eq!(trusted_runners, vec!["github-actions/release".to_string()]);
            }
            _ => panic!("expected create command"),
        }
    }

    #[test]
    fn parses_eval_json() {
        let cli = TestCli::parse_from([
            "claw",
            "eval",
            "release",
            "--revision",
            "heads/main",
            "--json",
        ]);

        match cli.args.command {
            PolicyCommand::Eval {
                id, revision, json, ..
            } => {
                assert_eq!(id, "release");
                assert_eq!(revision, "heads/main");
                assert!(json);
            }
            _ => panic!("expected eval command"),
        }
    }

    #[test]
    fn parses_visibility_values() {
        assert_eq!(parse_visibility("public").unwrap(), Visibility::Public);
        assert_eq!(parse_visibility("PRIVATE").unwrap(), Visibility::Private);
        assert_eq!(
            parse_visibility("encrypted-metadata-required").unwrap(),
            Visibility::EncryptedMetadataRequired
        );
        assert_eq!(
            parse_visibility("restricted").unwrap(),
            Visibility::EncryptedMetadataRequired
        );
    }

    #[test]
    fn rejects_unknown_visibility() {
        assert!(parse_visibility("secret").is_err());
    }

    #[test]
    fn validates_min_trust_score() {
        assert!(validate_min_trust_score("0.75").is_ok());
        assert!(validate_min_trust_score("80%").is_ok());
        assert!(validate_min_trust_score("1.5").is_err());
        assert!(validate_min_trust_score("abc").is_err());
    }

    #[test]
    fn build_policy_preserves_default_freshness_max_age() {
        let policy = build_policy(PolicyBuildOptions {
            id: "fresh".to_string(),
            visibility: "public".to_string(),
            checks: vec!["test".to_string()],
            reviewers: vec![],
            sensitive_paths: vec![],
            quarantine_lane: false,
            min_trust_score: None,
            recipients: vec![],
            revoked_recipients: vec![],
            require_fresh_evidence: true,
            evidence_max_age_ms: None,
            trusted_runners: vec![],
        })
        .unwrap();

        assert!(policy.evidence_policy.require_fresh_evidence);
        assert_eq!(
            policy.evidence_policy.max_age_ms,
            Some(24 * 60 * 60 * 1_000)
        );
    }
}

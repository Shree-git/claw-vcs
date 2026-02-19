use clap::{Args, Subcommand};

use claw_core::object::Object;
use claw_core::types::{Policy, Visibility};
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
        /// Visibility: public|private|restricted
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
        /// Optional minimum trust score label
        #[arg(long)]
        min_trust_score: Option<String>,
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
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let visibility = parse_visibility(&visibility)?;

            let policy = Policy {
                policy_id: id.clone(),
                required_checks: checks,
                required_reviewers: reviewers,
                sensitive_paths,
                quarantine_lane,
                min_trust_score,
                visibility,
            };

            let obj_id = store.store_object(&Object::Policy(policy.clone()))?;
            let ref_name = format!("policies/{}", id);
            let old = store.get_ref(&ref_name)?;
            store.update_ref_cas(
                &ref_name,
                old.as_ref(),
                &obj_id,
                "policy",
                "policy create/update",
            )?;

            println!("Saved policy: {}", policy.policy_id);
            println!("  Ref: {ref_name}");
            println!("  Object: {obj_id}");
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

fn parse_visibility(value: &str) -> anyhow::Result<Visibility> {
    match value.to_ascii_lowercase().as_str() {
        "public" => Ok(Visibility::Public),
        "private" => Ok(Visibility::Private),
        "restricted" => Ok(Visibility::Restricted),
        _ => anyhow::bail!(
            "unknown visibility '{}'; expected public|private|restricted",
            value
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_visibility;
    use claw_core::types::Visibility;

    #[test]
    fn parses_visibility_values() {
        assert_eq!(parse_visibility("public").unwrap(), Visibility::Public);
        assert_eq!(parse_visibility("PRIVATE").unwrap(), Visibility::Private);
        assert_eq!(
            parse_visibility("restricted").unwrap(),
            Visibility::Restricted
        );
    }

    #[test]
    fn rejects_unknown_visibility() {
        assert!(parse_visibility("secret").is_err());
    }
}

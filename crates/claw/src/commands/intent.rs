use clap::{Args, Subcommand};

use claw_core::id::IntentId;
use claw_core::object::Object;
use claw_core::types::{Intent, IntentStatus};
use claw_store::ClawStore;

use crate::config::find_repo_root;

#[derive(Args)]
pub struct IntentArgs {
    /// Output result as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: IntentCommand,
}

#[derive(Subcommand)]
enum IntentCommand {
    /// Create a new intent
    #[command(alias = "create")]
    New {
        /// Intent title
        #[arg(short, long)]
        title: String,
        /// Intent goal
        #[arg(short, long, default_value = "")]
        goal: String,
    },
    /// Show an intent
    Show {
        /// Intent ID (ULID)
        id: String,
    },
    /// List intents
    List,
    /// Update an intent
    Update {
        /// Intent ID (ULID)
        id: String,
        /// New status
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Manage policy references attached to an intent
    Policy {
        #[command(subcommand)]
        command: IntentPolicyCommand,
    },
}

#[derive(Subcommand)]
enum IntentPolicyCommand {
    /// List policy references attached to an intent
    List {
        /// Intent ID (ULID)
        id: String,
    },
    /// Attach a policy reference to an intent
    Add {
        /// Intent ID (ULID)
        id: String,
        /// Policy ID or ref, for example `ci-required` or `policies/ci-required`
        policy_ref: String,
        /// Validate and print the planned change without writing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove a policy reference from an intent
    Remove {
        /// Intent ID (ULID)
        id: String,
        /// Policy ID or ref to remove
        policy_ref: String,
        /// Validate and print the planned change without writing it
        #[arg(long)]
        dry_run: bool,
    },
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use clap::Parser;

    use super::{
        add_policy_ref, normalized_policy_ref, remove_policy_ref, IntentArgs, IntentCommand,
        IntentPolicyCommand,
    };

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        args: IntentArgs,
    }

    #[test]
    fn parse_create_alias_as_new() {
        let cli = TestCli::parse_from(["claw", "create", "--title", "hello"]);

        match cli.args.command {
            IntentCommand::New { title, goal } => {
                assert_eq!(title, "hello");
                assert_eq!(goal, "");
            }
            _ => panic!("expected new command"),
        }
    }

    #[test]
    fn parse_policy_add_dry_run() {
        let cli = TestCli::parse_from([
            "claw",
            "policy",
            "add",
            "01H00000000000000000000000",
            "ci-required",
            "--dry-run",
        ]);

        match cli.args.command {
            IntentCommand::Policy {
                command:
                    IntentPolicyCommand::Add {
                        id,
                        policy_ref,
                        dry_run,
                    },
            } => {
                assert_eq!(id, "01H00000000000000000000000");
                assert_eq!(policy_ref, "ci-required");
                assert!(dry_run);
            }
            _ => panic!("expected policy add command"),
        }
    }

    #[test]
    fn policy_ref_helpers_are_idempotent() {
        let mut refs = Vec::new();

        assert!(add_policy_ref(&mut refs, "ci-required".to_string()));
        assert!(!add_policy_ref(&mut refs, "ci-required".to_string()));
        assert_eq!(refs, vec!["ci-required"]);
        assert!(remove_policy_ref(&mut refs, "ci-required"));
        assert!(!remove_policy_ref(&mut refs, "ci-required"));
        assert!(refs.is_empty());
    }

    #[test]
    fn policy_ref_normalization_strips_repo_prefix() {
        assert_eq!(
            normalized_policy_ref("policies/ci-required").unwrap(),
            "ci-required"
        );
        assert_eq!(
            normalized_policy_ref(" ci-required ").unwrap(),
            "ci-required"
        );
        assert!(normalized_policy_ref("  ").is_err());
    }
}

pub fn run(args: IntentArgs) -> anyhow::Result<()> {
    let json = args.json;
    match args.command {
        IntentCommand::New { title, goal } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64;

            let intent = Intent {
                id: IntentId::new(),
                title: title.clone(),
                goal,
                constraints: vec![],
                acceptance_tests: vec![],
                links: vec![],
                policy_refs: vec![],
                agents: vec![],
                change_ids: vec![],
                depends_on: vec![],
                supersedes: vec![],
                status: IntentStatus::Open,
                created_at_ms: now,
                updated_at_ms: now,
            };

            let id = store.store_object(&Object::Intent(intent.clone()))?;
            store.set_ref(&format!("intents/{}", intent.id), &id)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "created": true,
                        "intent": intent_json(&intent, Some(id.to_hex())),
                    }))?
                );
            } else {
                println!("Created intent: {}", intent.id);
                println!("  Title: {title}");
                println!("  Object: {id}");
            }
        }
        IntentCommand::Show { id } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let obj_id = store.get_ref(&format!("intents/{id}"))?.ok_or_else(|| {
                anyhow::anyhow!(
                    "intent not found: {id}. Run `claw intent list` to inspect available intents."
                )
            })?;
            let obj = store.load_object(&obj_id)?;
            if let Object::Intent(intent) = obj {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "intent": intent_json(&intent, Some(obj_id.to_hex())),
                        }))?
                    );
                } else {
                    println!("Intent: {}", intent.id);
                    println!("  Title: {}", intent.title);
                    println!("  Status: {:?}", intent.status);
                    println!("  Goal: {}", intent.goal);
                }
            }
        }
        IntentCommand::List => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let refs = store.list_refs("intents")?;
            if json {
                let intents = refs
                    .iter()
                    .filter_map(|(_name, id)| match store.load_object(id) {
                        Ok(Object::Intent(intent)) => Some(intent_json(&intent, Some(id.to_hex()))),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "intents": intents,
                    }))?
                );
            } else if refs.is_empty() {
                println!("No intents found.");
            } else {
                for (_name, id) in &refs {
                    if let Ok(Object::Intent(intent)) = store.load_object(id) {
                        println!("{} {:?} {}", intent.id, intent.status, intent.title);
                    }
                }
            }
        }
        IntentCommand::Update { id, status } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let (mut intent, _) = load_intent(&store, &id)?;
            if let Some(s) = status {
                intent.status = match s.to_lowercase().as_str() {
                    "open" => IntentStatus::Open,
                    "blocked" => IntentStatus::Blocked,
                    "done" => IntentStatus::Done,
                    "superseded" => IntentStatus::Superseded,
                    _ => anyhow::bail!(
                        "unknown status: {s}. Expected one of: open, blocked, done, superseded."
                    ),
                };
            }
            intent.updated_at_ms = current_time_ms()?;
            let new_id = store_intent(&store, &intent)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "updated": true,
                        "intent": intent_json(&intent, Some(new_id.to_hex())),
                    }))?
                );
            } else {
                println!("Updated intent: {}", intent.id);
            }
        }
        IntentCommand::Policy { command } => run_policy_command(command, json)?,
    }
    Ok(())
}

fn run_policy_command(command: IntentPolicyCommand, json: bool) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    match command {
        IntentPolicyCommand::List { id } => {
            let (intent, object_id) = load_intent(&store, &id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "intent_id": intent.id.to_string(),
                        "object_id": object_id.to_hex(),
                        "policy_refs": intent.policy_refs,
                    }))?
                );
            } else if intent.policy_refs.is_empty() {
                println!("No policy refs attached to intent {}", intent.id);
            } else {
                for policy_ref in &intent.policy_refs {
                    println!("{policy_ref}");
                }
            }
        }
        IntentPolicyCommand::Add {
            id,
            policy_ref,
            dry_run,
        } => {
            let policy_ref = normalized_policy_ref(&policy_ref)?;
            ensure_policy_ref_exists(&store, &policy_ref)?;
            let (mut intent, old_object_id) = load_intent(&store, &id)?;
            let changed = add_policy_ref(&mut intent.policy_refs, policy_ref.clone());
            let new_object_id = if !dry_run && changed {
                intent.updated_at_ms = current_time_ms()?;
                Some(store_intent(&store, &intent)?)
            } else {
                None
            };
            print_policy_update(
                &intent,
                &policy_ref,
                "add",
                changed,
                dry_run,
                Some(old_object_id.to_hex()),
                new_object_id.map(|id| id.to_hex()),
                json,
            )?;
        }
        IntentPolicyCommand::Remove {
            id,
            policy_ref,
            dry_run,
        } => {
            let policy_ref = normalized_policy_ref(&policy_ref)?;
            let (mut intent, old_object_id) = load_intent(&store, &id)?;
            let changed = remove_policy_ref(&mut intent.policy_refs, &policy_ref);
            let new_object_id = if !dry_run && changed {
                intent.updated_at_ms = current_time_ms()?;
                Some(store_intent(&store, &intent)?)
            } else {
                None
            };
            print_policy_update(
                &intent,
                &policy_ref,
                "remove",
                changed,
                dry_run,
                Some(old_object_id.to_hex()),
                new_object_id.map(|id| id.to_hex()),
                json,
            )?;
        }
    }

    Ok(())
}

fn load_intent(store: &ClawStore, id: &str) -> anyhow::Result<(Intent, claw_core::id::ObjectId)> {
    let obj_id = store.get_ref(&format!("intents/{id}"))?.ok_or_else(|| {
        anyhow::anyhow!(
            "intent not found: {id}. Run `claw intent list` to inspect available intents."
        )
    })?;
    let obj = store.load_object(&obj_id)?;
    match obj {
        Object::Intent(intent) => Ok((intent, obj_id)),
        _ => anyhow::bail!("intent ref points to a non-intent object: intents/{id}"),
    }
}

fn store_intent(store: &ClawStore, intent: &Intent) -> anyhow::Result<claw_core::id::ObjectId> {
    let id = store.store_object(&Object::Intent(intent.clone()))?;
    store.set_ref(&format!("intents/{}", intent.id), &id)?;
    Ok(id)
}

fn normalized_policy_ref(policy_ref: &str) -> anyhow::Result<String> {
    let value = policy_ref.trim();
    if value.is_empty() {
        anyhow::bail!("policy ref cannot be empty");
    }
    Ok(value.strip_prefix("policies/").unwrap_or(value).to_string())
}

fn ensure_policy_ref_exists(store: &ClawStore, policy_ref: &str) -> anyhow::Result<()> {
    if store.get_ref(policy_ref)?.is_some() {
        return Ok(());
    }
    let prefixed = format!("policies/{policy_ref}");
    if store.get_ref(&prefixed)?.is_some() {
        return Ok(());
    }
    anyhow::bail!(
        "policy ref not found: {policy_ref}. Run `claw policy show {policy_ref}` or `claw policy create --id {policy_ref}`."
    )
}

fn add_policy_ref(policy_refs: &mut Vec<String>, policy_ref: String) -> bool {
    if policy_refs.iter().any(|existing| existing == &policy_ref) {
        return false;
    }
    policy_refs.push(policy_ref);
    policy_refs.sort();
    true
}

fn remove_policy_ref(policy_refs: &mut Vec<String>, policy_ref: &str) -> bool {
    let original_len = policy_refs.len();
    policy_refs.retain(|existing| existing != policy_ref);
    original_len != policy_refs.len()
}

#[allow(clippy::too_many_arguments)]
fn print_policy_update(
    intent: &Intent,
    policy_ref: &str,
    operation: &str,
    changed: bool,
    dry_run: bool,
    old_object_id: Option<String>,
    new_object_id: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "operation": operation,
                "changed": changed,
                "dry_run": dry_run,
                "policy_ref": policy_ref,
                "old_object_id": old_object_id,
                "new_object_id": new_object_id,
                "intent": intent_json(intent, new_object_id),
            }))?
        );
    } else if dry_run {
        let verb = if changed {
            "would update"
        } else {
            "already unchanged"
        };
        println!("{verb}: intent {} policy ref {policy_ref}", intent.id);
    } else if changed {
        println!("Updated intent {} policy ref {policy_ref}", intent.id);
    } else {
        println!("No change: intent {} policy ref {policy_ref}", intent.id);
    }
    Ok(())
}

fn current_time_ms() -> anyhow::Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64)
}

fn intent_json(intent: &claw_core::types::Intent, object_id: Option<String>) -> serde_json::Value {
    serde_json::json!({
        "id": intent.id.to_string(),
        "object_id": object_id,
        "title": intent.title,
        "goal": intent.goal,
        "status": intent_status(intent.status),
        "policy_refs": intent.policy_refs,
        "change_ids": intent.change_ids,
        "created_at_ms": intent.created_at_ms,
        "updated_at_ms": intent.updated_at_ms,
    })
}

fn intent_status(status: IntentStatus) -> &'static str {
    match status {
        IntentStatus::Open => "open",
        IntentStatus::Blocked => "blocked",
        IntentStatus::Done => "done",
        IntentStatus::Superseded => "superseded",
    }
}

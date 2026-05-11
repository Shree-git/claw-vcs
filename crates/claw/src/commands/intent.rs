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
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use clap::Parser;

    use super::{IntentArgs, IntentCommand};

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
            let obj_id = store.get_ref(&format!("intents/{id}"))?.ok_or_else(|| {
                anyhow::anyhow!(
                    "intent not found: {id}. Run `claw intent list` to inspect available intents."
                )
            })?;
            let obj = store.load_object(&obj_id)?;
            if let Object::Intent(mut intent) = obj {
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
                intent.updated_at_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis() as u64;
                let new_id = store.store_object(&Object::Intent(intent.clone()))?;
                store.set_ref(&format!("intents/{}", intent.id), &new_id)?;
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
        }
    }
    Ok(())
}

fn intent_json(intent: &claw_core::types::Intent, object_id: Option<String>) -> serde_json::Value {
    serde_json::json!({
        "id": intent.id.to_string(),
        "object_id": object_id,
        "title": intent.title,
        "goal": intent.goal,
        "status": intent_status(intent.status),
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

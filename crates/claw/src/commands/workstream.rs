use clap::{Args, Subcommand};

use claw_core::id::ChangeId;
use claw_core::object::Object;
use claw_core::types::Workstream;
use claw_store::ClawStore;

use crate::config::find_repo_root;

#[derive(Args)]
pub struct WorkstreamArgs {
    #[command(subcommand)]
    command: WorkstreamCommand,
}

#[derive(Subcommand)]
enum WorkstreamCommand {
    /// Create a new workstream
    Create {
        /// Workstream ID
        #[arg(long)]
        id: String,
    },
    /// Show a workstream
    Show {
        /// Workstream ID
        id: String,
    },
    /// List workstreams
    List,
    /// Add a change to a workstream
    Push {
        /// Workstream ID
        #[arg(long)]
        id: String,
        /// Change ID to add
        #[arg(long)]
        change: String,
    },
    /// Remove the top change from a workstream
    Pop {
        /// Workstream ID
        #[arg(long)]
        id: String,
    },
}

pub fn run(args: WorkstreamArgs) -> anyhow::Result<()> {
    match args.command {
        WorkstreamCommand::Create { id } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            let ws = Workstream {
                workstream_id: id.clone(),
                change_stack: vec![],
            };

            let obj_id = store.store_object(&Object::Workstream(ws))?;
            store.set_ref(&format!("workstreams/{}", id), &obj_id)?;

            println!("Created workstream: {}", id);
            println!("  Object: {}", obj_id);
        }
        WorkstreamCommand::Show { id } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            let ref_name = format!("workstreams/{}", id);
            let obj_id = store
                .get_ref(&ref_name)?
                .ok_or_else(|| anyhow::anyhow!("workstream not found: {}", id))?;
            let obj = store.load_object(&obj_id)?;

            if let Object::Workstream(ws) = obj {
                println!("Workstream: {}", ws.workstream_id);
                println!("  Changes: {}", ws.change_stack.len());
                for (i, cid) in ws.change_stack.iter().enumerate() {
                    println!("    [{}] {}", i, cid);
                }
            }
        }
        WorkstreamCommand::List => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            let refs = store.list_refs("workstreams/")?;
            if refs.is_empty() {
                println!("No workstreams found.");
            } else {
                for (name, id) in &refs {
                    let short_name = name.strip_prefix("workstreams/").unwrap_or(name);
                    if let Ok(Object::Workstream(ws)) = store.load_object(id) {
                        println!("{} ({} changes)", short_name, ws.change_stack.len());
                    } else {
                        println!("{}", short_name);
                    }
                }
            }
        }
        WorkstreamCommand::Push { id, change } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            let ref_name = format!("workstreams/{}", id);
            let obj_id = store
                .get_ref(&ref_name)?
                .ok_or_else(|| anyhow::anyhow!("workstream not found: {}", id))?;
            let obj = store.load_object(&obj_id)?;

            if let Object::Workstream(mut ws) = obj {
                let change_id = ChangeId::from_string(&change)?;
                ws.change_stack.push(change_id);
                let new_id = store.store_object(&Object::Workstream(ws))?;
                store.set_ref(&ref_name, &new_id)?;
                println!("Pushed change {} onto workstream {}", change, id);
            } else {
                anyhow::bail!("ref does not point to a workstream: {}", ref_name);
            }
        }
        WorkstreamCommand::Pop { id } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            let ref_name = format!("workstreams/{}", id);
            let obj_id = store
                .get_ref(&ref_name)?
                .ok_or_else(|| anyhow::anyhow!("workstream not found: {}", id))?;
            let obj = store.load_object(&obj_id)?;

            if let Object::Workstream(mut ws) = obj {
                if let Some(popped) = ws.change_stack.pop() {
                    let new_id = store.store_object(&Object::Workstream(ws))?;
                    store.set_ref(&ref_name, &new_id)?;
                    println!("Popped change {} from workstream {}", popped, id);
                } else {
                    println!("Workstream {} has no changes to pop.", id);
                }
            } else {
                anyhow::bail!("ref does not point to a workstream: {}", ref_name);
            }
        }
    }

    Ok(())
}

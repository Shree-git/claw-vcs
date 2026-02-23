use clap::{Args, Subcommand};

use claw_store::ClawStore;

use crate::config::find_repo_root;

#[derive(Args)]
pub struct TagArgs {
    #[command(subcommand)]
    command: Option<TagCommand>,
}

#[derive(Subcommand)]
enum TagCommand {
    /// Create a new tag
    Create {
        /// Tag name (e.g. v1.0.0)
        name: String,
        /// Object to tag (default: HEAD)
        #[arg(long)]
        target: Option<String>,
        /// Tag message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Delete a tag
    Delete {
        /// Tag name
        name: String,
    },
}

pub fn run(args: TagArgs) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    match args.command {
        None => {
            // List tags
            let refs = store.list_refs("tags/")?;
            if refs.is_empty() {
                println!("No tags found.");
            } else {
                for (name, id) in &refs {
                    let short_name = name.strip_prefix("tags/").unwrap_or(name);
                    let short_id = &id.to_hex()[..12];
                    println!("{} ({})", short_name, short_id);
                }
            }
        }
        Some(TagCommand::Create {
            name,
            target,
            message,
        }) => {
            let ref_name = format!("tags/{}", name);
            if store.get_ref(&ref_name)?.is_some() {
                anyhow::bail!("tag '{}' already exists", name);
            }

            let target_id = match target {
                Some(t) => {
                    // Try as ref, then hex, then display
                    if let Some(id) = store.get_ref(&t)? {
                        id
                    } else if let Ok(id) = claw_core::id::ObjectId::from_hex(&t) {
                        if !store.has_object(&id) {
                            anyhow::bail!("object not found: {}", t);
                        }
                        id
                    } else {
                        anyhow::bail!("cannot resolve: {}", t);
                    }
                }
                None => store
                    .resolve_head()?
                    .ok_or_else(|| anyhow::anyhow!("no commits yet"))?,
            };

            let msg = message.as_deref().unwrap_or("tag");
            store.update_ref_cas(&ref_name, None, &target_id, "tag", msg)?;
            println!("Created tag '{}' at {}", name, target_id);
        }
        Some(TagCommand::Delete { name }) => {
            let ref_name = format!("tags/{}", name);
            if store.get_ref(&ref_name)?.is_none() {
                anyhow::bail!("tag '{}' not found", name);
            }
            store.delete_ref(&ref_name)?;
            println!("Deleted tag '{}'", name);
        }
    }

    Ok(())
}

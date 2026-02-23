use clap::Args;

use claw_store::reflog::{read_reflog, RefLogLine};
use claw_store::{ClawStore, HeadState};

use crate::config::find_repo_root;

#[derive(Args)]
pub struct ReflogArgs {
    /// Ref to show reflog for (default: current branch)
    #[arg(long, name = "ref")]
    ref_name: Option<String>,
    /// Maximum number of entries
    #[arg(long, default_value = "20")]
    limit: usize,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

pub fn run(args: ReflogArgs) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    let ref_name = match args.ref_name {
        Some(r) => r,
        None => {
            let head = store.read_head()?;
            match head {
                HeadState::Symbolic { ref_name } => ref_name,
                HeadState::Detached { .. } => {
                    anyhow::bail!(
                        "HEAD is detached; specify --ref to view a specific reflog"
                    );
                }
            }
        }
    };

    let entries = read_reflog(store.layout(), &ref_name)?;

    if entries.is_empty() {
        println!("No reflog entries for '{}'.", ref_name);
        return Ok(());
    }

    let display: Vec<&RefLogLine> = entries.iter().rev().take(args.limit).collect();

    if args.json {
        let json_entries: Vec<serde_json::Value> = display
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                serde_json::json!({
                    "index": i,
                    "old": entry.old.to_hex(),
                    "new": entry.new.to_hex(),
                    "timestamp_ms": entry.timestamp_ms,
                    "author": entry.author,
                    "message": entry.message,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_entries)?);
    } else {
        let branch = ref_name.strip_prefix("heads/").unwrap_or(&ref_name);
        for (i, entry) in display.iter().enumerate() {
            let short_new = &entry.new.to_hex()[..12];
            println!(
                "{}@{{{}}}: {} {} {}",
                branch, i, short_new, entry.author, entry.message
            );
        }
    }

    Ok(())
}

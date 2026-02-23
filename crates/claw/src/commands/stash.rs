use clap::{Args, Subcommand};

use claw_core::object::Object;
use claw_core::types::Revision;
use claw_store::tree_diff::diff_trees;
use claw_store::{ClawStore, HeadState};

use crate::config::find_repo_root;
use crate::ignore::IgnoreRules;
use crate::worktree;

#[derive(Args)]
pub struct StashArgs {
    #[command(subcommand)]
    command: Option<StashCommand>,
}

#[derive(Subcommand)]
enum StashCommand {
    /// Save working tree changes to the stash
    Save {
        /// Stash message
        #[arg(short, long, default_value = "WIP")]
        message: String,
    },
    /// Restore the most recent stash entry
    Pop,
    /// List stash entries
    List,
    /// Drop a stash entry
    Drop {
        /// Stash index (default: 0)
        #[arg(default_value = "0")]
        index: usize,
    },
}

pub fn run(args: StashArgs) -> anyhow::Result<()> {
    let command = args.command.unwrap_or(StashCommand::Save {
        message: "WIP".to_string(),
    });

    match command {
        StashCommand::Save { message } => stash_save(message),
        StashCommand::Pop => stash_pop(),
        StashCommand::List => stash_list(),
        StashCommand::Drop { index } => stash_drop(index),
    }
}

fn stash_save(message: String) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;
    let ignore = IgnoreRules::load(&root);

    let head_state = store.read_head()?;
    let _branch_ref = match &head_state {
        HeadState::Symbolic { ref_name } => ref_name.clone(),
        HeadState::Detached { .. } => anyhow::bail!("cannot stash in detached HEAD state"),
    };

    let head_id = store
        .resolve_head()?
        .ok_or_else(|| anyhow::anyhow!("no commits yet; nothing to stash"))?;

    // Scan worktree
    let worktree_tree = worktree::scan_worktree(&store, &root, &ignore)?;

    // Check for actual changes
    let head_obj = store.load_object(&head_id)?;
    let head_tree = match head_obj {
        Object::Revision(ref rev) => rev.tree,
        _ => None,
    };

    let changes = diff_trees(&store, head_tree.as_ref(), Some(&worktree_tree), "")?;
    if changes.is_empty() {
        println!("No changes to stash.");
        return Ok(());
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    // Create a stash revision pointing at the worktree tree
    let stash_rev = Revision {
        change_id: None,
        parents: vec![head_id],
        patches: vec![],
        snapshot_base: None,
        tree: Some(worktree_tree),
        capsule_id: None,
        author: "stash".to_string(),
        created_at_ms: now_ms,
        summary: format!("stash: {}", message),
        policy_evidence: vec![],
    };

    let stash_id = store.store_object(&Object::Revision(stash_rev))?;

    // Push onto the stash stack
    let stash_refs = store.list_refs("stash/")?;
    let next_index = stash_refs.len();
    let stash_ref = format!("stash/{}", next_index);
    store.set_ref(&stash_ref, &stash_id)?;

    // Restore working tree to HEAD state
    if let Some(ref tree_id) = head_tree {
        worktree::materialize_tree(&store, tree_id, &root)?;
    }

    println!(
        "Saved working tree ({} file(s) changed): {}",
        changes.len(),
        message
    );

    Ok(())
}

fn stash_pop() -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    let stash_refs = store.list_refs("stash/")?;
    if stash_refs.is_empty() {
        anyhow::bail!("no stash entries");
    }

    // Find highest index
    let (max_ref, max_id) = stash_refs
        .iter()
        .max_by_key(|(name, _)| {
            name.strip_prefix("stash/")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0)
        })
        .unwrap();

    let stash_obj = store.load_object(max_id)?;
    let stash_tree = match stash_obj {
        Object::Revision(ref rev) => rev
            .tree
            .ok_or_else(|| anyhow::anyhow!("stash revision has no tree"))?,
        _ => anyhow::bail!("stash entry is not a revision"),
    };

    // Restore stashed tree to working directory
    worktree::materialize_tree(&store, &stash_tree, &root)?;

    // Remove the stash ref
    store.delete_ref(max_ref)?;

    println!("Restored stash and dropped {}", max_ref);
    Ok(())
}

fn stash_list() -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    let mut stash_refs = store.list_refs("stash/")?;
    if stash_refs.is_empty() {
        println!("No stash entries.");
        return Ok(());
    }

    // Sort by index descending (newest first)
    stash_refs.sort_by_key(|(name, _)| {
        name.strip_prefix("stash/")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
    });
    stash_refs.reverse();

    for (name, id) in &stash_refs {
        let index = name.strip_prefix("stash/").unwrap_or(name);
        let summary = match store.load_object(id) {
            Ok(Object::Revision(rev)) => rev.summary,
            _ => "(unknown)".to_string(),
        };
        println!("stash@{{{}}}: {}", index, summary);
    }

    Ok(())
}

fn stash_drop(index: usize) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    let ref_name = format!("stash/{}", index);
    if store.get_ref(&ref_name)?.is_none() {
        anyhow::bail!("stash@{{{}}} not found", index);
    }

    store.delete_ref(&ref_name)?;
    println!("Dropped stash@{{{}}}", index);
    Ok(())
}

use clap::Args;
use std::path::Path;

use claw_git::importer::{list_git_refs, GitImporter};
use claw_store::ClawStore;

use super::git_notes::{import_note_into_store, read_note};
use crate::config::find_repo_root;

#[derive(Args)]
pub struct GitImportArgs {
    /// Git ref to import (e.g. refs/heads/main)
    #[arg(long, default_value = "refs/heads/main")]
    git_ref: String,
    /// Destination claw ref (e.g. heads/main)
    #[arg(long, name = "ref", default_value = "heads/main")]
    ref_name: String,
    /// Path to .git directory
    #[arg(long, default_value = ".git")]
    git_dir: String,
    /// Import every git branch under refs/heads/*
    #[arg(long)]
    all_branches: bool,
    /// Destination claw ref prefix used with --all-branches
    #[arg(long, default_value = "heads/")]
    head_prefix: String,
    /// Import Claw provenance from git notes
    #[arg(long)]
    read_notes: bool,
    /// Git notes ref used when --read-notes is enabled
    #[arg(long, default_value = "claw")]
    notes_ref: String,
}

fn validate_ref_path(ref_name: &str) -> anyhow::Result<()> {
    let path = Path::new(ref_name);
    if path.is_absolute() {
        anyhow::bail!("invalid ref '{}': must be relative", ref_name);
    }

    for component in path.components() {
        match component {
            std::path::Component::Normal(_) => {}
            std::path::Component::CurDir
            | std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                anyhow::bail!(
                    "invalid ref '{}': cannot contain '.', '..', or root components",
                    ref_name
                );
            }
        }
    }

    Ok(())
}

pub fn run(args: GitImportArgs) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;
    let git_dir = root.join(&args.git_dir);

    let mut importer = GitImporter::new(&store);
    if args.all_branches {
        let prefix = normalize_head_prefix(&args.head_prefix);
        validate_ref_path(prefix.trim_end_matches('/'))?;

        let refs = list_git_refs(&git_dir, "refs/heads/")?;
        if refs.is_empty() {
            anyhow::bail!("no git branches found under refs/heads/");
        }

        let mut imported = 0usize;
        for (git_ref, _sha) in refs {
            let short = git_ref.strip_prefix("refs/heads/").unwrap_or(&git_ref);
            let claw_ref = format!("{prefix}{short}");
            validate_ref_path(&claw_ref)?;
            let revision_id = importer.import_ref(&git_dir, &git_ref, &claw_ref)?;
            println!(
                "Imported {git_ref} -> {claw_ref} ({})",
                revision_id.to_hex()
            );
            imported += 1;
        }
        println!("Imported {imported} branch(es) from git.");
    } else {
        validate_ref_path(&args.ref_name)?;
        let revision_id = importer.import_ref(&git_dir, &args.git_ref, &args.ref_name)?;
        println!("Imported git ref {} -> {}", args.git_ref, args.ref_name);
        println!("  Revision: {}", revision_id.to_hex());
    }

    if args.read_notes {
        let imported_notes =
            import_notes_for_imported_commits(&store, &importer, &git_dir, &args.notes_ref)?;
        println!(
            "Imported {imported_notes} provenance note(s) from refs/notes/{}",
            args.notes_ref
        );
    }

    Ok(())
}

fn normalize_head_prefix(prefix: &str) -> String {
    if prefix.is_empty() {
        return "heads/".to_string();
    }
    if prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{prefix}/")
    }
}

fn import_notes_for_imported_commits(
    store: &ClawStore,
    importer: &GitImporter<'_>,
    git_dir: &std::path::Path,
    notes_ref: &str,
) -> anyhow::Result<usize> {
    let mut imported = 0usize;
    for (commit_sha, revision_id) in importer.imported_commits() {
        let commit_hex = hex::encode(commit_sha);
        let Some(note) = read_note(git_dir, notes_ref, &commit_hex)? else {
            continue;
        };
        import_note_into_store(store, &revision_id, note)?;
        imported += 1;
    }
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::{normalize_head_prefix, validate_ref_path};

    #[test]
    fn allows_relative_ref_paths() {
        assert!(validate_ref_path("heads/main").is_ok());
        assert!(validate_ref_path("heads/imported/main").is_ok());
    }

    #[test]
    fn rejects_parent_and_root_components() {
        assert!(validate_ref_path("../outside").is_err());
        assert!(validate_ref_path("heads/../outside").is_err());
        assert!(validate_ref_path("/absolute").is_err());
    }

    #[test]
    fn normalizes_head_prefix() {
        assert_eq!(normalize_head_prefix("heads"), "heads/");
        assert_eq!(
            normalize_head_prefix("imports/branches/"),
            "imports/branches/"
        );
        assert_eq!(normalize_head_prefix(""), "heads/");
    }
}

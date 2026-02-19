use clap::Args;
use std::path::Path;

use claw_git::importer::GitImporter;
use claw_store::ClawStore;

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
    validate_ref_path(&args.ref_name)?;

    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;
    let git_dir = root.join(&args.git_dir);

    let mut importer = GitImporter::new(&store);
    let revision_id = importer.import_ref(&git_dir, &args.git_ref, &args.ref_name)?;

    println!("Imported git ref {} -> {}", args.git_ref, args.ref_name);
    println!("  Revision: {}", revision_id.to_hex());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_ref_path;

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
}

use std::collections::BTreeMap;
use std::path::Path;

use clap::{Args, Subcommand};

use crate::config::find_repo_root;

#[derive(Args)]
pub struct RemoteArgs {
    /// Output result as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: RemoteCommand,
}

#[derive(Subcommand)]
enum RemoteCommand {
    /// Add a remote
    Add {
        /// Remote name
        name: String,
        /// URL for gRPC remotes or base URL for clawlab remotes
        url: String,
        /// Transport kind (grpc|clawlab)
        #[arg(long, default_value = "grpc")]
        kind: String,
        /// Repository slug for clawlab remotes
        #[arg(long)]
        repo: Option<String>,
        /// Auth profile for transport bearer token (grpc/clawlab)
        #[arg(long)]
        token_profile: Option<String>,
        /// Preview without updating .claw/remotes.toml
        #[arg(long)]
        dry_run: bool,
    },
    /// List remotes
    List,
    /// Remove a remote
    Remove {
        /// Remote name
        name: String,
        /// Preview without updating .claw/remotes.toml
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub(crate) struct RemotesConfig {
    #[serde(default)]
    pub remotes: BTreeMap<String, RemoteEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub(crate) struct RemoteEntry {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub token_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ResolvedRemote {
    Grpc {
        addr: String,
        token_profile: Option<String>,
    },
    ClawLab {
        base_url: String,
        repo: String,
        token_profile: Option<String>,
    },
}

pub fn run(args: RemoteArgs) -> anyhow::Result<()> {
    match args.command {
        RemoteCommand::Add {
            name,
            url,
            kind,
            repo,
            token_profile,
            dry_run,
        } => run_add(&name, &url, &kind, repo, token_profile, args.json, dry_run),
        RemoteCommand::List => run_list(args.json),
        RemoteCommand::Remove { name, dry_run } => run_remove(&name, args.json, dry_run),
    }
}

fn run_add(
    name: &str,
    url: &str,
    kind: &str,
    repo: Option<String>,
    token_profile: Option<String>,
    json: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let config_path = root.join(".claw").join("remotes.toml");

    let mut config = load_remotes(&config_path);
    if config.remotes.contains_key(name) {
        anyhow::bail!(
            "remote '{}' already exists. Run `claw remote list` to inspect it.",
            name
        );
    }

    let entry = match kind {
        "grpc" => RemoteEntry {
            kind: Some("grpc".to_string()),
            url: Some(url.to_string()),
            token_profile,
            ..RemoteEntry::default()
        },
        "clawlab" => {
            let repo = repo.ok_or_else(|| {
                anyhow::anyhow!("--repo is required for clawlab remotes (example: acme/widgets)")
            })?;
            RemoteEntry {
                kind: Some("clawlab".to_string()),
                base_url: Some(url.to_string()),
                repo: Some(repo),
                token_profile,
                ..RemoteEntry::default()
            }
        }
        other => anyhow::bail!("unsupported remote kind: {other} (expected grpc|clawlab)"),
    };

    let resolved = normalize_entry(entry.clone())?;
    config.remotes.insert(name.to_string(), entry);
    if !dry_run {
        save_remotes(&config_path, &config)?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": "add",
                "remote": remote_json(name, &resolved),
                "dry_run": dry_run,
                "saved": !dry_run,
                "config_path": config_path.display().to_string(),
            }))?
        );
    } else if dry_run {
        println!("Would add remote '{}' ({kind}) -> {url}", name);
    } else {
        println!("Added remote '{}' ({kind}) -> {url}", name);
    }
    Ok(())
}

fn run_list(json: bool) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let config_path = root.join(".claw").join("remotes.toml");

    let config = load_remotes(&config_path);
    if json {
        let remotes = config
            .remotes
            .iter()
            .map(|(name, entry)| {
                normalize_entry(entry.clone()).map(|resolved| remote_json(name, &resolved))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "remotes": remotes,
                "config_path": config_path.display().to_string(),
            }))?
        );
        return Ok(());
    }

    if config.remotes.is_empty() {
        println!("No remotes configured.");
        return Ok(());
    }

    for (name, entry) in &config.remotes {
        match normalize_entry(entry.clone())? {
            ResolvedRemote::Grpc {
                addr,
                token_profile,
            } => {
                println!(
                    "{}\tgrpc\t{}\t{}",
                    name,
                    addr,
                    token_profile.unwrap_or_else(|| "-".to_string())
                );
            }
            ResolvedRemote::ClawLab {
                base_url,
                repo,
                token_profile,
            } => {
                println!(
                    "{}\tclawlab\t{}\t{}\t{}",
                    name,
                    base_url,
                    repo,
                    token_profile.unwrap_or_else(|| "default".to_string())
                );
            }
        }
    }
    Ok(())
}

fn run_remove(name: &str, json: bool, dry_run: bool) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let config_path = root.join(".claw").join("remotes.toml");

    let mut config = load_remotes(&config_path);
    let removed = config.remotes.remove(name).ok_or_else(|| {
        anyhow::anyhow!(
            "remote '{}' not found. Run `claw remote list` to inspect configured remotes.",
            name
        )
    })?;
    let resolved = normalize_entry(removed)?;
    if !dry_run {
        save_remotes(&config_path, &config)?;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "action": "remove",
                "remote": remote_json(name, &resolved),
                "dry_run": dry_run,
                "saved": !dry_run,
                "config_path": config_path.display().to_string(),
            }))?
        );
    } else if dry_run {
        println!("Would remove remote '{}'", name);
    } else {
        println!("Removed remote '{}'", name);
    }
    Ok(())
}

pub fn load_remotes(config_path: &Path) -> RemotesConfig {
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
    }
    RemotesConfig::default()
}

fn save_remotes(config_path: &Path, config: &RemotesConfig) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(config)?;
    std::fs::write(config_path, content)?;
    Ok(())
}

fn normalize_entry(entry: RemoteEntry) -> anyhow::Result<ResolvedRemote> {
    let kind = entry.kind.clone().unwrap_or_else(|| {
        if entry.base_url.is_some() || entry.repo.is_some() {
            "clawlab".to_string()
        } else {
            "grpc".to_string()
        }
    });

    match kind.as_str() {
        "grpc" => {
            let addr = entry
                .url
                .or(entry.base_url)
                .ok_or_else(|| anyhow::anyhow!("missing grpc url in remote entry"))?;
            Ok(ResolvedRemote::Grpc {
                addr,
                token_profile: entry.token_profile,
            })
        }
        "clawlab" => {
            let base_url = entry
                .base_url
                .or(entry.url)
                .ok_or_else(|| anyhow::anyhow!("missing base_url in clawlab remote entry"))?;
            let repo = entry
                .repo
                .ok_or_else(|| anyhow::anyhow!("missing repo in clawlab remote entry"))?;
            Ok(ResolvedRemote::ClawLab {
                base_url,
                repo,
                token_profile: entry.token_profile,
            })
        }
        other => anyhow::bail!("unsupported remote kind in config: {other}"),
    }
}

fn remote_json(name: &str, remote: &ResolvedRemote) -> serde_json::Value {
    match remote {
        ResolvedRemote::Grpc {
            addr,
            token_profile,
        } => serde_json::json!({
            "name": name,
            "kind": "grpc",
            "url": addr,
            "token_profile": token_profile,
        }),
        ResolvedRemote::ClawLab {
            base_url,
            repo,
            token_profile,
        } => serde_json::json!({
            "name": name,
            "kind": "clawlab",
            "base_url": base_url,
            "repo": repo,
            "token_profile": token_profile.as_deref().unwrap_or("default"),
        }),
    }
}

/// Resolve a remote argument to its transport-specific connection details.
pub fn resolve_remote(root: &Path, remote_arg: &str) -> anyhow::Result<ResolvedRemote> {
    if remote_arg.contains("://") || remote_arg.contains("localhost") {
        return Ok(ResolvedRemote::Grpc {
            addr: remote_arg.to_string(),
            token_profile: None,
        });
    }

    let config_path = root.join(".claw").join("remotes.toml");
    let config = load_remotes(&config_path);
    let entry = config.remotes.get(remote_arg).cloned().ok_or_else(|| {
        anyhow::anyhow!(
            "remote '{}' not found. Use a URL or `claw remote add`.",
            remote_arg
        )
    })?;

    normalize_entry(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_legacy_grpc_entry() {
        let entry = RemoteEntry {
            url: Some("http://localhost:50051".to_string()),
            ..RemoteEntry::default()
        };

        match normalize_entry(entry).unwrap() {
            ResolvedRemote::Grpc {
                addr,
                token_profile,
            } => {
                assert_eq!(addr, "http://localhost:50051");
                assert!(token_profile.is_none());
            }
            _ => panic!("expected grpc"),
        }
    }

    #[test]
    fn normalize_clawlab_entry() {
        let entry = RemoteEntry {
            kind: Some("clawlab".to_string()),
            base_url: Some("https://api.clawlab.com".to_string()),
            repo: Some("acme/widgets".to_string()),
            token_profile: Some("default".to_string()),
            ..RemoteEntry::default()
        };

        match normalize_entry(entry).unwrap() {
            ResolvedRemote::ClawLab {
                base_url,
                repo,
                token_profile,
            } => {
                assert_eq!(base_url, "https://api.clawlab.com");
                assert_eq!(repo, "acme/widgets");
                assert_eq!(token_profile.as_deref(), Some("default"));
            }
            _ => panic!("expected clawlab"),
        }
    }
}

use clap::{Args, Subcommand};

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_store::{ClawStore, HeadState};
use claw_sync::client::{RetryPolicy, SyncClient};
use claw_sync::compat::{compatibility_report, CompatibilityLevel};
use claw_sync::negotiation::ordered_reachable_objects;
use claw_sync::proto::sync::HelloResponse;
use claw_sync::transport::RemoteTransportConfig;

use crate::auth_store;
use crate::config::{self, find_repo_root};
use crate::worktree;

use super::{remote, RuntimeOptions};

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

fn resolve_token_profiles(
    token_profile: Option<&str>,
    runtime_profile: &str,
    repo_default_profile: &str,
) -> Vec<String> {
    let explicit_profile = token_profile.map(str::trim).filter(|value| !value.is_empty());
    if let Some(profile) = explicit_profile {
        return vec![profile.to_string()];
    }

    let mut profiles = Vec::new();
    let runtime_profile = runtime_profile.trim();
    if !runtime_profile.is_empty() {
        profiles.push(runtime_profile.to_string());
    }

    let repo_default_profile = repo_default_profile.trim();
    if !repo_default_profile.is_empty() && !profiles.iter().any(|p| p == repo_default_profile) {
        profiles.push(repo_default_profile.to_string());
    }

    if profiles.is_empty() {
        profiles.push("default".to_string());
    }

    profiles
}

fn require_access_token(
    token_profile: Option<&str>,
    runtime_profile: &str,
    repo_default_profile: &str,
) -> anyhow::Result<String> {
    let candidates = resolve_token_profiles(token_profile, runtime_profile, repo_default_profile);
    for profile_name in &candidates {
        if let Some(token) = auth_store::resolve_access_token(Some(profile_name)) {
            return Ok(token);
        }
    }

    let suggested_profile = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| "default".to_string());
    anyhow::bail!(
        "no token found for profiles [{}]; run `claw auth login --profile {}`",
        candidates.join(", "),
        suggested_profile
    );
}

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    command: Option<SyncCommand>,
    /// Remote name or address (compatibility form: claw sync <remote>)
    remote: Option<String>,
}

#[derive(Subcommand)]
enum SyncCommand {
    /// Push objects to remote
    Push {
        /// Remote name or address (e.g., origin or http://localhost:50051)
        #[arg(short, long, default_value = "origin")]
        remote: String,
        /// Ref to push
        #[arg(short = 'b', long, default_value = "heads/main")]
        ref_name: String,
        /// Force non-fast-forward push
        #[arg(long)]
        force: bool,
    },
    /// Pull objects from remote
    Pull {
        /// Remote name or address
        #[arg(short, long, default_value = "origin")]
        remote: String,
        /// Ref to pull
        #[arg(short = 'b', long, default_value = "heads/main")]
        ref_name: String,
        /// Force non-fast-forward update
        #[arg(long)]
        force: bool,
    },
    /// Clone a remote repository
    Clone {
        /// Remote address
        remote: String,
        /// Transport kind (grpc|clawlab)
        #[arg(long, default_value = "grpc")]
        kind: String,
        /// Repository slug for clawlab remotes
        #[arg(long)]
        repo: Option<String>,
        /// Auth profile for clawlab remotes
        #[arg(long)]
        token_profile: Option<String>,
        /// Local path
        #[arg(default_value = ".")]
        path: String,
    },
}

fn resolve_command(args: SyncArgs) -> SyncCommand {
    match args.command {
        Some(command) => command,
        None => SyncCommand::Pull {
            remote: args.remote.unwrap_or_else(|| "origin".to_string()),
            ref_name: "heads/main".to_string(),
            force: false,
        },
    }
}

async fn connect_from_remote(
    root: &std::path::Path,
    remote_arg: &str,
    runtime: &RuntimeOptions,
) -> anyhow::Result<SyncClient> {
    let resolved = remote::resolve_remote(root, remote_arg)?;
    let cfg = config::load_or_default_config(root)?;
    let repo_default_profile = config::default_profile(&cfg);
    let transport = match resolved {
        remote::ResolvedRemote::Grpc {
            addr,
            token_profile,
        } => {
            let bearer_token = token_profile
                .as_deref()
                .map(|profile| {
                    require_access_token(Some(profile), &runtime.profile, repo_default_profile)
                })
                .transpose()?;
            RemoteTransportConfig::Grpc { addr, bearer_token }
        }
        remote::ResolvedRemote::ClawLab {
            base_url,
            repo,
            token_profile,
        } => {
            let token =
                require_access_token(token_profile.as_deref(), &runtime.profile, repo_default_profile)?;
            RemoteTransportConfig::Http {
                base_url,
                repo,
                bearer_token: Some(token),
            }
        }
    };

    let retry_policy = RetryPolicy {
        idempotent_only: cfg.retries.idempotent_only,
        max_attempts: cfg.retries.max_attempts,
        base_backoff_ms: cfg.retries.base_backoff_ms,
        max_backoff_ms: cfg.retries.max_backoff_ms,
        jitter: cfg.retries.jitter,
    };

    let client = SyncClient::connect_with_transport_and_retry(transport, retry_policy).await?;
    Ok(client)
}

fn check_remote_compatibility(remote: &str, hello: &HelloResponse) -> anyhow::Result<()> {
    let report = compatibility_report(CLI_VERSION, &hello.server_version);
    match report.level {
        CompatibilityLevel::Full => Ok(()),
        CompatibilityLevel::Limited => {
            eprintln!(
                "compatibility check: limited support for {remote} (local {local}, remote {remote_ver}); N/N-1 compatibility applies, but prefer matching versions for best results",
                local = report.local,
                remote_ver = report.remote,
            );
            Ok(())
        }
        CompatibilityLevel::Unsupported => anyhow::bail!(
            "compatibility check failed for {remote}: local claw version '{local}' is incompatible with remote version '{remote_ver}'; use matching major version and at most one minor difference (N/N-1), or retry without --compat-check",
            local = report.local,
            remote_ver = report.remote,
        ),
    }
}

async fn maybe_check_compatibility(
    runtime: &RuntimeOptions,
    remote: &str,
    client: &mut SyncClient,
) -> anyhow::Result<()> {
    if runtime.compat_check {
        let hello = client.hello().await?;
        check_remote_compatibility(remote, &hello)?;
    }

    Ok(())
}

pub async fn run(args: SyncArgs, runtime: &RuntimeOptions) -> anyhow::Result<()> {
    match resolve_command(args) {
        SyncCommand::Push {
            remote,
            ref_name,
            force,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let mut client = connect_from_remote(&root, &remote, runtime).await?;
            maybe_check_compatibility(runtime, &remote, &mut client).await?;

            let local_id = store
                .get_ref(&ref_name)?
                .ok_or_else(|| anyhow::anyhow!("ref not found: {ref_name}"))?;

            let push_ids: Vec<ObjectId> = ordered_reachable_objects(&store, &[local_id]);

            let resp = client.push_objects(&store, &push_ids).await?;
            println!("Push: {}", resp.message);

            let remote_refs = client.advertise_refs("").await?;
            let remote_old = remote_refs
                .iter()
                .find(|(name, _)| name == &ref_name)
                .map(|(_, id)| *id);

            let updates = vec![(ref_name.clone(), remote_old, local_id)];
            let ref_resp = client.update_refs(&updates, force).await?;

            if ref_resp.success {
                println!("Pushed {} to {}", ref_name, remote);
            } else {
                anyhow::bail!("ref update failed: {}", ref_resp.message);
            }
        }
        SyncCommand::Pull {
            remote,
            ref_name,
            force,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let mut client = connect_from_remote(&root, &remote, runtime).await?;
            maybe_check_compatibility(runtime, &remote, &mut client).await?;

            let remote_refs = client.advertise_refs("").await?;
            let remote_target = remote_refs
                .iter()
                .find(|(name, _)| name == &ref_name)
                .map(|(_, id)| *id);

            let remote_id = match remote_target {
                Some(id) => id,
                None => {
                    println!("Remote ref {ref_name} not found");
                    return Ok(());
                }
            };

            let local_id = store.get_ref(&ref_name)?;
            let have: Vec<ObjectId> = local_id.into_iter().collect();

            let fetched = client.fetch_objects(&store, &[remote_id], &have).await?;
            println!("Fetched {} objects", fetched.len());

            if let Some(local) = store.get_ref(&ref_name)? {
                let is_ff = claw_sync::ancestry::is_ancestor(&store, &local, &remote_id);
                if !is_ff && !force {
                    anyhow::bail!(
                        "non-fast-forward update on {}; use --force to override",
                        ref_name
                    );
                }
            }

            let old = store.get_ref(&ref_name)?;
            store.update_ref_cas(&ref_name, old.as_ref(), &remote_id, "sync", "pull")?;
            println!("Updated {} to {}", ref_name, remote_id);

            let head_state = store.read_head()?;
            if let HeadState::Symbolic {
                ref_name: ref head_ref,
            } = head_state
            {
                if *head_ref == ref_name {
                    let rev_obj = store.load_object(&remote_id)?;
                    if let Object::Revision(ref rev) = rev_obj {
                        if let Some(ref tree_id) = rev.tree {
                            worktree::materialize_tree(&store, tree_id, &root)?;
                            println!("Working tree updated.");
                        }
                    }
                }
            }
        }
        SyncCommand::Clone {
            remote,
            kind,
            repo,
            token_profile,
            path,
        } => {
            let root = std::path::Path::new(&path);
            let store = ClawStore::init(root)?;
            let cfg = config::load_or_default_config(root)?;
            let repo_default_profile = config::default_profile(&cfg);
            let retry_policy = RetryPolicy {
                idempotent_only: cfg.retries.idempotent_only,
                max_attempts: cfg.retries.max_attempts,
                base_backoff_ms: cfg.retries.base_backoff_ms,
                max_backoff_ms: cfg.retries.max_backoff_ms,
                jitter: cfg.retries.jitter,
            };
            let mut client = match kind.as_str() {
                "grpc" => {
                    let bearer_token = token_profile
                        .as_deref()
                        .map(|profile| {
                            require_access_token(Some(profile), &runtime.profile, repo_default_profile)
                        })
                        .transpose()?;
                    SyncClient::connect_with_transport_and_retry(
                        RemoteTransportConfig::Grpc {
                            addr: remote.clone(),
                            bearer_token,
                        },
                        retry_policy,
                    )
                    .await?
                }
                "clawlab" => {
                    let repo_slug = repo.clone().ok_or_else(|| {
                        anyhow::anyhow!(
                            "--repo is required for --kind clawlab (example: acme/widgets)"
                        )
                    })?;
                    let token = require_access_token(
                        token_profile.as_deref(),
                        &runtime.profile,
                        repo_default_profile,
                    )?;
                    SyncClient::connect_with_transport_and_retry(
                        RemoteTransportConfig::Http {
                            base_url: remote.clone(),
                            repo: repo_slug,
                            bearer_token: Some(token),
                        },
                        retry_policy,
                    )
                    .await?
                }
                other => anyhow::bail!("unsupported --kind: {other} (expected grpc|clawlab)"),
            };

            let hello = client.hello().await?;
            if runtime.compat_check {
                check_remote_compatibility(&remote, &hello)?;
            }
            let remote_refs = client.advertise_refs("").await?;

            let want: Vec<_> = remote_refs.iter().map(|(_, id)| *id).collect();
            let fetched = client.fetch_objects(&store, &want, &[]).await?;

            for (name, id) in &remote_refs {
                store.set_ref(name, id)?;
            }

            store.write_head(&HeadState::Symbolic {
                ref_name: "heads/main".to_string(),
            })?;

            let main_id = store.get_ref("heads/main")?;
            let checkout_id = main_id.or_else(|| remote_refs.first().map(|(_, id)| *id));
            if let Some(rev_id) = checkout_id {
                let rev_obj = store.load_object(&rev_id)?;
                if let Object::Revision(ref rev) = rev_obj {
                    if let Some(ref tree_id) = rev.tree {
                        worktree::materialize_tree(&store, tree_id, root)?;
                    }
                }
            }

            let config_path = root.join(".claw").join("remotes.toml");
            let mut remotes = remote::load_remotes(&config_path);
            let origin_entry = match kind.as_str() {
                "grpc" => remote::RemoteEntry {
                    kind: Some("grpc".to_string()),
                    url: Some(remote.clone()),
                    token_profile: token_profile.clone(),
                    ..remote::RemoteEntry::default()
                },
                "clawlab" => remote::RemoteEntry {
                    kind: Some("clawlab".to_string()),
                    base_url: Some(remote.clone()),
                    repo: repo.clone(),
                    token_profile: token_profile.clone(),
                    ..remote::RemoteEntry::default()
                },
                _ => remote::RemoteEntry::default(),
            };
            remotes.remotes.insert("origin".to_string(), origin_entry);
            let content = toml::to_string_pretty(&remotes)?;
            std::fs::write(&config_path, content)?;

            println!(
                "Cloned {} ({} objects, {} refs)",
                remote,
                fetched.len(),
                remote_refs.len()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{
        check_remote_compatibility, resolve_command, resolve_token_profiles, SyncArgs,
        SyncCommand, CLI_VERSION,
    };
    use claw_sync::proto::sync::HelloResponse;

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        args: SyncArgs,
    }

    #[test]
    fn parse_compat_remote_form() {
        let cli = TestCli::parse_from(["claw", "origin"]);

        match resolve_command(cli.args) {
            SyncCommand::Pull {
                remote,
                ref_name,
                force,
            } => {
                assert_eq!(remote, "origin");
                assert_eq!(ref_name, "heads/main");
                assert!(!force);
            }
            _ => panic!("expected pull command"),
        }
    }

    #[test]
    fn parse_pull_subcommand_form() {
        let cli = TestCli::parse_from(["claw", "pull", "--remote", "upstream"]);

        match resolve_command(cli.args) {
            SyncCommand::Pull {
                remote,
                ref_name,
                force,
            } => {
                assert_eq!(remote, "upstream");
                assert_eq!(ref_name, "heads/main");
                assert!(!force);
            }
            _ => panic!("expected pull command"),
        }
    }

    #[test]
    fn parse_sync_without_remote_defaults_to_origin() {
        let cli = TestCli::parse_from(["claw"]);

        match resolve_command(cli.args) {
            SyncCommand::Pull {
                remote,
                ref_name,
                force,
            } => {
                assert_eq!(remote, "origin");
                assert_eq!(ref_name, "heads/main");
                assert!(!force);
            }
            _ => panic!("expected pull command"),
        }
    }

    #[test]
    fn compat_check_accepts_matching_version() {
        let hello = HelloResponse {
            server_version: CLI_VERSION.to_string(),
            capabilities: vec!["partial-clone".to_string()],
        };

        check_remote_compatibility("origin", &hello).expect("expected compatible versions");
    }

    #[test]
    fn compat_check_accepts_n_minus_one_minor_version() {
        let mut parts = CLI_VERSION.split('.');
        let major: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let minor: u64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
        let n_minus_one = minor.saturating_sub(1);
        let hello = HelloResponse {
            server_version: format!("{major}.{n_minus_one}.99"),
            capabilities: vec!["partial-clone".to_string()],
        };

        check_remote_compatibility("origin", &hello)
            .expect("N/N-1 compatibility should be accepted");
    }

    #[test]
    fn compat_check_rejects_unsupported_version_gap() {
        let hello = HelloResponse {
            server_version: "9.9.9".to_string(),
            capabilities: vec!["partial-clone".to_string()],
        };

        let err = check_remote_compatibility("origin", &hello).expect_err("expected mismatch");
        let message = err.to_string();
        assert!(message.contains("compatibility check failed"));
        assert!(message.contains("incompatible"));
        assert!(message.contains("N/N-1"));
        assert!(message.contains("--compat-check"));
    }

    #[test]
    fn resolve_token_profiles_prefers_explicit_profile() {
        let profiles = resolve_token_profiles(Some("team-ci"), "prod", "default");

        assert_eq!(profiles, vec!["team-ci"]);
    }

    #[test]
    fn resolve_token_profiles_uses_runtime_then_repo_default_when_omitted() {
        let profiles = resolve_token_profiles(None, "prod", "default");

        assert_eq!(profiles, vec!["prod", "default"]);
    }

    #[test]
    fn resolve_token_profiles_deduplicates_runtime_and_repo_default() {
        let profiles = resolve_token_profiles(None, "default", "default");

        assert_eq!(profiles, vec!["default"]);
    }
}

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use claw_core::object::Object;
use claw_core::types::CapsulePublic;
use claw_crypto::keypair::KeyPair;
use claw_store::ClawStore;

use crate::config::find_repo_root;

#[derive(Args)]
pub struct AgentArgs {
    /// Output command results as JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Register a new agent
    Register {
        /// Agent ID
        #[arg(short, long)]
        name: String,
        /// Agent version
        #[arg(short, long)]
        version: Option<String>,
        /// Register an externally managed Ed25519 public key instead of generating a local signing key
        #[arg(long)]
        public_key: Option<String>,
    },
    /// Generate a local agent signing key and print its public key
    Keygen {
        /// Agent ID used to choose the local key path
        #[arg(short, long)]
        name: String,
        /// Replace an existing local key for this agent name
        #[arg(long)]
        overwrite: bool,
    },
    /// Rotate an agent signing key and trust the replacement key
    Rotate {
        /// Agent ID
        #[arg(short, long)]
        name: String,
        /// Replacement agent version metadata
        #[arg(short, long)]
        version: Option<String>,
        /// Trust an externally managed replacement public key instead of generating a local signing key
        #[arg(long)]
        public_key: Option<String>,
        /// Validate and print the planned rotation without writing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Revoke an agent registration for future policy decisions
    Revoke {
        /// Agent ID
        #[arg(short, long)]
        name: String,
        /// Human-readable revocation reason
        #[arg(long)]
        reason: Option<String>,
        /// Validate and print the planned revocation without writing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Show agent status
    Status {
        /// Agent name
        name: Option<String>,
    },
    /// List registered agents
    List,
}

const AGENT_SCHEMA_VERSION: u8 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentRegistration {
    #[serde(default = "agent_schema_version")]
    pub schema_version: u8,
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
    pub public_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation_reason: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl AgentRegistration {
    pub(crate) fn is_revoked(&self) -> bool {
        self.revoked_at_ms.is_some()
    }

    fn public_key_prefix(&self) -> &str {
        let end = self.public_key.len().min(16);
        &self.public_key[..end]
    }
}

fn public_key_prefix(public_key: &str) -> &str {
    let end = public_key.len().min(16);
    &public_key[..end]
}

enum AgentRecordState {
    Missing,
    Registered(AgentRegistration),
    Legacy(CapsulePublic),
}

fn agent_schema_version() -> u8 {
    AGENT_SCHEMA_VERSION
}

fn now_ms() -> anyhow::Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64)
}

fn claw_home_dir() -> anyhow::Result<std::path::PathBuf> {
    dirs::home_dir()
        .map(|dir| dir.join(".claw"))
        .ok_or_else(|| anyhow::anyhow!("could not find home directory"))
}

fn agent_keys_dir() -> anyhow::Result<std::path::PathBuf> {
    Ok(claw_home_dir()?.join("agent-keys"))
}

fn agent_key_path(name: &str) -> anyhow::Result<std::path::PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let digest = hex::encode(hasher.finalize());
    Ok(agent_keys_dir()?.join(format!("{digest}.ed25519")))
}

fn set_private_permissions(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    let _ = path;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn save_local_agent_key(name: &str, keypair: &KeyPair) -> anyhow::Result<()> {
    let path = agent_key_path(name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    keypair.save_to_file(&path)?;
    set_private_permissions(&path)?;
    Ok(())
}

fn load_local_agent_key(name: &str) -> anyhow::Result<Option<KeyPair>> {
    let path = agent_key_path(name)?;
    if !path.exists() {
        return Ok(None);
    }
    let keypair =
        KeyPair::load_from_file(&path).map_err(|e| anyhow::anyhow!("invalid local key: {e}"))?;
    set_private_permissions(&path)?;
    Ok(Some(keypair))
}

fn decode_hex_32(value: &str, field: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = hex::decode(value).map_err(|e| anyhow::anyhow!("invalid {field}: {e}"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid {field}: expected 32 bytes"))
}

fn normalize_public_key(public_key: &str) -> anyhow::Result<String> {
    let bytes = decode_hex_32(public_key, "public key")?;
    Ok(hex::encode(bytes))
}

fn keypair_from_private(private_hex: &str, expected_public: &[u8; 32]) -> anyhow::Result<KeyPair> {
    let private = decode_hex_32(private_hex, "private key")?;
    let keypair = KeyPair::from_bytes(&private)?;
    if keypair.public_key_bytes() != *expected_public {
        anyhow::bail!("agent key mismatch: stored public key does not match private key");
    }
    Ok(keypair)
}

fn ensure_local_key_for_registration(
    record: &AgentRegistration,
    name: &str,
) -> anyhow::Result<bool> {
    let expected_public = decode_hex_32(&record.public_key, "public key")?;

    if let Some(local_key) = load_local_agent_key(name)? {
        if local_key.public_key_bytes() == expected_public {
            return Ok(true);
        }

        if let Some(private_hex) = record.private_key.as_deref() {
            let recovered = keypair_from_private(private_hex, &expected_public)?;
            save_local_agent_key(name, &recovered)?;
            return Ok(true);
        }

        anyhow::bail!("agent key mismatch: local key does not match stored public key");
    }

    if let Some(private_hex) = record.private_key.as_deref() {
        let recovered = keypair_from_private(private_hex, &expected_public)?;
        save_local_agent_key(name, &recovered)?;
        return Ok(true);
    }

    Ok(false)
}

fn registration_keypair(record: &AgentRegistration, name: &str) -> anyhow::Result<KeyPair> {
    let expected_public = decode_hex_32(&record.public_key, "public key")?;
    let Some(local_key) = load_local_agent_key(name)? else {
        anyhow::bail!("local signing key not found for agent '{name}'");
    };
    if local_key.public_key_bytes() != expected_public {
        anyhow::bail!("agent key mismatch: local key does not match stored public key");
    }
    Ok(local_key)
}

fn read_agent_record(store: &ClawStore, name: &str) -> anyhow::Result<AgentRecordState> {
    let Some(id) = store.get_ref(&format!("agents/{name}"))? else {
        return Ok(AgentRecordState::Missing);
    };
    let obj = store.load_object(&id)?;
    let Object::Blob(blob) = obj else {
        anyhow::bail!("agent ref points to non-blob object");
    };

    if let Ok(record) = serde_json::from_slice::<AgentRegistration>(&blob.data) {
        return Ok(AgentRecordState::Registered(record));
    }
    if let Ok(legacy) = serde_json::from_slice::<CapsulePublic>(&blob.data) {
        return Ok(AgentRecordState::Legacy(legacy));
    }

    anyhow::bail!("agent record format is not recognized")
}

fn store_agent_registration(
    store: &ClawStore,
    name: &str,
    record: &AgentRegistration,
) -> anyhow::Result<claw_core::id::ObjectId> {
    let mut sanitized = record.clone();
    sanitized.private_key = None;
    let serialized =
        serde_json::to_vec(&sanitized).map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
    let blob = Object::Blob(claw_core::types::Blob {
        data: serialized,
        media_type: Some("application/json".to_string()),
    });
    let id = store.store_object(&blob)?;
    store.set_ref(&format!("agents/{name}"), &id)?;
    Ok(id)
}

fn new_registration(
    name: &str,
    version: Option<String>,
) -> anyhow::Result<(AgentRegistration, KeyPair)> {
    let now = now_ms()?;
    let keypair = KeyPair::generate();
    Ok((
        AgentRegistration {
            schema_version: AGENT_SCHEMA_VERSION,
            agent_id: name.to_string(),
            agent_version: version,
            public_key: hex::encode(keypair.public_key_bytes()),
            private_key: None,
            revoked_at_ms: None,
            revocation_reason: None,
            created_at_ms: now,
            updated_at_ms: now,
        },
        keypair,
    ))
}

fn new_external_registration(
    name: &str,
    version: Option<String>,
    public_key: String,
) -> anyhow::Result<AgentRegistration> {
    let now = now_ms()?;
    Ok(AgentRegistration {
        schema_version: AGENT_SCHEMA_VERSION,
        agent_id: name.to_string(),
        agent_version: version,
        public_key: normalize_public_key(&public_key)?,
        private_key: None,
        revoked_at_ms: None,
        revocation_reason: None,
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub(crate) fn ensure_registered_signing_agent(
    store: &ClawStore,
    name: &str,
) -> anyhow::Result<AgentRegistration> {
    match read_agent_record(store, name)? {
        AgentRecordState::Registered(mut record) => {
            if record.is_revoked() {
                anyhow::bail!(
                    "agent '{name}' is revoked; run `claw agent rotate --name {name}` to trust a replacement key"
                );
            }
            if !ensure_local_key_for_registration(&record, name)? {
                // Existing metadata without local key: rotate to a new local-only private key.
                let (mut rotated, keypair) = new_registration(name, record.agent_version.clone())?;
                rotated.created_at_ms = record.created_at_ms;
                rotated.updated_at_ms = now_ms()?;
                save_local_agent_key(name, &keypair)?;
                store_agent_registration(store, name, &rotated)?;
                return Ok(rotated);
            }

            if record.private_key.is_some() {
                record.private_key = None;
                record.updated_at_ms = now_ms()?;
                store_agent_registration(store, name, &record)?;
            }
            Ok(record)
        }
        AgentRecordState::Legacy(legacy) => {
            let (record, keypair) = new_registration(name, legacy.agent_version)?;
            save_local_agent_key(name, &keypair)?;
            store_agent_registration(store, name, &record)?;
            Ok(record)
        }
        AgentRecordState::Missing => {
            let (record, keypair) = new_registration(name, None)?;
            save_local_agent_key(name, &keypair)?;
            store_agent_registration(store, name, &record)?;
            Ok(record)
        }
    }
}

pub(crate) fn keypair_for_agent(
    agent_name: &str,
    record: &AgentRegistration,
) -> anyhow::Result<KeyPair> {
    registration_keypair(record, agent_name)
}

pub fn run(args: AgentArgs) -> anyhow::Result<()> {
    let json = args.json;
    match args.command {
        AgentCommand::Register {
            name,
            version,
            public_key,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let requested_version = version.clone();

            let (record, id, created) = match read_agent_record(&store, &name)? {
                AgentRecordState::Registered(mut existing) => {
                    if existing.is_revoked() {
                        anyhow::bail!(
                            "agent '{name}' is revoked; run `claw agent rotate --name {name}` to trust a replacement key"
                        );
                    }
                    if let Some(v) = requested_version.clone() {
                        existing.agent_version = Some(v);
                    }
                    existing.private_key = None;

                    if let Some(public_key) = public_key.clone() {
                        existing.public_key = normalize_public_key(&public_key)?;
                    } else {
                        match ensure_local_key_for_registration(&existing, &name) {
                            Ok(true) => {}
                            Ok(false) | Err(_) => {
                                let keypair = KeyPair::generate();
                                existing.public_key = hex::encode(keypair.public_key_bytes());
                                save_local_agent_key(&name, &keypair)?;
                            }
                        }
                    }
                    existing.updated_at_ms = now_ms()?;
                    let id = store_agent_registration(&store, &name, &existing)?;
                    (existing, id, false)
                }
                AgentRecordState::Legacy(legacy) => {
                    let version = requested_version.clone().or(legacy.agent_version);
                    let record = if let Some(public_key) = public_key.clone() {
                        new_external_registration(&name, version, public_key)?
                    } else {
                        let (record, keypair) = new_registration(&name, version)?;
                        save_local_agent_key(&name, &keypair)?;
                        record
                    };
                    let id = store_agent_registration(&store, &name, &record)?;
                    (record, id, true)
                }
                AgentRecordState::Missing => {
                    let record = if let Some(public_key) = public_key.clone() {
                        new_external_registration(&name, requested_version.clone(), public_key)?
                    } else {
                        let (record, keypair) = new_registration(&name, requested_version.clone())?;
                        save_local_agent_key(&name, &keypair)?;
                        record
                    };
                    let id = store_agent_registration(&store, &name, &record)?;
                    (record, id, true)
                }
            };

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": if created { "registered" } else { "updated" },
                        "agent_id": record.agent_id,
                        "agent_version": record.agent_version,
                        "public_key": record.public_key,
                        "object_id": id.to_string(),
                        "status": if record.is_revoked() { "revoked" } else { "active" },
                    }))?
                );
            } else {
                if created {
                    println!("Registered agent: {name}");
                } else {
                    println!("Updated agent: {name}");
                }
                if let Some(v) = record.agent_version.as_deref() {
                    println!("  Version: {v}");
                }
                println!("  Public key: {}", record.public_key_prefix());
                println!("  Object: {id}");
            }
        }
        AgentCommand::Keygen { name, overwrite } => {
            let path = agent_key_path(&name)?;
            if path.exists() && !overwrite {
                anyhow::bail!(
                    "local signing key already exists for agent '{name}'; use --overwrite to replace it"
                );
            }
            let keypair = KeyPair::generate();
            save_local_agent_key(&name, &keypair)?;
            let public_key = hex::encode(keypair.public_key_bytes());
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "keygen",
                        "agent_id": name,
                        "public_key": public_key,
                        "key_path": path.display().to_string(),
                    }))?
                );
            } else {
                println!("Generated local agent key: {name}");
                println!("  Public key: {public_key}");
                println!("  Key path: {}", path.display());
            }
        }
        AgentCommand::Rotate {
            name,
            version,
            public_key,
            dry_run,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let current = match read_agent_record(&store, &name)? {
                AgentRecordState::Registered(record) => record,
                AgentRecordState::Legacy(_) => {
                    anyhow::bail!(
                        "agent '{name}' uses a legacy registration; run `claw agent register --name {name}` before rotating"
                    )
                }
                AgentRecordState::Missing => {
                    anyhow::bail!("agent '{name}' is not registered; run `claw agent register --name {name}` first")
                }
            };

            let replacement_version = version.clone().or(current.agent_version.clone());
            let normalized_public_key = public_key
                .as_deref()
                .map(normalize_public_key)
                .transpose()?;
            if dry_run {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "action": "rotate",
                            "dry_run": true,
                            "agent_id": name,
                            "current_public_key": current.public_key,
                            "replacement_public_key": normalized_public_key,
                            "replacement_version": replacement_version,
                            "current_status": if current.is_revoked() { "revoked" } else { "active" },
                        }))?
                    );
                } else {
                    println!("Dry run: would rotate agent: {name}");
                    println!("  Current public key: {}", current.public_key_prefix());
                    if let Some(public_key) = normalized_public_key.as_deref() {
                        println!(
                            "  Replacement public key: {}",
                            public_key_prefix(public_key)
                        );
                    }
                    if current.is_revoked() {
                        println!("  Current status: revoked");
                    }
                    if let Some(v) = replacement_version {
                        println!("  Replacement version: {v}");
                    }
                    println!("  Repository and local key store were not modified.");
                }
                return Ok(());
            }

            let mut rotated = if let Some(public_key) = normalized_public_key {
                new_external_registration(&name, replacement_version, public_key)?
            } else {
                let (record, keypair) = new_registration(&name, replacement_version)?;
                save_local_agent_key(&name, &keypair)?;
                record
            };
            rotated.created_at_ms = current.created_at_ms;
            rotated.updated_at_ms = now_ms()?;
            rotated.revoked_at_ms = None;
            rotated.revocation_reason = None;
            let id = store_agent_registration(&store, &name, &rotated)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "rotated",
                        "agent_id": rotated.agent_id,
                        "agent_version": rotated.agent_version,
                        "previous_public_key": current.public_key,
                        "public_key": rotated.public_key,
                        "object_id": id.to_string(),
                        "status": "active",
                    }))?
                );
            } else {
                println!("Rotated agent: {name}");
                println!("  Previous public key: {}", current.public_key_prefix());
                println!("  New public key: {}", rotated.public_key_prefix());
                if let Some(v) = rotated.agent_version.as_deref() {
                    println!("  Version: {v}");
                }
                println!("  Object: {id}");
            }
        }
        AgentCommand::Revoke {
            name,
            reason,
            dry_run,
        } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let mut record = match read_agent_record(&store, &name)? {
                AgentRecordState::Registered(record) => record,
                AgentRecordState::Legacy(_) => {
                    anyhow::bail!(
                        "agent '{name}' uses a legacy registration; re-register before revoking"
                    )
                }
                AgentRecordState::Missing => anyhow::bail!("agent '{name}' is not registered"),
            };

            if dry_run {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "action": "revoke",
                            "dry_run": true,
                            "agent_id": name,
                            "public_key": record.public_key,
                            "reason": reason,
                        }))?
                    );
                } else {
                    println!("Dry run: would revoke agent: {name}");
                    println!("  Public key: {}", record.public_key_prefix());
                    if let Some(reason) = reason.as_deref() {
                        println!("  Reason: {reason}");
                    }
                    println!("  Repository was not modified.");
                }
                return Ok(());
            }

            if record.is_revoked() {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "action": "already_revoked",
                            "agent_id": record.agent_id,
                            "public_key": record.public_key,
                            "revoked_at_ms": record.revoked_at_ms,
                            "reason": record.revocation_reason,
                        }))?
                    );
                } else {
                    println!("Agent already revoked: {name}");
                    if let Some(reason) = record.revocation_reason.as_deref() {
                        println!("  Reason: {reason}");
                    }
                }
                return Ok(());
            }

            record.revoked_at_ms = Some(now_ms()?);
            record.revocation_reason = reason;
            record.updated_at_ms = now_ms()?;
            let id = store_agent_registration(&store, &name, &record)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "revoked",
                        "agent_id": record.agent_id,
                        "public_key": record.public_key,
                        "revoked_at_ms": record.revoked_at_ms,
                        "reason": record.revocation_reason,
                        "object_id": id.to_string(),
                    }))?
                );
            } else {
                println!("Revoked agent: {name}");
                println!("  Public key: {}", record.public_key_prefix());
                if let Some(reason) = record.revocation_reason.as_deref() {
                    println!("  Reason: {reason}");
                }
                println!("  Object: {id}");
            }
        }
        AgentCommand::Status { name } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            if let Some(n) = name {
                match read_agent_record(&store, &n)? {
                    AgentRecordState::Registered(agent) => {
                        let key_ok = registration_keypair(&agent, &n).is_ok();
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "found": true,
                                    "kind": "registered",
                                    "agent_id": agent.agent_id,
                                    "agent_version": agent.agent_version,
                                    "public_key": agent.public_key,
                                    "key_status": if key_ok { "verified" } else { "invalid" },
                                    "status": if agent.is_revoked() { "revoked" } else { "active" },
                                    "revoked_at_ms": agent.revoked_at_ms,
                                    "revocation_reason": agent.revocation_reason,
                                }))?
                            );
                        } else {
                            println!("Agent: {}", agent.agent_id);
                            if let Some(v) = &agent.agent_version {
                                println!("  Version: {v}");
                            }
                            println!(
                                "  Key: {} ({})",
                                agent.public_key_prefix(),
                                if key_ok { "verified" } else { "invalid" }
                            );
                            if agent.is_revoked() {
                                println!("  Status: revoked");
                                if let Some(reason) = agent.revocation_reason.as_deref() {
                                    println!("  Revocation reason: {reason}");
                                }
                            } else {
                                println!("  Status: active");
                            }
                        }
                    }
                    AgentRecordState::Legacy(agent) => {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "found": true,
                                    "kind": "legacy",
                                    "agent_id": agent.agent_id,
                                    "agent_version": agent.agent_version,
                                    "key_status": "missing",
                                    "status": "legacy",
                                }))?
                            );
                        } else {
                            println!("Agent: {}", agent.agent_id);
                            if let Some(v) = &agent.agent_version {
                                println!("  Version: {v}");
                            }
                            println!("  Key: missing (legacy registration)");
                            println!("  Status: legacy");
                        }
                    }
                    AgentRecordState::Missing => {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&serde_json::json!({
                                    "found": false,
                                    "agent_id": n,
                                }))?
                            );
                        } else {
                            println!("Agent {n}: not found");
                        }
                    }
                }
            } else if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "error": "missing_agent_name",
                        "remediation": "Use `claw agent list --json` to see all agents.",
                    }))?
                );
            } else {
                println!("Use 'claw agent list' to see all agents.");
            }
        }
        AgentCommand::List => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let refs = store.list_refs("agents")?;
            if json {
                let mut agents = Vec::new();
                for (name, id) in &refs {
                    let agent_name = name.trim_start_matches("agents/");
                    match read_agent_record(&store, agent_name) {
                        Ok(AgentRecordState::Registered(agent)) => {
                            let key_ok = registration_keypair(&agent, agent_name).is_ok();
                            agents.push(serde_json::json!({
                                "agent_id": agent.agent_id,
                                "agent_version": agent.agent_version,
                                "public_key": agent.public_key,
                                "key_status": if key_ok { "verified" } else { "invalid" },
                                "status": if agent.is_revoked() { "revoked" } else { "active" },
                                "object_id": id.to_string(),
                            }));
                        }
                        Ok(AgentRecordState::Legacy(agent)) => {
                            agents.push(serde_json::json!({
                                "agent_id": agent.agent_id,
                                "agent_version": agent.agent_version,
                                "key_status": "legacy",
                                "status": "legacy",
                                "object_id": id.to_string(),
                            }));
                        }
                        Ok(AgentRecordState::Missing) | Err(_) => {
                            agents.push(serde_json::json!({
                                "ref": name,
                                "object_id": id.to_string(),
                                "status": "unreadable",
                            }));
                        }
                    }
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({ "agents": agents }))?
                );
            } else if refs.is_empty() {
                println!("No agents registered.");
            } else {
                for (name, id) in &refs {
                    let agent_name = name.trim_start_matches("agents/");
                    match read_agent_record(&store, agent_name) {
                        Ok(AgentRecordState::Registered(agent)) => {
                            let key_ok = registration_keypair(&agent, agent_name).is_ok();
                            println!(
                                "{} v{} key:{} status:{}",
                                agent.agent_id,
                                agent.agent_version.as_deref().unwrap_or("?"),
                                if key_ok { "verified" } else { "invalid" },
                                if agent.is_revoked() {
                                    "revoked"
                                } else {
                                    "active"
                                }
                            );
                        }
                        Ok(AgentRecordState::Legacy(agent)) => {
                            println!(
                                "{} v{} key:legacy",
                                agent.agent_id,
                                agent.agent_version.as_deref().unwrap_or("?")
                            );
                        }
                        Ok(AgentRecordState::Missing) => {
                            println!("{name}");
                        }
                        Err(_) => {
                            if store.load_object(id).is_ok() {
                                println!("{name}");
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{AgentArgs, AgentCommand, AgentRegistration};

    #[derive(Parser)]
    struct TestCli {
        #[command(flatten)]
        args: AgentArgs,
    }

    #[test]
    fn parses_rotate_dry_run() {
        let cli = TestCli::parse_from([
            "claw",
            "rotate",
            "--name",
            "ci-agent",
            "--version",
            "2026-05-11",
            "--dry-run",
        ]);

        match cli.args.command {
            AgentCommand::Rotate {
                name,
                version,
                public_key,
                dry_run,
            } => {
                assert_eq!(name, "ci-agent");
                assert_eq!(version.as_deref(), Some("2026-05-11"));
                assert!(public_key.is_none());
                assert!(dry_run);
            }
            _ => panic!("expected rotate command"),
        }
    }

    #[test]
    fn parses_keygen_overwrite() {
        let cli = TestCli::parse_from(["claw", "keygen", "--name", "ci-agent", "--overwrite"]);

        match cli.args.command {
            AgentCommand::Keygen { name, overwrite } => {
                assert_eq!(name, "ci-agent");
                assert!(overwrite);
            }
            _ => panic!("expected keygen command"),
        }
    }

    #[test]
    fn parses_json_after_subcommand() {
        let cli = TestCli::parse_from(["claw", "list", "--json"]);

        assert!(cli.args.json);
        assert!(matches!(cli.args.command, AgentCommand::List));
    }

    #[test]
    fn parses_register_public_key() {
        let public_key = "aa".repeat(32);
        let cli = TestCli::parse_from([
            "claw",
            "register",
            "--name",
            "ci-agent",
            "--public-key",
            &public_key,
        ]);

        match cli.args.command {
            AgentCommand::Register {
                name,
                public_key: parsed_public_key,
                ..
            } => {
                assert_eq!(name, "ci-agent");
                assert_eq!(parsed_public_key.as_deref(), Some(public_key.as_str()));
            }
            _ => panic!("expected register command"),
        }
    }

    #[test]
    fn parses_revoke_reason_and_dry_run() {
        let cli = TestCli::parse_from([
            "claw",
            "revoke",
            "--name",
            "ci-agent",
            "--reason",
            "compromised",
            "--dry-run",
        ]);

        match cli.args.command {
            AgentCommand::Revoke {
                name,
                reason,
                dry_run,
            } => {
                assert_eq!(name, "ci-agent");
                assert_eq!(reason.as_deref(), Some("compromised"));
                assert!(dry_run);
            }
            _ => panic!("expected revoke command"),
        }
    }

    #[test]
    fn registration_revocation_defaults_to_active_for_legacy_json() {
        let json = r#"{
            "schema_version": 2,
            "agent_id": "ci-agent",
            "public_key": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "created_at_ms": 1,
            "updated_at_ms": 1
        }"#;

        let record: AgentRegistration = serde_json::from_str(json).unwrap();
        assert!(!record.is_revoked());
        assert!(record.revocation_reason.is_none());
    }
}

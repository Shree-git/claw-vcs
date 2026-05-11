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
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
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
            created_at_ms: now,
            updated_at_ms: now,
        },
        keypair,
    ))
}

pub(crate) fn ensure_registered_signing_agent(
    store: &ClawStore,
    name: &str,
) -> anyhow::Result<AgentRegistration> {
    match read_agent_record(store, name)? {
        AgentRecordState::Registered(mut record) => {
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
    match args.command {
        AgentCommand::Register { name, version } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let requested_version = version.clone();

            let (record, id, created) = match read_agent_record(&store, &name)? {
                AgentRecordState::Registered(mut existing) => {
                    if let Some(v) = requested_version.clone() {
                        existing.agent_version = Some(v);
                    }
                    existing.private_key = None;

                    match ensure_local_key_for_registration(&existing, &name) {
                        Ok(true) => {}
                        Ok(false) | Err(_) => {
                            let keypair = KeyPair::generate();
                            existing.public_key = hex::encode(keypair.public_key_bytes());
                            save_local_agent_key(&name, &keypair)?;
                        }
                    }
                    existing.updated_at_ms = now_ms()?;
                    let id = store_agent_registration(&store, &name, &existing)?;
                    (existing, id, false)
                }
                AgentRecordState::Legacy(legacy) => {
                    let (record, keypair) = new_registration(
                        &name,
                        requested_version.clone().or(legacy.agent_version),
                    )?;
                    save_local_agent_key(&name, &keypair)?;
                    let id = store_agent_registration(&store, &name, &record)?;
                    (record, id, true)
                }
                AgentRecordState::Missing => {
                    let (record, keypair) = new_registration(&name, requested_version.clone())?;
                    save_local_agent_key(&name, &keypair)?;
                    let id = store_agent_registration(&store, &name, &record)?;
                    (record, id, true)
                }
            };

            if created {
                println!("Registered agent: {name}");
            } else {
                println!("Updated agent: {name}");
            }
            if let Some(v) = record.agent_version {
                println!("  Version: {v}");
            }
            println!("  Public key: {}", &record.public_key[..16]);
            println!("  Object: {id}");
        }
        AgentCommand::Status { name } => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;

            if let Some(n) = name {
                match read_agent_record(&store, &n)? {
                    AgentRecordState::Registered(agent) => {
                        let key_ok = registration_keypair(&agent, &n).is_ok();
                        println!("Agent: {}", agent.agent_id);
                        if let Some(v) = &agent.agent_version {
                            println!("  Version: {v}");
                        }
                        println!(
                            "  Key: {} ({})",
                            &agent.public_key[..16],
                            if key_ok { "verified" } else { "invalid" }
                        );
                        println!("  Status: active");
                    }
                    AgentRecordState::Legacy(agent) => {
                        println!("Agent: {}", agent.agent_id);
                        if let Some(v) = &agent.agent_version {
                            println!("  Version: {v}");
                        }
                        println!("  Key: missing (legacy registration)");
                        println!("  Status: legacy");
                    }
                    AgentRecordState::Missing => {
                        println!("Agent {n}: not found");
                    }
                }
            } else {
                println!("Use 'claw agent list' to see all agents.");
            }
        }
        AgentCommand::List => {
            let root = find_repo_root()?;
            let store = ClawStore::open(&root)?;
            let refs = store.list_refs("agents")?;
            if refs.is_empty() {
                println!("No agents registered.");
            } else {
                for (name, id) in &refs {
                    let agent_name = name.trim_start_matches("agents/");
                    match read_agent_record(&store, agent_name) {
                        Ok(AgentRecordState::Registered(agent)) => {
                            let key_ok = registration_keypair(&agent, agent_name).is_ok();
                            println!(
                                "{} v{} key:{}",
                                agent.agent_id,
                                agent.agent_version.as_deref().unwrap_or("?"),
                                if key_ok { "verified" } else { "invalid" }
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

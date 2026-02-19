use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

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

const AGENT_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentRegistration {
    #[serde(default = "agent_schema_version")]
    pub schema_version: u8,
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
    pub public_key: String,
    pub private_key: String,
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

fn decode_hex_32(value: &str, field: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = hex::decode(value).map_err(|e| anyhow::anyhow!("invalid {field}: {e}"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid {field}: expected 32 bytes"))
}

fn registration_keypair(record: &AgentRegistration) -> anyhow::Result<KeyPair> {
    let private = decode_hex_32(&record.private_key, "private key")?;
    let public = decode_hex_32(&record.public_key, "public key")?;
    let keypair = KeyPair::from_bytes(&private)?;
    if keypair.public_key_bytes() != public {
        anyhow::bail!("agent key mismatch: stored public key does not match private key");
    }
    Ok(keypair)
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
    let serialized =
        serde_json::to_vec(record).map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
    let blob = Object::Blob(claw_core::types::Blob {
        data: serialized,
        media_type: Some("application/json".to_string()),
    });
    let id = store.store_object(&blob)?;
    store.set_ref(&format!("agents/{name}"), &id)?;
    Ok(id)
}

fn new_registration(name: &str, version: Option<String>) -> anyhow::Result<AgentRegistration> {
    let now = now_ms()?;
    let keypair = KeyPair::generate();
    Ok(AgentRegistration {
        schema_version: AGENT_SCHEMA_VERSION,
        agent_id: name.to_string(),
        agent_version: version,
        public_key: hex::encode(keypair.public_key_bytes()),
        private_key: hex::encode(keypair.to_bytes()),
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub(crate) fn ensure_registered_signing_agent(
    store: &ClawStore,
    name: &str,
) -> anyhow::Result<AgentRegistration> {
    match read_agent_record(store, name)? {
        AgentRecordState::Registered(record) => {
            registration_keypair(&record)?;
            Ok(record)
        }
        AgentRecordState::Legacy(legacy) => {
            let record = new_registration(name, legacy.agent_version)?;
            store_agent_registration(store, name, &record)?;
            Ok(record)
        }
        AgentRecordState::Missing => {
            let record = new_registration(name, None)?;
            store_agent_registration(store, name, &record)?;
            Ok(record)
        }
    }
}

pub(crate) fn keypair_for_agent(record: &AgentRegistration) -> anyhow::Result<KeyPair> {
    registration_keypair(record)
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
                    existing.updated_at_ms = now_ms()?;
                    let id = store_agent_registration(&store, &name, &existing)?;
                    (existing, id, false)
                }
                AgentRecordState::Legacy(legacy) => {
                    let record = new_registration(
                        &name,
                        requested_version.clone().or(legacy.agent_version),
                    )?;
                    let id = store_agent_registration(&store, &name, &record)?;
                    (record, id, true)
                }
                AgentRecordState::Missing => {
                    let record = new_registration(&name, requested_version.clone())?;
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
                        let key_ok = registration_keypair(&agent).is_ok();
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
                    match read_agent_record(&store, name.trim_start_matches("agents/")) {
                        Ok(AgentRecordState::Registered(agent)) => {
                            let key_ok = registration_keypair(&agent).is_ok();
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

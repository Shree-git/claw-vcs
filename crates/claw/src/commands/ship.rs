use clap::Args;

use claw_core::id::ChangeId;
use claw_core::object::Object;
use claw_core::types::{CapsulePublic, ChangeStatus, Evidence, IntentStatus};
use claw_crypto::capsule::build_capsule;
use claw_store::ClawStore;

use super::agent::{ensure_registered_signing_agent, keypair_for_agent};
use crate::config::find_repo_root;

#[derive(Args)]
pub struct ShipArgs {
    /// Intent ID to ship
    #[arg(short, long)]
    intent: String,
    /// Revision ref to ship
    #[arg(short, long, default_value = "heads/main")]
    revision_ref: String,
    /// Agent ID
    #[arg(short, long, default_value = "claw")]
    agent: String,
    /// Capsule evidence item in the form name=status[:duration_ms]
    #[arg(long = "evidence")]
    evidence: Vec<String>,
}

fn parse_evidence(items: &[String]) -> anyhow::Result<Vec<Evidence>> {
    let mut out = Vec::with_capacity(items.len());

    for raw in items {
        let (name, status_and_duration) = if let Some(parts) = raw.split_once('=') {
            parts
        } else if let Some(parts) = raw.split_once(':') {
            parts
        } else {
            anyhow::bail!(
                "invalid --evidence '{}'; expected name=status[:duration_ms]",
                raw
            );
        };

        if name.trim().is_empty() {
            anyhow::bail!("invalid --evidence '{}'; name cannot be empty", raw);
        }

        let (status, duration_ms) =
            if let Some((status, duration)) = status_and_duration.split_once(':') {
                let parsed = duration.parse::<u64>().map_err(|_| {
                    anyhow::anyhow!(
                        "invalid --evidence '{}'; duration_ms must be an unsigned integer",
                        raw
                    )
                })?;
                (status, parsed)
            } else {
                (status_and_duration, 0)
            };

        if status.trim().is_empty() {
            anyhow::bail!("invalid --evidence '{}'; status cannot be empty", raw);
        }

        out.push(Evidence {
            name: name.trim().to_string(),
            status: status.trim().to_string(),
            duration_ms,
            artifact_refs: vec![],
            summary: None,
        });
    }

    Ok(out)
}

pub fn run(args: ShipArgs) -> anyhow::Result<()> {
    let root = find_repo_root()?;
    let store = ClawStore::open(&root)?;

    // Load intent
    let intent_obj_id = store
        .get_ref(&format!("intents/{}", args.intent))?
        .ok_or_else(|| anyhow::anyhow!("intent not found: {}", args.intent))?;
    let intent_obj = store.load_object(&intent_obj_id)?;
    let mut intent = match intent_obj {
        Object::Intent(i) => i,
        _ => anyhow::bail!("not an intent"),
    };

    // Load revision
    let rev_id = store
        .get_ref(&args.revision_ref)?
        .ok_or_else(|| anyhow::anyhow!("ref not found: {}", args.revision_ref))?;
    let revision_obj = store.load_object(&rev_id)?;
    let revision = match revision_obj {
        Object::Revision(r) => r,
        _ => anyhow::bail!("ref is not a revision: {}", args.revision_ref),
    };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;

    let registered_agent = ensure_registered_signing_agent(&store, &args.agent)?;
    let keypair = keypair_for_agent(&registered_agent)?;
    let evidence = parse_evidence(&args.evidence)?;

    let public = CapsulePublic {
        agent_id: registered_agent.agent_id.clone(),
        agent_version: registered_agent.agent_version.clone(),
        toolchain_digest: None,
        env_fingerprint: None,
        evidence,
    };

    let capsule = build_capsule(&rev_id, public, None, None, &keypair)?;
    let capsule_id = store.store_object(&Object::Capsule(capsule))?;

    // Add capsule mapping refs
    store.set_ref(&format!("capsules/{}", rev_id.to_hex()), &capsule_id)?;
    store.set_ref(
        &format!("capsules/by-revision/{}", rev_id.to_hex()),
        &capsule_id,
    )?;
    // Backward-compatible short reverse key.
    store.set_ref(
        &format!("capsules/by-revision/{}", &rev_id.to_hex()[..16]),
        &capsule_id,
    )?;

    let mut finalized_change: Option<ChangeId> = revision.change_id;
    if finalized_change.is_none() && intent.change_ids.len() == 1 {
        if let Ok(single_change) = ChangeId::from_string(&intent.change_ids[0]) {
            finalized_change = Some(single_change);
        }
    }

    if let Some(change_id) = finalized_change {
        let change_ref = format!("changes/{change_id}");
        let Some(change_obj_id) = store.get_ref(&change_ref)? else {
            anyhow::bail!("change not found: {change_id}");
        };
        let change_obj = store.load_object(&change_obj_id)?;
        let Object::Change(mut change) = change_obj else {
            anyhow::bail!("ref {} is not a change", change_ref);
        };
        if change.intent_id != intent.id {
            anyhow::bail!(
                "change {} does not belong to intent {}",
                change.id,
                intent.id
            );
        }
        change.head_revision = Some(rev_id);
        change.status = ChangeStatus::Integrated;
        change.updated_at_ms = now_ms;
        let new_change_id = store.store_object(&Object::Change(change.clone()))?;
        store.set_ref(&change_ref, &new_change_id)?;

        let change_id_string = change.id.to_string();
        if !intent.change_ids.iter().any(|id| id == &change_id_string) {
            intent.change_ids.push(change_id_string);
        }
    }

    // Update intent status coherently with associated changes.
    let mut updated_intent = intent;
    let mut intent_changed = false;

    let all_integrated = if updated_intent.change_ids.is_empty() {
        false
    } else {
        let mut all_integrated = true;
        for change_id in &updated_intent.change_ids {
            let Some(change_obj_id) = store.get_ref(&format!("changes/{change_id}"))? else {
                all_integrated = false;
                break;
            };
            let change_obj = store.load_object(&change_obj_id)?;
            let Object::Change(change) = change_obj else {
                all_integrated = false;
                break;
            };
            if change.status != ChangeStatus::Integrated {
                all_integrated = false;
                break;
            }
        }
        all_integrated
    };

    if all_integrated {
        if updated_intent.status != IntentStatus::Done
            && updated_intent.status != IntentStatus::Superseded
        {
            updated_intent.status = IntentStatus::Done;
            intent_changed = true;
        }
    } else if updated_intent.change_ids.is_empty() && updated_intent.status != IntentStatus::Done {
        // Backward-compatible behavior for intents without explicit changes.
        updated_intent.status = IntentStatus::Done;
        intent_changed = true;
    } else if updated_intent.status == IntentStatus::Done {
        updated_intent.status = IntentStatus::Open;
        intent_changed = true;
    }

    if intent_changed {
        updated_intent.updated_at_ms = now_ms;
    }

    let new_intent_id = store.store_object(&Object::Intent(updated_intent.clone()))?;
    store.set_ref(&format!("intents/{}", updated_intent.id), &new_intent_id)?;

    println!("Shipped intent: {}", updated_intent.id);
    println!("  Capsule: {capsule_id}");
    println!("  Revision: {rev_id}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_evidence;

    #[test]
    fn parses_evidence_with_equals_and_optional_duration() {
        let parsed =
            parse_evidence(&["test=pass:1200".to_string(), "lint=pass".to_string()]).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "test");
        assert_eq!(parsed[0].status, "pass");
        assert_eq!(parsed[0].duration_ms, 1200);
        assert_eq!(parsed[1].name, "lint");
        assert_eq!(parsed[1].status, "pass");
        assert_eq!(parsed[1].duration_ms, 0);
    }

    #[test]
    fn rejects_invalid_evidence_format() {
        assert!(parse_evidence(&["broken".to_string()]).is_err());
        assert!(parse_evidence(&["=pass".to_string()]).is_err());
        assert!(parse_evidence(&["test=".to_string()]).is_err());
        assert!(parse_evidence(&["test=pass:notnum".to_string()]).is_err());
    }
}

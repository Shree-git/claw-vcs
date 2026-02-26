use std::path::Path;
use std::process::Command;

use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Capsule, Revision};
use claw_store::ClawStore;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Serialize, Deserialize)]
pub struct GitProvenanceNote {
    pub version: u8,
    pub revision_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capsule: Option<Capsule>,
    #[serde(default)]
    pub policy_evidence: Vec<String>,
}

impl GitProvenanceNote {
    pub fn from_revision(
        store: &ClawStore,
        revision_id: &ObjectId,
        revision: &Revision,
    ) -> anyhow::Result<Option<Self>> {
        let capsule = match revision.capsule_id {
            Some(capsule_id) => {
                let obj = store.load_object(&capsule_id)?;
                match obj {
                    Object::Capsule(c) => Some(c),
                    _ => anyhow::bail!(
                        "revision capsule pointer {} does not reference a capsule object",
                        capsule_id
                    ),
                }
            }
            None => None,
        };

        if capsule.is_none() && revision.policy_evidence.is_empty() {
            return Ok(None);
        }

        Ok(Some(Self {
            version: 1,
            revision_id: revision_id.to_hex(),
            capsule,
            policy_evidence: revision.policy_evidence.clone(),
        }))
    }
}

pub fn write_note(
    git_dir: &Path,
    notes_ref: &str,
    commit_hex: &str,
    note: &GitProvenanceNote,
) -> anyhow::Result<()> {
    let note_text = serde_json::to_string_pretty(note)?;
    run_git(
        git_dir,
        &[
            "notes", "--ref", notes_ref, "add", "-f", "-m", &note_text, commit_hex,
        ],
    )?;
    Ok(())
}

pub fn read_note(
    git_dir: &Path,
    notes_ref: &str,
    commit_hex: &str,
) -> anyhow::Result<Option<GitProvenanceNote>> {
    let output = Command::new("git")
        .arg(format!("--git-dir={}", git_dir.display()))
        .args(["-c", "user.name=claw", "-c", "user.email=claw@local"])
        .args(["notes", "--ref", notes_ref, "show", commit_hex])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run git for notes lookup: {e}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let body = std::str::from_utf8(&output.stdout)
        .map_err(|e| anyhow::anyhow!("git notes payload is not utf-8: {e}"))?;
    match serde_json::from_str::<GitProvenanceNote>(body.trim()) {
        Ok(note) => Ok(Some(note)),
        Err(err) => {
            warn!("skipping unparsable provenance note on {commit_hex}: {err}");
            Ok(None)
        }
    }
}

pub fn import_note_into_store(
    store: &ClawStore,
    revision_id: &ObjectId,
    note: GitProvenanceNote,
) -> anyhow::Result<()> {
    if note.version != 1 {
        anyhow::bail!("unsupported provenance note version: {}", note.version);
    }

    if let Some(mut capsule) = note.capsule {
        // Only attach capsules when they bind to this imported revision id.
        if capsule.revision_id == *revision_id {
            let capsule_id = store.store_object(&Object::Capsule(capsule.clone()))?;
            store.set_ref(
                &format!("capsules/by-revision/{}", revision_id.to_hex()),
                &capsule_id,
            )?;
            store.set_ref(
                &format!("capsules/by-revision/{}", &revision_id.to_hex()[..16]),
                &capsule_id,
            )?;
            store.set_ref(&format!("capsules/{}", revision_id.to_hex()), &capsule_id)?;
        } else {
            capsule.revision_id = *revision_id;
            let blob = Object::Blob(claw_core::types::Blob {
                data: serde_json::to_vec(&capsule)?,
                media_type: Some("application/json".to_string()),
            });
            let blob_id = store.store_object(&blob)?;
            store.set_ref(
                &format!("notes/provenance/unbound-capsule/{}", revision_id.to_hex()),
                &blob_id,
            )?;
        }
    }

    if !note.policy_evidence.is_empty() {
        let blob = Object::Blob(claw_core::types::Blob {
            data: serde_json::to_vec(&note.policy_evidence)?,
            media_type: Some("application/json".to_string()),
        });
        let blob_id = store.store_object(&blob)?;
        store.set_ref(
            &format!("notes/provenance/policy-evidence/{}", revision_id.to_hex()),
            &blob_id,
        )?;
    }

    Ok(())
}

fn run_git(git_dir: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new("git")
        .arg(format!("--git-dir={}", git_dir.display()))
        .args(["-c", "user.name=claw", "-c", "user.email=claw@local"])
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run git command: {e}"))?;

    if !status.success() {
        anyhow::bail!(
            "git command failed: git --git-dir={} {:?}",
            git_dir.display(),
            args
        );
    }
    Ok(())
}

# Data layout

This page documents repository files that operators may need for backup,
restore, migration, and incident work.

## Repository root

Claw metadata lives under `.claw/` in the repository root. `claw init` creates
the store directories and writes initial repository metadata.

| Path | Purpose | Operator notes |
|---|---|---|
| `.claw/objects/` | Loose COF-encoded objects, sharded by object ID prefix. | Back up with the rest of `.claw/`. |
| `.claw/packs/` | Packed object data. | Treat as repository data. |
| `.claw/indices/` | Pack and lookup indexes. | Can be rebuilt only when tooling says so. |
| `.claw/cache/` | Local cache data. | Not a public storage contract. |
| `.claw/meta.db` | Reserved local index path. | Internal implementation detail. |
| `.claw/refs/heads/` | Branch refs. | Do not edit by hand. |
| `.claw/refs/intents/` | Intent refs. | Do not edit by hand. |
| `.claw/refs/changes/` | Change refs. | Do not edit by hand. |
| `.claw/refs/workstreams/` | Workstream refs. | Do not edit by hand. |
| `.claw/refs/agents/` | Agent registration refs. | Public agent metadata, not private keys. |
| `.claw/refs/policies/` | Policy refs. | Created by `claw policy`. |
| `.claw/refs/capsules/` | Capsule reverse lookup refs. | Created by `claw ship`. |
| `.claw/HEAD` | Current branch or detached object ID. | Use `claw checkout` and branch commands. |
| `.claw/reflogs/` | Append-only ref update history. | Include in backups. |
| `.claw/repo.toml` | Legacy store metadata written by `claw init`. | Kept for compatibility. |
| `.claw/config.toml` | Runtime config schema `config_version = 1`. | Use admin migration commands. |
| `.claw/remotes.toml` | Remote aliases and auth profile names. | Written by `claw remote`. |
| `.claw/MERGE_STATE.toml` | Active merge state. | Present only during conflicted merges. |
| `.claw/backups/` | Metadata backups from admin commands. | Replicate off-host. |
| `.claw/migrations/ledger.jsonl` | Admin migration and rollback ledger. | Keep for audit and rollback review. |
| `.claw/support/` | Support bundles. | Review before sharing outside your org. |

## User home

Auth profiles live outside the repository:

| Path | Purpose |
|---|---|
| `~/.claw/auth.toml` | Named auth profiles and encrypted token fields. |
| `~/.claw/auth.key` | Local symmetric key used to encrypt auth tokens. |
| `~/.claw/agent-keys/` | Local Ed25519 private keys for registered agents. |

## Backup rule

Back up the full `.claw/` directory as one unit. Do not copy only `objects/` or
only `refs/`; the store needs refs, logs, config, and object data to recover.

User-home files are credential material, not repository history. Back up or
re-provision them through your credential process, and never commit them into a
repository.

## Object storage

Loose objects are COF bytes under `.claw/objects/<shard>/<object-id>`. Pack files
use `.clwpack` data files and `.idx` index files under `.claw/packs/` in the
current implementation. Treat both loose and packed object paths as opaque.

## Ref names

Claw refs are repository-relative names such as `heads/main`, `intents/<id>`,
`changes/<id>`, `agents/<name>`, and `policies/<id>`. Refs are normal files
under `.claw/refs/`, but manual edits can bypass compare-and-swap checks and
reflog recording.

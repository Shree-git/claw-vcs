# Object model

Claw stores typed, content-addressed objects. Object IDs are BLAKE3 hashes with
domain separation over object type and payload, displayed as `clw_` plus
lowercase Base32. The on-disk payload is deterministic Protocol Buffers wrapped
in COF, the Claw Object Format.

The current model has 12 first-class primitives. They are deliberately small:
refs, commands, sync, policy, and Git interop compose these objects instead of
inventing separate hidden state.

## Primitive map

| Type | Tag | Primary ref namespace | Purpose |
|---|---:|---|---|
| Blob | `0x01` | none | File bytes with optional media type. |
| Tree | `0x02` | through revisions | Directory entries pointing at blobs or subtrees. |
| Patch | `0x03` | through revisions | Codec-specific operations from one file object to another. |
| Revision | `0x04` | `heads/*` | History node with parents, patches, author, timestamp, and tree. |
| Snapshot | `0x05` | none | Atomic record linking a revision to the captured tree root. |
| Intent | `0x06` | `intents/<intent-id>` | Goal, constraints, acceptance tests, policies, and status. |
| Change | `0x07` | `changes/<change-id>` | Implementation attempt linked to one intent. |
| Conflict | `0x08` | merge state, future refs | Merge conflict record for base/left/right revisions. |
| Capsule | `0x09` | `capsules/*` | Signed provenance and evidence envelope for one revision. |
| Policy | `0x0a` | `policies/<policy-id>` | Versioned enforcement rules. |
| Workstream | `0x0b` | `workstreams/<id>` | Ordered stack of related changes. |
| RefLog | `0x0c` | `.claw/reflogs/*` | Append-only reference update history. |

## Blob

A blob is raw file content:

- `data`: file bytes.
- `media_type`: optional MIME-like hint.

Blobs have no dependencies. File mode and path are not stored on the blob; those
belong to tree entries so the same bytes can be referenced from different paths
or modes.

## Tree

A tree is a directory listing:

- `entries`: ordered `TreeEntry` records.
- `TreeEntry.name`: single path component.
- `TreeEntry.mode`: `Regular`, `Executable`, `Symlink`, or `Directory`.
- `TreeEntry.object_id`: blob or subtree object ID.

Tree entry names must not be empty, `.`, `..`, contain `/`, `\`, NUL, or control
characters. A tree cannot contain duplicate entry names. Full paths are formed
by walking nested trees.

## Patch

A patch captures codec-specific file operations:

- `target_path`: repository-relative path.
- `codec_id`: codec such as `text/line`, `json/tree`, or `binary`.
- `base_object`: optional old blob ID.
- `result_object`: optional new blob ID.
- `ops`: codec operations.
- `codec_payload`: optional opaque codec payload.

Each patch operation includes an address, operation type, optional old/new bytes,
and optional context hash. Revisions reference patch objects so readers can
inspect both the final tree and the semantic changes that produced it.

## Revision

A revision is the main history node:

- `change_id`: optional linked change ID.
- `parents`: zero, one, or many parent revision IDs.
- `patches`: patch object IDs.
- `snapshot_base`: optional base snapshot object ID.
- `tree`: optional root tree ID for the resulting state.
- `capsule_id`: optional capsule ID.
- `author`: human or agent label used for the snapshot.
- `created_at_ms`: Unix epoch milliseconds.
- `summary`: revision message.
- `policy_evidence`: legacy/export-facing evidence strings.

`claw snapshot` writes revisions and advances the current branch ref. Merge
completion writes a multi-parent revision.

## Snapshot

A snapshot records an atomic capture:

- `tree_root`: root tree captured from the worktree.
- `revision_id`: revision created for that capture.
- `created_at_ms`: Unix epoch milliseconds.

The current CLI primarily exposes snapshot behavior through revision creation.
The primitive remains useful for APIs and future audit paths that need an
explicit "this tree was captured at this time" record.

## Intent

An intent is the structured "why":

- `id`: ULID-style intent ID.
- `title`: short human-readable goal.
- `goal`: longer objective.
- `constraints`: limits or invariants.
- `acceptance_tests`: expected checks or outcomes.
- `links`: external references.
- `policy_refs`: policy IDs or refs that gate linked changes.
- `agents`: expected or participating agent IDs.
- `change_ids`: linked changes.
- `depends_on`: prerequisite intent IDs.
- `supersedes`: replaced intent IDs.
- `status`: `Open`, `Blocked`, `Done`, or `Superseded`.
- `created_at_ms`, `updated_at_ms`: Unix epoch milliseconds.

The current CLI creates, lists, shows, and updates status. It does not yet expose
all intent fields, so integrations that need `policy_refs` must write through an
API/import path or a future CLI surface.

## Change

A change is one implementation attempt for an intent:

- `id`: ULID-style change ID.
- `intent_id`: owning intent.
- `head_revision`: optional current revision for the attempt.
- `workstream_id`: optional stack/workstream membership.
- `status`: `Open`, `Ready`, `Integrated`, or `Abandoned`.
- `created_at_ms`, `updated_at_ms`: Unix epoch milliseconds.

`claw change create --intent <intent-id>` links the change back into the intent.
`claw snapshot --change <change-id>` records the revision relationship directly.
`claw ship` can infer the single linked change when an intent has exactly one
change.

## Conflict

A conflict records a merge disagreement:

- `base_revision`: optional common ancestor.
- `left_revision`, `right_revision`: revisions being merged.
- `file_path`: conflicted repository path.
- `codec_id`: codec that detected the conflict.
- `left_patch_ids`, `right_patch_ids`: contributing patches.
- `resolution_patch_ids`: patches produced by resolution.
- `status`: `Open` or `Resolved`.
- `created_at_ms`: Unix epoch milliseconds.

The CLI currently writes merge sidecars and `.claw/MERGE_STATE.toml` for active
conflicts. The object type is the durable model for conflict-aware APIs.

## Capsule

A capsule binds provenance and evidence to one revision:

- `revision_id`: revision being claimed.
- `public_fields.agent_id`: agent identity.
- `public_fields.agent_version`: optional tool version.
- `public_fields.toolchain_digest`: optional toolchain digest.
- `public_fields.env_fingerprint`: optional environment fingerprint.
- `public_fields.evidence`: evidence records.
- `encrypted_private`: optional encrypted private payload.
- `encryption`: encryption scheme marker.
- `key_id`: optional encryption/signing key ID for policy checks.
- `recipients`: optional per-recipient encrypted content-key envelopes.
- `signatures`: signer IDs and Ed25519 signatures.

Evidence records contain `name`, `status`, optional `duration_ms`, artifact refs,
summary, revision binding, runner identity, command, exit code, timestamps,
environment digest, log/artifact digests, expiration, trust domain, and optional
signature bytes. `claw ship` creates capsules and stores reverse refs under
`capsules/` and `capsules/by-revision/`.

## Policy

A policy is an in-repo gate:

- `policy_id`: stable ID.
- `required_checks`: evidence names that must have passing status.
- `required_reviewers`: signer agent IDs or key IDs that must be present.
- `sensitive_paths`: path prefixes requiring private metadata handling.
- `quarantine_lane`: block automated integration for sensitive paths.
- `min_trust_score`: required pass ratio as `0.0-1.0` or a percent string.
- `visibility`: `Public`, `Private`, or `EncryptedMetadataRequired`.
- `authorized_recipients`: recipient IDs that must have encrypted capsule envelopes.
- `revoked_recipients`: recipient IDs that must not appear in capsule envelopes.
- `evidence_policy`: freshness, runner, and digest requirements for evidence.

`claw policy create` stores policy objects. Enforcement only applies when the
intent associated with a revision references the policy.

## Workstream

A workstream is an ordered stack:

- `workstream_id`: stable stack ID.
- `change_stack`: ordered change IDs.

Workstreams model dependent or review-stacked changes without overloading
branch names. The object exists before the CLI has a complete stack workflow.

## RefLog

A reflog records ref updates:

- `ref_name`: updated ref.
- `entries`: update entries.
- `old_target`: previous object ID, if any.
- `new_target`: new object ID.
- `author`: updater label.
- `message`: update reason.
- `timestamp`: Unix epoch milliseconds.

Ref logs are stored under `.claw/reflogs/` and should be included in backups.
They are append-only audit data, not a user-editable recovery mechanism.

## Encoding and identity

Objects are serialized with Protocol Buffers and wrapped in COF. COF includes a
magic value, version, type tag, flags, compression mode, payload length, payload,
and CRC32.

Object hashes use BLAKE3 with domain separation over object type, COF version,
and payload. Different object types do not share the same hash domain, even when
their protobuf payload bytes match.

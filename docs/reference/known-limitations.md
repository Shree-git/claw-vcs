# Known Limitations

Claw VCS is v0.1 experimental software. It is appropriate for local exploration, demos, and design feedback, but it is not yet recommended as the sole source of truth for production repositories.

## Object and Protocol Stability

- The CLI, daemon API, object format, policy semantics, and sync protocol may change before v1.0.
- Repositories created by v0.1.x should be backed up before any upgrade.
- Future versions will include migration notes where possible, but pre-1.0 compatibility is not guaranteed.

## Event Streaming

- Daemon-generated sync ref updates use the internal event bus.
- A polling path remains as a compatibility fallback for externally modified refs.
- Agent integrations should tolerate duplicate or delayed events.

## Visibility and Private Metadata

- `EncryptedMetadataRequired` visibility means encrypted private capsule fields are required and a capsule `key_id` must match an authorized signer key in policy context. The legacy `restricted` spelling is accepted as a compatibility alias.
- Policies may define `authorized_recipients`; recipient envelopes use X25519/BLAKE3/XChaCha20-Poly1305 to wrap the capsule content key for policy-authorized recipient IDs.
- Policies may define `revoked_recipients`; policy evaluation fails if a capsule includes an envelope for a revoked recipient.
- `claw ship --private-file ... --recipient-key ...` can create recipient-encrypted private capsule payloads, and `claw show --decrypt-private` can decrypt them for a matching recipient key.
- Revocation is enforced on future policy evaluation. It does not retroactively decrypt, rewrite, or remove existing private capsule payloads.

## Git Bridge

- Git import/export is intended for interop, testing, and migration paths.
- Bridge behavior must be verified with real Git commands such as `git fsck`, `git log`, `git cat-file`, and checkout tests before relying on exported repositories.
- Unsupported or lossy Git features should be documented in migration notes for each release.

## Patching and Merge

- Text, JSON, and binary codecs are implemented, but patch commutation remains conservative.
- Binary files use replacement semantics.
- Complex semantic merges for formats such as YAML, TOML, SQL migrations, and protobuf schemas are future codec work.

## Workflow Model

- There is intentionally no staging area. `claw snapshot` captures the working tree atomically.
- Teams that depend on partial staging should keep using Git for those workflows until they have validated a Claw-native alternative.
- `claw ship` defaults to `--revision-ref heads/main`, not the current branch.
  Branch automation must pass the intended ref explicitly.
- `claw integrate` requires `--right`, including when `--dry-run` is used.

## Policy Attachment

- The CLI can attach existing policy refs with `claw intent policy add`.
- Policy enforcement only applies when an intent references the policy. A policy
  object named `default` is not global.

## Agent Key Lifecycle

- Agent registration creates or repairs local signing keys.
- `claw agent rotate` replaces the trusted public key and local signing key for
  an existing agent.
- `claw agent revoke` blocks future `claw ship --agent <name>` use and removes
  the registration from integration provenance trust checks.
- The current CLI does not import caller-supplied public keys and does not have
  a standalone `agent keygen` command.
- Revocation does not rewrite old capsules or invalidate historical signatures;
  it changes future trust decisions.

## Platforms

- Release tooling targets macOS, Linux, and Windows.
- Windows CI covers workspace compatibility plus release-channel smoke checks. Broader real-world validation for executable bits, symlinks, reserved filenames, path length, and CRLF handling should still be recorded before broad Windows rollout.

## Remote Sync

- Self-hosted daemon deployments are the primary supported remote model.
- Hosted ClawLab-style remotes are planned and should not be assumed live unless release notes say so.
- Sync clients should handle interrupted streams, missing objects, stale refs, auth failures, and protocol mismatches as normal error cases.

## Evidence Freshness

- Evidence freshness policies support exact revision match, evidence-after-revision checks, expiration, max age, trusted runner identity, command/exit code, environment digest, and log or artifact digest requirements.
- Freshness enforcement only runs for policies with `require_fresh_evidence` enabled.

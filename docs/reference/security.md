# Security Reference

This page defines launch-hardening behavior for daemon and sync security surfaces.

## Daemon Auth And TLS

- Production daemon binds beyond localhost must use bearer auth when `auth.require_auth_for_daemon = true`.
- Production daemon binds beyond localhost must use daemon TLS or TLS termination when `tls.require_for_non_localhost = true`.
- Bearer credentials are passed as `authorization: Bearer <token>` and must not be logged. Debug output for sync transports redacts configured bearer tokens.
- Use `--auth-token`, `--auth-profile`, or the configured default auth profile for daemon startup.
- `--auth-principal`, `--auth-role`, and repeated `--auth-scope` configure the sync authorization grant attached to the daemon bearer token. The default role is `admin` for compatibility.
- `--client-ca-cert <path>` enables required client certificate verification for the gRPC listener. It must be used with `--tls-cert` and `--tls-key`.
- `claw sync` can connect to TLS and mTLS gRPC remotes with `--tls-ca-cert`, `--tls-domain`, `--client-cert`, and `--client-key`. Client certificates require both the certificate and key.

## Sync Authorization

The sync service supports a concrete role/scope model. Bearer-authenticated daemon requests are annotated with a server-controlled principal and token ID before they reach the sync service; incoming caller-provided principal metadata is overwritten by the auth interceptor.

Roles:

- `reader`: `sync:hello`, `refs:read`, `objects:read`, `events:read`
- `object-writer`: reader object access plus `objects:write`
- `ref-writer`: reader object/ref access plus `refs:write`
- `writer`: read access plus `objects:write` and `refs:write`
- `event-reader`: `sync:hello`, `events:read`
- `admin`: `sync:*`

Scopes:

- `sync:hello`, `sync:*`
- `refs:read`, `refs:write`, `refs:*`
- `objects:read`, `objects:write`, `objects:*`
- `events:read`, `events:*`

Authorization failures return gRPC `PermissionDenied`. Local unauthenticated daemon usage remains compatibility-oriented; use bearer auth plus role/scope flags when a daemon is shared beyond a single trusted local user.

## Visibility Semantics

- `public`: no private capsule material is required.
- `private`: capsule private fields must be encrypted and include encryption metadata.
- `encrypted-metadata-required`: capsule private fields must be encrypted, include encryption metadata, include a non-empty `key_id`, and policy evaluation context must contain that key ID as an authorized signer key. Missing authorization fails closed. The legacy spelling `restricted` is accepted only as a compatibility alias.
- Policies can also define `authorized_recipients`. When present, encrypted
  private capsule fields must include recipient envelopes for every authorized
  recipient and no unauthorized recipient envelopes. Envelopes wrap the capsule
  content key with X25519, BLAKE3 key derivation, and XChaCha20-Poly1305.
- Policies can define `revoked_recipients`. Any capsule that still includes an
  envelope for a revoked recipient fails policy evaluation.

## Agent Keys

- `claw agent register --name <agent>` generates or verifies a local Ed25519
  signing key and stores public registration metadata in the repo.
- Private agent keys live under `~/.claw/agent-keys/`, outside the repository.
- Do not commit private agent keys, auth tokens, TLS private keys, or support
  bundles without review.
- There is no current `agent revoke` command. Remove trust through policy,
  integration denylists, and runner credential rotation.
- Old signatures remain useful for attribution even after the signer is no
  longer trusted for future policy decisions.

## Auth Token Storage

- `claw auth token set` stores auth profiles in `~/.claw/auth.toml`.
- Tokens are encrypted with a local key at `~/.claw/auth.key`.
- Treat both files as credential material. Back them up or re-provision them
  through your normal secret-management process.

## Sync Protocol Security Hooks

- Sync `Hello` capability negotiation returns only daemon-supported capabilities plus the protocol marker. Current baseline: `protocol:claw-sync/1`, `partial-clone`, `event-bus`, `request-limits`. Clients run compatibility checks by default and fail closed when the negotiated protocol marker is absent.
- Event subscriptions use an internal event bus for daemon-generated ref changes. The stream emits `ref_created` and `ref_updated` events from sync ref updates.
- Sync server options enforce per-minute request rate limits when configured with `--rate-limit-per-minute` or `queues.rate_limit_per_minute`.
- Push object uploads enforce per-chunk and per-request byte limits, configurable with `--max-push-chunk-bytes`, `--max-push-request-bytes`, `queues.max_push_chunk_bytes`, and `queues.max_push_request_bytes`.
- Sync ref/object actions emit structured `sync_audit_event` tracing records with request ID, principal, token ID, action, resource, outcome, and denial reason when available.
- gRPC clients send `x-claw-replay-nonce` on `PushObjects` and `UpdateRefs`; HTTP clients send the same value as the `idempotency-key` for mutating requests. The daemon can require nonces with `--require-replay-nonce`; otherwise duplicate nonces are still rejected when present.

Known limitations:

- Role/scope enforcement is wired for the core `SyncService` ref/object methods. Capsule reads also redact recipient-encrypted private fields unless the authenticated principal is a listed recipient. Other daemon gRPC services still rely on bearer authentication only until they receive service-specific resource models.
- Evidence freshness is enforced by policy only when `require_fresh_evidence`
  is enabled on the policy object.

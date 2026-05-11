# Evidence Schema

Evidence records should be stable enough for policy and humans to inspect years later.

Recommended fields:

| Field | Meaning |
|---|---|
| `name` | Stable check name such as `test`, `lint`, or `security-scan`. |
| `command` | Command or job that produced the evidence. |
| `exit_code` | Process exit status. |
| `started_at` | Start timestamp. |
| `ended_at` | End timestamp. |
| `duration_ms` | Runtime in milliseconds. |
| `revision_id` | Exact Claw revision evaluated. |
| `environment_digest` | Digest of runner/toolchain environment. |
| `runner_identity` | Runner key or identity that produced the evidence. |
| `log_digest` | Digest of logs. |
| `artifact_digest` | Digest of generated artifacts. |
| `expires_at` | Freshness deadline. |
| `trust_domain` | Domain such as `local`, `ci`, or `release`. |
| `signature` | Signature over the evidence payload. |

Policy should fail closed when required fields are missing.

Freshness-critical fields are `revision_id`, `started_at` or `ended_at`,
`expires_at`, `runner_identity`, `command`, `exit_code`,
`environment_digest`, and at least one of `log_digest` or `artifact_digest`.

## Current capsule fields

The v0.1 object model stores the full launch freshness schema inside capsule
evidence:

| Field | CLI source | Notes |
|---|---|---|
| `name` | `--evidence test=pass` | Required. Must match policy names exactly. |
| `status` | `--evidence test=pass` | Required. `pass` is the normal success value. |
| `duration_ms` | `--evidence test=pass:4200` | Optional, defaults to `0`. |
| `artifact_refs` | API/object model | Defaults to empty in CLI-created capsules. |
| `summary` | API/object model | Optional human-readable note. |
| `revision_id` | `claw ship` | Filled from `--revision-ref`. |
| `command` | `--evidence-command` | Required by freshness policies. |
| `exit_code` | `claw ship` | Defaults to `0` for `pass`, `1` otherwise. |
| `started_at_ms`, `ended_at_ms` | `claw ship` | Filled at capsule creation time unless an API producer supplies richer timings. |
| `environment_digest` | `--environment-digest` | Required by freshness policies. |
| `runner_identity` | `--runner` | Required and optionally allow-listed by freshness policies. |
| `log_digest` | `--log-digest` | One of log or artifact digest is required by freshness policies. |
| `artifact_digest` | `--artifact-digest` | One of log or artifact digest is required by freshness policies. |
| `expires_at_ms` | `--evidence-expires-in-ms` | Stored as creation time plus TTL. |
| `trust_domain` | API/object model | Optional trust-domain label. |
| `signature` | API/object model | Optional detached evidence signature bytes. |

Use `claw policy create --require-fresh-evidence` with `--trusted-runner` and
`--evidence-max-age-ms` to fail closed on stale or incomplete evidence.

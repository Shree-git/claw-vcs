# `claw ship`

Finalize an intent/change path and produce capsule evidence for policy evaluation.

## Examples

```bash
claw ship --intent <intent-id> --evidence test=pass --evidence lint=pass
claw ship --intent <intent-id> --evidence test=pass:1200 --co-sign <key>
claw ship \
  --intent <intent-id> \
  --revision-ref heads/main \
  --evidence test=pass \
  --evidence-command "cargo test --workspace" \
  --runner github-actions/release \
  --environment-digest sha256:<toolchain-digest> \
  --log-digest sha256:<log-digest> \
  --evidence-expires-in-ms 86400000
claw ship \
  --intent <intent-id> \
  --evidence test=pass \
  --private-file private-capsule.json \
  --recipient-key security-reviewer:security-key:<hex-x25519-public-key>
```

Evidence should reference checks that can be rerun or audited. Policy failures return a non-zero exit and should be treated as integration blockers.

Freshness policy fields are optional unless the referenced policy enables
`require_fresh_evidence`. When enabled, provide a trusted runner, command, exit
status implied by the evidence result, environment digest, log or artifact
digest, and an expiration window.

Private capsule metadata is encrypted when `--private-file` is used. Each
`--recipient-key` value wraps the capsule content key for one recipient ID and
must use the `recipient-id:key-id:hex-x25519-public-key` form.

## JSON Output

`claw ship` does not currently emit command-specific success JSON. Use global
JSON errors for automation failures:

```bash
claw --error-format json ship --intent <intent-id> --evidence test=pass
```

The created revision and capsule can be inspected afterward with:

```bash
claw log --json
claw show --json <capsule-id>
```

## Exit Codes

- `0`: revision and capsule were written.
- `2`: invalid CLI usage or malformed evidence/recipient input.
- `3`: not in a Claw repository.
- `5`: object/ref/key store read or write failure.
- `10`: policy evaluation denied shipping.

## Common Errors

- Unknown intent: run `claw intent list`.
- Missing or revoked agent key: run `claw agent register --name <agent>` or rotate the key.
- Evidence rejected by policy: include required checks, runner identity, command, digest, and expiration fields.
- Recipient key format rejected: use `recipient-id:key-id:hex-x25519-public-key`.
- Default revision ref mismatch: pass `--revision-ref <ref>` when shipping a branch other than `heads/main`.

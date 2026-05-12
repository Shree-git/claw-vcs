# `claw sync`

Pull from or push to configured remotes.

## Examples

```bash
claw sync pull --remote origin --ref-name heads/main
claw sync push --remote origin --ref-name heads/main
claw sync push --remote origin --ref-name heads/main --dry-run
claw sync clone <remote> <path>
```

`sync push --dry-run` connects to the remote, resolves the local and remote refs, and reports the object upload/ref update that would occur without mutating the remote.

Production remotes should require authentication and TLS. Use the sync-level TLS flags before the subcommand:

```bash
claw sync \
  --tls-ca-cert ./ca.pem \
  --tls-domain claw.example.com \
  --client-cert ./client.pem \
  --client-key ./client-key.pem \
  push --remote https://claw.example.com:50051
```

`--client-cert` and `--client-key` must be provided together. Protocol negotiation failures should be treated as compatibility issues, not retried blindly.

The daemon protocol supports partial-clone filters for object fetches, but the
current `claw sync clone` CLI performs a full clone and does not expose filter
flags yet.

## JSON Output

`claw sync` does not currently emit command-specific success JSON. Use global
JSON errors for automation failures:

```bash
claw --error-format json sync push --remote origin --ref-name heads/main --dry-run
```

Use `claw remote --json list`, `claw status --json`, and `claw version --json`
to gather structured local state around sync operations.

## Exit Codes

- `0`: sync operation or dry-run completed.
- `2`: invalid CLI usage.
- `3`: not in a Claw repository for local push/pull operations.
- `6`: missing or invalid authentication material.
- `7`: remote configuration or transport failure.
- `11`: client/server compatibility check failed.

## Common Errors

- Remote not found: run `claw remote list` or add one with `claw remote add`.
- TLS trust failure: pass `--tls-ca-cert` and `--tls-domain` for private CAs.
- mTLS pair incomplete: pass both `--client-cert` and `--client-key`.
- Auth failure: configure the token profile with `claw auth token set`.
- Protocol mismatch: upgrade/downgrade client or daemon to a compatible pair.
- Stale or missing ref: verify the remote ref name and retry after fetching current refs.

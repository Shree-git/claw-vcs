# Capsule evidence guide

Evidence names should be short, stable, and tied to commands or jobs that a
reviewer can rerun.

## Recommended names

| Evidence | Typical source |
|---|---|
| `test` | Unit, integration, or workspace test command. |
| `lint` | Formatter, linter, or static analysis command. |
| `security-scan` | Dependency audit or static security scan. |
| `smoke` | End-to-end smoke test. |
| `release-check` | Release gate or artifact verification. |

## Status values

Use `pass` only when the command completed successfully. Use failure text in the
change or PR notes when checks failed and were later fixed.

Current policy checks treat evidence names as exact names and accept passing
status case-insensitively. Keep names stable; changing `test` to `unit-test`
means a policy requiring `test` will not match.

CLI evidence uses this form:

```bash
claw ship \
  --intent <intent-id> \
  --agent release-bot \
  --evidence test=pass:4200 \
  --evidence lint=pass \
  --evidence-command "cargo test --workspace" \
  --runner github-actions/release \
  --environment-digest sha256:<toolchain-digest> \
  --log-digest sha256:<log-digest>
```

The optional third field in `--evidence` is `duration_ms`. Additional evidence
flags populate revision binding, command, runner identity, environment digest,
log/artifact digests, and expiration fields used by freshness policies.

## Private data

Do not place secrets, tokens, private URLs, or customer data in public evidence
fields. Use encrypted capsule private fields when policy requires private
metadata:

```bash
claw ship \
  --intent <intent-id> \
  --evidence test=pass \
  --private-file private-capsule.json \
  --recipient-key security-reviewer:security-key:<hex-x25519-public-key>
```

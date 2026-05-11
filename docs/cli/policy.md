# `claw policy`

Create, apply, inspect, and evaluate repository policies.

```bash
claw policy create --id release --check ci
claw policy create \
  --id release \
  --check test \
  --require-fresh-evidence \
  --trusted-runner github-actions/release \
  --evidence-max-age-ms 86400000
claw policy create \
  --id sensitive \
  --sensitive-path secrets/ \
  --visibility encrypted-metadata-required \
  --recipient security-reviewer
claw policy create \
  --id sensitive-v2 \
  --recipient security-reviewer \
  --revoked-recipient former-reviewer
claw policy apply --id release --check ci --dry-run
claw policy eval release --revision heads/main --json
```

`policy apply --dry-run` validates the policy definition, computes the object ID that would be written, and skips the object write and ref update.

`policy eval --json` emits `allowed`, `error`, policy object metadata, revision/capsule IDs, and the evaluation context. A denied policy exits with the policy exit-code family documented in `exit-codes.md`.

Recipient flags require capsules to carry encrypted recipient envelopes for the
listed recipient IDs. Revoked recipient flags fail closed if a capsule includes
an envelope for that recipient. Freshness flags require revision-bound evidence
with runner, command, exit status, expiration, and digest metadata.

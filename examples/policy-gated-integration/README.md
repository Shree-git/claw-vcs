# Policy-Gated Integration

```bash
claw policy create --id release \
  --check test \
  --check lint \
  --sensitive-path secrets/ \
  --min-trust-score 0.85

claw policy eval release --revision heads/main --path secrets/example.txt --json
```

Expected failure examples:

- Missing `test` evidence.
- Missing `lint` evidence.
- Evidence references a different revision.
- Evidence is stale under the configured freshness window.
- Sensitive path changed without encrypted private capsule fields.
- Capsule signer is not trusted by the policy.
- Trust score is below the configured threshold.

Run [`scripts/failure-cases.sh`](scripts/failure-cases.sh) for checked local
examples of common denied policy decisions.

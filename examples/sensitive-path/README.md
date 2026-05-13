# Sensitive Path

```bash
claw policy create --id sensitive \
  --check test \
  --sensitive-path secrets/ \
  --visibility encrypted-metadata-required \
  --recipient security-reviewer

mkdir -p secrets
printf 'placeholder\n' > secrets/example.txt
claw snapshot -m "touch sensitive path"
printf '{"review":"security"}\n' > private-capsule.json
claw ship \
  --intent <intent-id> \
  --evidence test=pass \
  --private-file private-capsule.json \
  --recipient-key security-reviewer:security-key:<hex-x25519-public-key>
claw policy eval sensitive --revision heads/main --path secrets/example.txt --json
```

`encrypted-metadata-required` requires encrypted capsule private fields for sensitive-path changes. When the policy lists recipients, the capsule must include encrypted recipient envelopes for those recipient IDs before policy allows the change.

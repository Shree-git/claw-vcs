# Sensitive Paths Workflow

Use this path for code touching secrets, deployment, auth, policy, or release infrastructure.

- Add sensitive prefixes to policy with `--sensitive-path`.
- Require registered signer identity and relevant evidence names.
- Use `encrypted-metadata-required` visibility when private metadata must be encrypted.
- Use policy `--recipient <id>` when private fields must carry envelopes for named recipients.
- Ship the encrypted private payload with `--private-file <path>` and one or more `--recipient-key <recipient-id>:<key-id>:<hex-x25519-public-key>` values.
- Evaluate with touched paths: `claw policy eval <policy> --revision <ref> --path secrets/config.toml --json`.
- Require human review before integration.

Encrypted private metadata is enforced by presence and key metadata. Recipient-aware policies also require encrypted recipient envelopes for every authorized recipient ID listed by policy.

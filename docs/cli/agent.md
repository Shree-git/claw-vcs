# `claw agent`

Manage local agent registration records and signing keys.

```bash
claw agent keygen --name ci-agent
claw agent register --name ci-agent
claw agent register --name hosted-agent --public-key <hex-ed25519-public-key>
claw agent rotate --name ci-agent --version "2026-05-11"
claw agent rotate --name hosted-agent --public-key <replacement-public-key>
claw agent revoke --name ci-agent --reason "runner compromise"
claw agent list --json
claw agent status ci-agent --json
```

Agent private keys are stored outside the repository under the user Claw home.

`agent keygen` creates a local signing key without registering repository trust.
Use it when a runner needs to provision its key before a maintainer registers
the public key.

`agent register --public-key` records an externally managed Ed25519 public key
without creating a local signing key. Use plain `register` for local agents that
should sign from the current machine.

`agent rotate` replaces the trusted public key and local signing key for an existing agent. Use `--public-key` for externally managed replacement keys. Use `--dry-run` to preview the operation without updating the repository or key store.

`agent revoke` marks the registration as revoked for future signing and integration decisions. Revoked agents are omitted from the integration trust registry, and `claw ship --agent <name>` refuses to sign as a revoked agent. Old signatures remain useful for attribution.

# `claw agent`

Manage local agent registration records and signing keys.

```bash
claw agent register --name ci-agent
claw agent rotate --name ci-agent --version "2026-05-11"
claw agent revoke --name ci-agent --reason "runner compromise"
claw agent list
claw agent status ci-agent
```

Agent private keys are stored outside the repository under the user Claw home.

`agent rotate` replaces the trusted public key and local signing key for an existing agent. Use `--dry-run` to preview the operation without updating the repository or key store.

`agent revoke` marks the registration as revoked for future signing and integration decisions. Revoked agents are omitted from the integration trust registry, and `claw ship --agent <name>` refuses to sign as a revoked agent. Old signatures remain useful for attribution.

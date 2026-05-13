# Agent registration

Register agents before they produce changes that need signed capsule evidence.

## Register identity

```bash
claw agent register \
  --name release-bot \
  --version "2026-05-11"
```

Use one identity per automation principal. Do not share one identity across
unrelated tools or teams.

The command creates or updates a repository registration and ensures a local
Ed25519 signing key exists for that agent. The public key is stored in the repo;
the private key is stored outside the repo under `~/.claw/agent-keys/`.

Check the result:

```bash
claw agent status release-bot
claw agent list
```

Expected `claw agent status <name>` output includes `Key: ... (verified)`.
`claw agent list` shows the compact form `key:verified`.

## Rotate or replace identities

When an agent key is rotated, keep old signed capsules readable and verifiable.
Use the explicit rotate command when replacement is intentional:

```bash
claw agent rotate --name release-bot --version "2026-05-12"
```

The CLI may still repair an active registration if the local key is missing or
mismatched, but planned rotations should use `agent rotate` so the operator
intent is visible in the command history.

Create a replacement identity when the automation principal changes. Reuse the
same identity only when it is the same principal with a new key or version.
Update policy reviewer IDs if policies pin public key IDs.

Revoke an identity when the runner or key should no longer be trusted:

```bash
claw agent revoke --name release-bot --reason "runner compromise"
```

Revocation blocks future signing through `claw ship --agent release-bot` and
removes that registration from integration provenance trust checks.

## Naming

Use names that survive tool changes:

- good: `release-bot`, `docs-agent`, `sync-smoke-runner`
- avoid: local hostnames, temporary CI job IDs, personal laptop names

## CI runner guidance

- Keep one stable identity per trusted runner class.
- Set `HOME` or the runner user consistently so the local private key path is
  stable across jobs that are expected to sign.
- Do not commit files from `~/.claw/agent-keys/`.
- If jobs are ephemeral, provision the private key through your secret manager or
  run `claw agent register` during bootstrap and treat the resulting new public
  key as a rotation event.

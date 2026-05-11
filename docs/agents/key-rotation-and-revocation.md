# Key Rotation and Revocation

Agent keys identify who signed capsules and evidence. Treat them as production credentials.

## Current command surface

```bash
claw agent register --name build-agent-01 --version "2026-05-11"
claw agent list
claw agent status build-agent-01
```

The current CLI does not have `claw agent keygen`, `claw agent rotate`, `claw
agent revoke`, or `--public-key` registration flags.

`claw agent register` generates or repairs a local Ed25519 signing key and
stores public registration metadata in the repository under `refs/agents/`. The
private key is local-only under `~/.claw/agent-keys/` using a hashed filename.
Re-running `register` for the same name updates the version and keeps the same
valid local key when possible. If the local key is missing or mismatched, the CLI
generates a replacement key and updates the stored public registration.

## Rotate

There is no explicit rotation command yet. Use this operational procedure:

1. Stop jobs that sign as the affected agent.
2. Back up the repository and record the current public key prefix from
   `claw agent status <name>`. If you need the full object ID, inspect the
   `agents/<name>` ref with repository tooling before changing it.
3. Move the local private key for that agent out of `~/.claw/agent-keys/` on the
   runner that should rotate. Do not delete the backup until verification
   finishes.
4. Run `claw agent register --name <name> --version <new-version>` on the
   runner that should own the new key.
5. Run `claw agent status <name>` and confirm the key is `verified`.
6. Ship a small test capsule with `claw ship --intent <intent-id>
   --revision-ref heads/<branch> --agent <name> --evidence smoke=pass`.
7. Update policy reviewer IDs only if your policies pin public key IDs instead
   of stable agent IDs.
8. Preserve old registration data so old capsules remain attributable.

Prefer stable agent IDs in policy where possible. Policies that pin raw public
key IDs need an update when the public key changes.

## Revoke

There is no first-class revocation object or `claw agent revoke` command in the
current CLI. Treat revocation as a policy and incident-response action:

1. Stop all runners that have the compromised private key.
2. Remove the affected agent ID or public key ID from policies that currently
   trust it.
3. If your integration layer has a denylist, add the public key ID there.
4. Audit capsules signed during the suspected compromise window.
5. Re-run required checks from trusted runner keys for affected revisions.
6. Rotate any encrypted private fields or external secrets the key could access.
7. Register a replacement identity or rotate the existing identity as described
   above.

Old signatures remain cryptographically valid. Revocation means future policy
evaluation must stop trusting that signer for new decisions.

## Compromise Response

Assume any capsule signed after the compromise window began is suspect. The
signature remains useful because it identifies exactly which claims used the
compromised key. Keep the old public registration available for audit even when
policy no longer trusts it.

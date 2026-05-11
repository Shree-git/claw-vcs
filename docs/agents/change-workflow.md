# Agent change workflow

Agents should work through the same intent and change objects as humans.

## Contract

An agent integration should:

- run `claw agent register --name <agent> --version <version>` during bootstrap
  or verify that registration already exists
- create or reuse an intent
- create a change for each implementation attempt
- snapshot with `--change <change-id>`
- ship with `--agent <agent>` and stable evidence names
- stop on any policy denial instead of retrying with weaker evidence

## Flow

1. Read or create an intent.
2. Create a change linked to that intent.
3. Modify files.
4. Run policy-required checks.
5. Create a snapshot.
6. Ship with evidence.
7. Stop for human review if policy denies ship or integration.

## Command sketch

```bash
claw agent register --name sync-fixer --version "2026-05-11"

intent_output="$(claw intent create \
  --title "Fix flaky sync test" \
  --goal "Remove nondeterminism")"
intent_id="$(printf '%s\n' "$intent_output" | awk '/^Created intent:/ {print $3; exit}')"

change_output="$(claw change create --intent "$intent_id")"
change_id="$(printf '%s\n' "$change_output" | awk '/^Created change:/ {print $3; exit}')"

# agent edits files here

cargo test --workspace
claw snapshot --change "$change_id" -m "Fix flaky sync test"
claw ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent sync-fixer \
  --evidence test=pass
```

Use `claw intent --json create` and `claw change --json create` if your
integration prefers JSON output. The shell sketch above avoids a hard dependency
on `jq`.
If the agent snapshots on a feature branch, pass that branch through
`--revision-ref heads/<branch>`; the default is `heads/main`.

## Handoff

When handing work to another agent or human, include:

- intent ID
- change ID
- last revision ID
- checks run
- policy denial text, if any
- files intentionally left untouched

## Event and daemon integrations

The CLI is the current stable integration path for local demos. Daemon and sync
surfaces are available for controlled deployments, but agent integrations should
pin the Claw version and validate the daemon compatibility check before relying
on remote behavior.

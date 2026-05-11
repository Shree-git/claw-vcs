# Agent docs

Claw treats agents as first-class producers of changes. Agents should create or
use intents, record changes, attach evidence, and sign capsules.

## Pages

- [Agent registration](agent-registration.md)
- [Agent change workflow](change-workflow.md)
- [Capsule evidence guide](capsule-evidence.md)
- [Evidence schema](evidence-schema.md)
- [Evidence freshness](evidence-freshness.md)
- [Key rotation and revocation](key-rotation-and-revocation.md)
- [Integration guide](integration-guide.md)

## Agent contract

An agent should:

- run under a registered identity
- use a stable agent ID and version string
- attach repeatable evidence names
- avoid writing secrets to public capsule fields
- keep private capsule fields encrypted when policy requires it
- stop when policy denies a ship or integration

## Minimal CLI integration

```bash
claw agent register --name docs-agent --version "2026-05-11"
claw intent create --title "Update docs" --goal "Clarify agent workflow"
claw change create --intent <intent-id>
claw snapshot --change <change-id> -m "Clarify agent workflow"
claw ship --intent <intent-id> --revision-ref heads/main --agent docs-agent --evidence test=pass
```

Use `claw agent status <name>` before signing if the runner may have lost its
local key. A usable local signer reports `Key: ... (verified)`; `claw agent
list` shows the compact form `key:verified`.

## Current limits

- Agent registration generates or repairs local keys; it does not import a
  caller-provided public key.
- Agent rotation and revocation are supported for repository registrations, but
  there is no standalone `agent keygen` command yet.
- Policy enforcement depends on policies referenced by intents. Creating a
  policy object does not attach it to every intent.

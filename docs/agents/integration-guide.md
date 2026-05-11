# Agent Integration Guide

This guide describes the current v0.1 CLI path for agent-produced work.

## Register

```bash
claw agent register --name demo-agent --version "2026-05-11"
claw agent status demo-agent
```

The CLI creates a local Ed25519 key under the user's Claw home and stores public registration metadata in the repository.

## Work

```bash
claw intent create --title "Task" --goal "Outcome"
claw change create --intent <intent-id>
# edit files
claw snapshot --change <change-id> -m "Implement task"
```

## Submit Evidence

```bash
claw ship \
  --intent <intent-id> \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence test=pass:1200 \
  --evidence lint=pass:300
```

Evidence is stored in the capsule public fields. Private capsule metadata is reserved for encrypted fields used by policy-sensitive workflows.

## Daemon API

Agents can use the daemon for sync, events, intents, changes, capsules, and workstreams. Use `claw daemon --help` and require auth/TLS for non-local production binds.

## Rotate or Revoke

There is no standalone `agent keygen`, `agent rotate`, or `agent revoke` command in v0.1. Rotate by registering a replacement agent identity, updating policies to trust the new signer, and auditing capsules signed by the old identity.

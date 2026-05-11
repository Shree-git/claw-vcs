# Agent Capsule

This example shows the expected shape of an agent-authored change.

```bash
claw agent register --name build-agent-01 --version 0.1.0
claw intent create --title "Refactor cache" --goal "Reduce duplicate cache lookups"
claw change create --intent <intent-id>
claw snapshot -m "refactor cache lookup path"
claw ship \
  --intent <intent-id> \
  --agent build-agent-01 \
  --evidence test=pass \
  --evidence lint=pass
claw show <capsule-id>
```

`claw agent register` creates a local signing key for the agent and stores only
the public registration in the repository. The signature makes the claim
attributable. It does not prove that the agent, runner, or tests were
trustworthy; policy decides which keys and evidence are accepted.

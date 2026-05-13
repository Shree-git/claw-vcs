# Agent Capsule

This example shows the expected shape of an agent-authored change. Run the
script to create a temporary repository, register an agent, ship signed evidence,
and inspect the resulting capsule:

```bash
CLAW_BIN=target/debug/claw examples/agent-capsule/scripts/demo.sh
```

`claw agent register` creates a local signing key for the agent and stores only
the public registration in the repository. The signature makes the claim
attributable. It does not prove that the agent, runner, or tests were
trustworthy; policy decides which keys and evidence are accepted.

# Human-Agent Pair Workflow

Use this path when a human owns intent and review while an agent produces a change.

1. Human creates or approves the intent.
2. Agent registers: `claw agent register --name <agent> --version <version>`.
3. Agent creates a change linked to the intent.
4. Agent snapshots on an isolated branch or worktree.
5. Agent ships with evidence and its registered signing key.
6. Human reviews `claw show --json <capsule>` and `claw policy eval <policy> --revision <ref> --json`.
7. Human integrates with `claw integrate --right <agent-ref>`.

Failure modes: missing evidence, unregistered signer, stale revision evidence, or policy denial should stop integration until the human resolves the issue.

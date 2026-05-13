# Agent integrator

Use this path when wiring an automation or AI agent into Claw.

## Start

- Read [Concepts](../concepts/index.md).
- Read [Agent docs](../agents/index.md).
- Register an agent identity before shipping signed work.
- Agree on evidence names with the repository maintainers.
- Decide whether the runner signs `heads/main` or a feature branch, because
  `claw ship` defaults `--revision-ref` to `heads/main`.

## First integration

1. Create a test repository.
2. Register the agent identity.
3. Create an intent and change.
4. Run the agent on a small docs-only change.
5. Ship with `test` or `lint` evidence.
6. Verify policy behavior.

Minimal command path:

```bash
claw agent register --name docs-agent --version "2026-05-11"
claw intent create --title "Agent smoke" --goal "Prove the integration path"
claw change create --intent <intent-id>
claw snapshot --change <change-id> -m "Agent smoke"
claw ship --intent <intent-id> --revision-ref heads/main --agent docs-agent --evidence smoke=pass
```

## Production gate

Before production use, confirm:

- private capsule fields do not contain secrets in plain text
- policy denies missing evidence
- support bundle output is safe to share under your org rules
- rollback behavior is tested
- local agent private keys are provisioned and rotated under your credential
  process
- policy reviewer IDs use stable agent IDs unless you intentionally pin public
  key IDs

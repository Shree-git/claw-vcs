# Persona index

These pages route readers to the right docs for their job.

| Reader | Start here |
|---|---|
| New contributor | [Contributor](contributor.md) |
| Agent integrator | [Agent integrator](agent-integrator.md) |
| Platform operator | [Platform operator](platform-operator.md) |
| Security reviewer | [Security reviewer](security-reviewer.md) |
| Git user | [Git user](git-user.md) |

## Shared docs

- [Quickstart](../getting-started/quickstart.md)
- [Concepts](../concepts/index.md)
- [Workflows](../workflows/index.md)
- [Support](../../SUPPORT.md)

## Choose the right path

- If you are changing Claw itself, start with contributor docs and run the
  relevant Rust or docs checks before opening a PR.
- If you are wiring an agent or CI job, start with agent registration and the
  agent change workflow. Confirm command examples against `claw --help` for the
  pinned Claw version in your runner.
- If you operate a daemon or sync endpoint, start with production install,
  compatibility, security, telemetry, backup, and rollback docs.

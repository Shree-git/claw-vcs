# Workflows

These pages show the normal paths for using Claw. Operator install and incident
steps live under `docs/operations/` and `docs/runbooks/`.

## Pages

- [Daily change workflow](daily-change.md)
- [Policy-gated ship workflow](policy-gated-ship.md)
- [Git interop workflow](git-interop.md)
- [Solo workflow](solo.md)
- [Human-agent pair workflow](human-agent-pair.md)
- [Multiple agents workflow](multiple-agents.md)
- [Policy-gated integration workflow](policy-gated-integration.md)
- [Sensitive paths workflow](sensitive-paths.md)
- [Release workflow](release.md)

## Command map

| Job | Main commands |
|---|---|
| Start repository metadata | `claw init` |
| Create goal | `claw intent create` |
| Start implementation attempt | `claw change create` |
| Record work | `claw snapshot --change <change-id> -m <message>` |
| Register automation signer | `claw agent register --name <agent> --version <version>` |
| Ship with evidence | `claw ship --intent <intent-id> --revision-ref heads/<branch> --evidence name=pass` |
| Merge work | `claw integrate --right heads/<branch>` |
| Create policy object | `claw policy create --id <policy-id> --check <name>` |
| Sync remote refs | `claw sync pull`, `claw sync push` |
| Bridge to Git | `claw git-export`, `claw git-import`, `claw git-roundtrip` |

Policy objects are enforced only when referenced by the intent for the revision
being shipped or integrated. The current CLI creates policies but does not yet
provide an `intent policy add` command.

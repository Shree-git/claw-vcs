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
| Rotate or revoke signer | `claw agent rotate --name <agent>`, `claw agent revoke --name <agent>` |
| Ship with evidence | `claw ship --intent <intent-id> --revision-ref heads/<branch> --evidence name=pass` |
| Merge work | `claw integrate --right heads/<branch>` |
| Create policy object | `claw policy create --id <policy-id> --check <name>` |
| Attach policy to intent | `claw intent policy add <intent-id> <policy-id>` |
| Sync remote refs | `claw sync pull`, `claw sync push` |
| Bridge to Git | `claw git-export`, `claw git-import`, `claw git-roundtrip` |

Policy objects are enforced only when referenced by the intent for the revision
being shipped or integrated. Use `claw intent policy list <intent-id>` to audit
which policies apply to a workflow before shipping or integrating work.

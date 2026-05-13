# `claw intent`

Create and inspect structured goals.

## Examples

```bash
claw intent create --title "Add dark mode" --goal "Support theme toggling"
claw intent list
claw intent show <intent-id>
claw intent update <intent-id> --status done
claw intent policy add <intent-id> ci-required
claw intent policy list <intent-id>
claw intent policy remove <intent-id> ci-required --dry-run
claw intent --json list
```

`claw intent policy add` validates that the policy ref exists before attaching
it. Policy refs can be bare policy IDs such as `ci-required` or full refs such
as `policies/ci-required`.

## JSON Output

Pass `--json` before the subcommand:

```bash
claw intent --json create --title "Add dark mode" --goal "Support theme toggling"
claw intent --json list
claw intent --json show <intent-id>
claw intent --json policy add <intent-id> ci-required --dry-run
```

Create/show/update output includes an `intent` object and its stored object ID
when available. List output is an `intents` array. Policy add/remove dry-runs
include `dry_run`, `changed`, `policy_ref`, and the candidate `intent`.

## Exit Codes

- `0`: command completed.
- `2`: invalid CLI usage.
- `3`: not in a Claw repository.
- `5`: object/ref read or write failure.
- `10`: policy attachment validation failed.

For machine-readable failures, use:

```bash
claw --error-format json intent show <intent-id>
```

## Common Errors

- Unknown intent ID: run `claw intent list` and retry with the displayed ID.
- Unknown policy ref: run `claw policy show <policy-id>` or create the policy first.
- Invalid status: use one of the statuses accepted by the CLI help for `intent update`.
- Policy remove has no effect: the ref was not attached; use `claw intent policy list <intent-id>`.

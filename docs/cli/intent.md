# `claw intent`

Create and inspect structured goals.

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

Common errors include unknown intent ID, unknown policy ref, and invalid status.
Use `claw intent list` to discover IDs and `claw policy show <policy-id>` to
verify a policy ref.

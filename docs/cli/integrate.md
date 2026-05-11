# `claw integrate`

Merge policy-approved changes.

## Examples

```bash
claw integrate --right heads/feature
claw integrate --right heads/feature --dry-run
claw integrate --left heads/main --right heads/feature -m "Integrate feature"
```

Use dry-run before mutating refs or the worktree when reviewing policy decisions or conflict risk. Conflicts use the merge/conflict exit-code family documented in `exit-codes.md`.

## JSON Output

`claw integrate` does not currently emit command-specific success JSON. Use
global JSON errors for automation failures:

```bash
claw --error-format json integrate --right heads/feature --dry-run
```

Inspect the resulting history afterward with:

```bash
claw log --json
claw status --json
```

## Exit Codes

- `0`: integration completed or dry-run succeeded.
- `2`: invalid CLI usage.
- `3`: not in a Claw repository.
- `5`: object/ref/worktree read or write failure.
- `8`: merge conflict blocks integration.
- `10`: policy evaluation denied integration.

## Common Errors

- Missing `--right`: pass the ref being integrated.
- Policy denial: run dry-run, inspect missing evidence, then rerun `claw ship` with required evidence.
- Merge conflicts: resolve conflict files and use `claw resolve`.
- Dirty worktree: run `claw status`, commit/snapshot intended changes, or clean the worktree before integration.

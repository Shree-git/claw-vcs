# `claw integrate`

Merge policy-approved changes.

```bash
claw integrate --right heads/feature
claw integrate --right heads/feature --dry-run
```

Use dry-run before mutating refs or the worktree when reviewing policy decisions or conflict risk. Conflicts use the merge/conflict exit-code family documented in `exit-codes.md`.

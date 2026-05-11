# `claw snapshot`

Record the current working tree atomically.

```bash
claw snapshot -m "initial implementation"
claw snapshot --json -m "initial implementation"
```

There is no staging area. Ignored paths and `.claw/` local state are excluded according to repository ignore rules.

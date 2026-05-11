# `claw init`

Initialize a Claw repository in the current directory.

```bash
claw init
claw init --json
claw init --dry-run
```

JSON output reports whether initialization would create repository state. Common failures use exit code `3` for repository discovery conflicts and `5` for filesystem errors.

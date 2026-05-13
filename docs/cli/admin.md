# `claw admin`

Administrative commands for production operators and release drills.

```bash
claw admin backup create
claw admin backup verify
claw admin rollback plan --backup-id <backup-id>
claw admin rollback execute --backup-id <backup-id>
claw admin migrate plan
claw admin migrate apply --dry-run
```

Use `--dry-run` on migration commands before mutating a repository. Common failures include missing backups, unsupported object format versions, and invalid repository layout.

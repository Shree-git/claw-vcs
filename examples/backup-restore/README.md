# Backup and Restore Demo

This demo creates a local Claw VCS repository, records metadata, creates a
backup, corrupts a metadata ref, and restores from the backup with the rollback
command.

```bash
./scripts/demo.sh
```

Use `CLAW_BIN=/path/to/claw ./scripts/demo.sh` to test a specific binary.

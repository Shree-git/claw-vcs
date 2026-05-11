# `claw git-export`

Export Claw history into Git objects and refs.

```bash
claw git-export --git-dir /tmp/exported.git --all-heads
claw git-export --git-dir /tmp/exported.git --all-heads --dry-run
git -C /tmp/exported.git fsck --strict
git -C /tmp/exported.git log --oneline
```

`--dry-run` validates refs and branch names, reports the planned branch exports, and skips Git object/ref/note writes.

Always verify exported repositories with real Git commands before handing them to users or automation.

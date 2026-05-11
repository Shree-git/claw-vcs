# `claw git-import`

Import Git objects and refs into Claw.

```bash
claw git-import --git-dir /path/to/repo/.git --all-branches
claw git-import --git-dir /path/to/repo/.git --all-branches --dry-run
claw git-roundtrip
```

`--dry-run` validates the Git refs and destination Claw refs, reports the planned imports, and skips Claw object/ref writes.

Review migration docs for unsupported or lossy Git features before using imported history as an authoritative repository.

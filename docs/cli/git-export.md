# `claw git-export`

Export Claw history into Git objects and refs.

## Examples

```bash
claw git-export --git-dir /tmp/exported.git --all-heads
claw git-export --git-dir /tmp/exported.git --all-heads --dry-run
git -C /tmp/exported.git fsck --strict
git -C /tmp/exported.git log --oneline
```

`--dry-run` validates refs and branch names, reports the planned branch exports, and skips Git object/ref/note writes.

Always verify exported repositories with real Git commands before handing them to users or automation.

## JSON Output

`claw git-export` does not currently emit command-specific success JSON. Use
global JSON errors for automation failures:

```bash
claw --error-format json git-export --git-dir /tmp/exported.git --all-heads --dry-run
```

Dry-run output is human-readable and names each planned branch export.

## Exit Codes

- `0`: export or dry-run completed.
- `1`: invalid branch/ref path or other post-parse validation failure.
- `2`: invalid CLI usage.
- `3`: not in a Claw repository.
- `5`: object read or Git object/ref write failure.

## Common Errors

- Invalid Git branch name: adjust `--branch` or `--branch-prefix`.
- Source ref missing: verify with `claw branch list` or `claw log`.
- Git directory missing: create the target directory or pass the correct `--git-dir`.
- Exported repo fails `git fsck`: treat as a bridge bug and do not distribute the export.

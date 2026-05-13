# `claw git-import`

Import Git objects and refs into Claw.

## Examples

```bash
claw git-import --git-dir /path/to/repo/.git --all-branches
claw git-import --git-dir /path/to/repo/.git --all-branches --dry-run
claw git-roundtrip
```

`--dry-run` validates the Git refs and destination Claw refs, reports the planned imports, and skips Claw object/ref writes.

Review migration docs for unsupported or lossy Git features before using imported history as an authoritative repository.

## JSON Output

`claw git-import` does not currently emit command-specific success JSON. Use
global JSON errors for automation failures:

```bash
claw --error-format json git-import --git-dir /path/to/repo/.git --all-branches --dry-run
```

Dry-run output is human-readable and names each planned Git-to-Claw ref import.

## Exit Codes

- `0`: import or dry-run completed.
- `1`: invalid destination ref path or other post-parse validation failure.
- `2`: invalid CLI usage.
- `3`: not in a Claw repository.
- `5`: Git object read or Claw object/ref write failure.

## Common Errors

- Git ref missing: verify with `git -C <repo> show-ref`.
- Unsupported Git object or tree mode: consult [migration docs](../migration/from-git.md).
- Destination ref rejected: use a relative Claw ref such as `heads/main`.
- Imported history does not roundtrip: run `claw git-roundtrip` and keep Git as source of truth until the discrepancy is understood.

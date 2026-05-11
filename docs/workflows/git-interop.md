# Git interop workflow

Git interop is experimental in the `v0.1.x` line. Pin Claw versions for any
automation that depends on import or export behavior.

## Before using the bridge

Start from a repository where both sides are easy to inspect:

```bash
git status --short
claw status
claw log --limit 5
```

Keep Git as the source of truth until your team has validated import, export,
checkout, notes, and rollback behavior with the exact Claw version you plan to
use.

## Export Claw history to Git

```bash
claw git-export --ref heads/main --branch claw/main
```

Export all heads:

```bash
claw git-export --all-heads
```

To write provenance into Git notes when supported by the command path:

```bash
claw git-export --ref heads/main --branch claw/main --git-notes
```

Validate the exported Git repository with Git itself:

```bash
git fsck --strict
git log --oneline refs/heads/claw/main
git cat-file -t refs/heads/claw/main
```

## Import Git history into Claw

```bash
claw git-import --git-ref refs/heads/main --ref heads/imported
```

Import all branches:

```bash
claw git-import --all-branches
```

When reading provenance notes is part of the migration, use the command's notes
option and record the notes ref in the migration log:

```bash
claw git-import --git-ref refs/heads/main --ref heads/imported --read-notes
```

## Verify a round trip

```bash
claw git-roundtrip --ref heads/main
```

Use round-trip checks before adopting Git interop in release automation.

## Known bridge limits

- Git author and committer metadata can be represented, but Claw intent/change
  structure is richer than Git commits.
- Git notes can carry Claw provenance, but consumers must explicitly read and
  verify those notes.
- Git branch protection and hosted CI settings do not become Claw policies.
- Unsupported Git features or lossy conversions should be listed in the release
  notes for the version being evaluated.

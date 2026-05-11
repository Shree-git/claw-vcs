# From Git to Claw

Use this flow to trial Claw beside an existing Git repository.

## Migration posture

Claw `v0.1.x` Git interop is for evaluation and controlled migration trials.
Keep Git as the source of truth until import, export, notes, checkout, and
rollback have been validated for your repository. Do not delete Git remotes or
branch protection during the trial.

## 1. Start from a clean Git checkout

```bash
git status --short
git branch --show-current
git log --oneline -5
```

Resolve or commit unrelated work before creating Claw metadata.

## 2. Initialize Claw

```bash
claw init
claw status
```

## 3. Import Git history

```bash
claw git-import --git-ref refs/heads/main --ref heads/imported
```

For multiple branches:

```bash
claw git-import --all-branches
```

If you rely on Git notes for provenance, include notes in the trial and record
the notes ref in your migration notes:

```bash
claw git-import --git-ref refs/heads/main --ref heads/imported --read-notes
```

## 4. Verify interop

```bash
claw git-roundtrip --ref heads/imported
claw log --ref heads/imported --limit 5
claw checkout imported --dry-run
```

Then export back to Git and validate with Git:

```bash
claw git-export --ref heads/imported --branch claw/imported --git-notes
git fsck --strict
git log --oneline refs/heads/claw/imported -5
```

## 5. Create new Claw-native work in a trial branch

```bash
claw intent create --title "Adopt Claw trial" --goal "Record first Claw-native change"
claw change create --intent <intent-id>
claw snapshot --change <change-id> -m "Start Claw trial"
claw ship --intent <intent-id> --revision-ref heads/main --evidence smoke=pass
```

For a branch trial, create and checkout a Claw branch after the first imported
snapshot, then use that branch in `--revision-ref`:

```bash
claw branch create claw-trial
claw checkout claw-trial
claw ship --intent <intent-id> --revision-ref heads/claw-trial --evidence smoke=pass
```

## 6. Decide adoption criteria

Before moving any team workflow, confirm:

- all expected Git branches imported
- exported Git history passes `git fsck --strict`
- representative checkout tests pass on imported and exported refs
- required provenance notes survive a round trip, if used
- policy-gated changes are blocked when evidence is missing
- `.claw/` backup and restore have been tested
- rollback has an owner and a written deadline

## Rollback

Keep Git as the system of record until the team has run restore and rollback
drills. If the trial stops, preserve `.claw/` for investigation, then keep using
the original Git remote.

If Claw metadata must be removed from a disposable trial checkout, archive or
delete only that checkout. Do not run cleanup commands against a production
checkout until the `.claw/` backup has been verified.

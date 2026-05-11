# Daily change workflow

Use this flow for a normal human or agent-assisted change.

## 0. Confirm repository state

```bash
claw status
claw branch
```

Claw has no staging area. `claw snapshot` captures the full working tree, so
start from a state where unrelated edits are either already captured or moved
out of the worktree.

## 1. Create an intent

```bash
claw intent create \
  --title "Add dark mode" \
  --goal "Support light and dark theme toggling"
```

Record the returned intent ID.

## 2. Create a change

```bash
claw change create --intent <intent-id>
```

Record the returned change ID. Use one change per implementation attempt. If an
intent needs multiple attempts, create multiple changes and pass the right
change ID to `claw snapshot --change`.

## 3. Edit and inspect

Make file changes, then inspect the worktree:

```bash
claw status
claw diff
```

## 4. Snapshot

```bash
claw snapshot \
  --change <change-id> \
  -m "Initial dark mode implementation"
```

Claw snapshots the working tree atomically. There is no staging area.

## 5. Register or confirm the signer

For human-only local trials, the default `claw` agent identity is created on
first ship if needed. For automation, register a stable identity explicitly:

```bash
claw agent register --name release-bot --version "2026-05-11"
claw agent status release-bot
```

Agent private keys are local files under `~/.claw/agent-keys/`; public
registration metadata is stored in the repository under `refs/agents/`.

## 6. Ship

Attach evidence that matches the policy for the repository:

```bash
claw ship \
  --intent <intent-id> \
  --revision-ref heads/main \
  --agent release-bot \
  --evidence test=pass \
  --evidence lint=pass
```

If this is a human local trial and you did not register `release-bot`, omit
`--agent release-bot` and Claw will use the default `claw` signer.
If you snapshot on a feature branch, replace `heads/main` with that branch, for
example `--revision-ref heads/dark-mode`.

If policy denies the ship, fix the missing evidence or policy mismatch and run
the command again.

## 7. Integrate another branch, when needed

`claw ship` creates a capsule and updates the linked intent/change state. It
does not merge a feature branch by itself. To merge a branch into the current
branch:

```bash
claw checkout main
claw integrate --right heads/dark-mode --dry-run
claw integrate --right heads/dark-mode --message "Integrate dark mode"
```

`--right` accepts a Claw ref such as `heads/dark-mode` or a revision ID. If the
merge conflicts, resolve the sidecar files shown by `claw status`, then run
`claw snapshot -m "Resolve dark mode merge"` to complete the merge.

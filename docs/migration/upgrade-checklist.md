# Upgrade and config migration checklist

Use this checklist with [Upgrade and rollback](../operations/upgrade-and-rollback.md).

## Before upgrade

- Record current `claw --version`.
- Save the release artifact or package URL for rollback.
- Create and verify a backup.
- Run `claw admin migrate plan`.
- Read release notes and the compatibility matrix.
- Confirm CLI and daemon target versions match.

## During upgrade

- Stop the daemon.
- Install the new binary.
- Apply config migration only after the plan is reviewed.
- Run `claw admin preflight`.
- Start the daemon with the documented auth profile.

## After upgrade

- Run `claw sync pull` and `claw sync push` smoke checks.
- Run one policy-gated ship in staging if policy changed.
- Verify metrics and logs are present.
- Record the backup ID and migration ledger entry.

## Rollback trigger

Rollback when daemon startup, sync, policy evaluation, or repository integrity
checks fail in a way the team cannot fix inside the maintenance window.

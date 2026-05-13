# Agent Honesty Is Not Enough

A signed claim is not magic. A signed bad test is still bad. A compromised runner can still lie. Claw VCS helps compose trust, but it does not replace key management, runner integrity, policy design, or human judgment.

## What Signatures Give You

- Attribution to a key.
- Tamper detection after signing.
- Offline verification of the signed payload.

## What Signatures Do Not Give You

- Assurance that the signer was honest.
- Assurance that the test command was meaningful.
- Assurance that the runner was clean.
- Assurance that secrets were not exposed before encryption.

## Practical Policy

For sensitive work, require:

- evidence bound to the exact revision ID
- trusted runner keys separate from agent author keys
- freshness windows
- log and artifact digests
- sensitive-path quarantine
- human review for policy or key changes

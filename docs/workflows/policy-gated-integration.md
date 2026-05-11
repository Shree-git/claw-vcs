# Policy-Gated Integration Workflow

This path gates integration on repository policy.

1. Create policy: `claw policy create --id release --check test --check lint`.
2. Attach the policy to the intent: `claw intent policy add <intent-id> release`.
3. Ship the revision with matching evidence: `claw ship --intent <id> --revision-ref <ref> --evidence test=pass --evidence lint=pass`.
4. Preview: `claw policy eval release --revision <ref> --json`.
5. Preview merge: `claw integrate --right <ref> --dry-run`.
6. Integrate only after policy allows the revision.

Common denials include missing checks, untrusted signer IDs, sensitive paths without encrypted private metadata, low trust score, and stale or mismatched evidence.

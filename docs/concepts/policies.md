# Policies

Policies are repository objects that define rules for ship and integration.
They are versioned with the repo, so a clone has the same rules as the source.

## Common rules

Policies can require:

- evidence names such as `test` or `lint`
- reviewer or signer identities
- encrypted private capsule fields for sensitive paths
- quarantine for changes that touch sensitive paths
- minimum trust score derived from capsule evidence

## Example

```bash
claw policy create \
  --id default \
  --visibility public \
  --check test \
  --check lint \
  --reviewer release-bot \
  --min-trust-score 0.8
```

## Operator posture

For production use, keep `fail_closed_integrate` and `fail_closed_ship` enabled
in `.claw/config.toml`. A policy check failure should stop the operation until
the evidence or policy is fixed.

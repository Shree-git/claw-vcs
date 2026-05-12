# `claw doctor`

Inspect CLI, repository, config, daemon, and object-store health.

```bash
claw doctor
claw doctor --json
claw doctor --strict
```

Use `--strict` in automation when errors should fail the command. Warnings are
reported in the summary but do not make `--strict` fail. JSON output includes
check names, status, messages, and remediation hints.

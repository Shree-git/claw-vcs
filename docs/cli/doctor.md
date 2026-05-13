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

When remotes are configured, `doctor` performs a short daemon reachability
probe with the sync `Hello` request. Offline or stale remotes are reported as
warnings so local repository checks remain usable without a running daemon.

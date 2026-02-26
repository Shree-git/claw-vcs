# Runbook: Daemon Start, Stop, and Health

## Purpose

Operate `claw daemon` safely in production.

## Preconditions

- Run from repository root containing `.claw/`.
- Valid auth token profile exists if using `--auth-profile`.
- `claw admin preflight` passes.

## Start

```bash
claw admin preflight
claw daemon --listen 0.0.0.0:50051 --auth-profile default
```

Expected startup output:

- `Claw daemon listening on ...`
- `gRPC auth enabled (bearer token required)` when auth is enabled.

## Stop

- Preferred: use your service manager (`systemctl stop claw`, `kubectl scale`, etc.).
- Manual foreground process: send `SIGTERM` and wait for exit.

## Health checks

### Local process-level

- Process is running and restart count is stable.
- Logs do not show repeated startup failures.

### Protocol-level (client repo)

```bash
claw sync pull --remote origin --ref-name heads/main
```

Healthy outcomes include:

- objects fetched and ref updated, or
- `Remote ref heads/main not found` for empty/new remotes.

Unhealthy outcomes include:

- auth failures (`missing bearer token`, `invalid bearer token`)
- transport/connectivity timeouts

## Failure handling

1. Capture diagnostics:

```bash
claw admin support-bundle
```

2. Check token profile and remote mapping.
3. If release-related, follow [Emergency rollback](emergency-rollback.md).

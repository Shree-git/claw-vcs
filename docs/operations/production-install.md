# Production Install

This guide installs Claw for the supported self-hosted `v0.1.x` production baseline. Use it for controlled rollouts where you own the network boundary, storage, backup policy, and upgrade process.

## Deployment model

- `claw daemon` runs in your environment and serves gRPC.
- Repository data is local under `.claw/` in the repo root.
- Clients connect through `claw sync` using gRPC remotes.
- Auth is bearer-token based (`authorization: Bearer <token>`).

## Host prerequisites

- Claw binary installed and on `PATH`.
- Install from a tagged GitHub release, Homebrew, or the Windows MSI before using this guide.
- Stable storage for repository checkout plus `.claw/`.
- Service account with read/write access to the repository directory.
- Network controls allowing only approved clients to daemon port.

## Install procedure

1. Create or select the repository directory on the server.
2. Initialize if needed:

```bash
claw init
```

3. Configure daemon auth profile token:

```bash
claw auth token set "<strong-random-token>" --profile default
```

4. Run preflight:

```bash
claw admin preflight
```

5. Start daemon:

```bash
claw daemon --listen 0.0.0.0:50051 --auth-profile default
```

## Service manager example (systemd)

```ini
[Unit]
Description=Claw Daemon
After=network.target

[Service]
Type=simple
User=claw
Group=claw
WorkingDirectory=/srv/claw/repo
ExecStart=/usr/local/bin/claw daemon --listen 0.0.0.0:50051 --auth-profile default
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
```

## TLS guidance

- If daemon is reachable beyond localhost, use TLS.
- You can terminate TLS at ingress/proxy, or configure cert/key in `.claw/config.toml`.
- `claw admin preflight` fails when TLS config is incomplete (`tls.cert_path` without `tls.key_path`, or inverse).

## Post-install checks

- `claw admin preflight` returns `PASS`.
- Daemon process remains healthy after restart.
- A client can run:

```bash
claw remote add origin http://claw-daemon.internal:50051 --kind grpc --token-profile default
claw sync pull --remote origin --ref-name heads/main
```

## Related docs

- [Upgrade and rollback](upgrade-and-rollback.md)
- [Troubleshooting](troubleshooting.md)
- [Daemon runbook](../runbooks/daemon-start-stop-health.md)

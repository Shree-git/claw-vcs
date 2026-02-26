# Compatibility Reference

This page defines the current compatibility baseline for production operators.

## Versioning posture

- Current project status is early (`v0.1.x` line).
- Treat cross-version interoperability as explicit test-required, not implied.
- Use pinned client and daemon versions per environment.

## Cross-version posture (N and N-1)

- **N (current release line)**: Full support baseline is same-version CLI and daemon.
- **N-1 (previous release line)**: Not a default compatibility guarantee; use only as a controlled upgrade window with explicit validation in your environment.
- `--compat-check` classifies compatibility by major/minor version:
  - Same major + same minor: full support.
  - Same major + one minor step difference (N/N-1 or N/N+1): limited support for controlled rollout windows.
  - Anything else: unsupported.

## Wire/service compatibility

- Daemon sync `Hello` reports `server_version: 0.1.0`.
- Supported sync capability currently includes `partial-clone`.
- Primary transport is gRPC (`claw daemon` + `claw sync`).
- ClawLab HTTP transport is available for hosted remote integration.
- Daemon HTTP schema artifact: `docs/reference/daemon-http-openapi-v1.json`.
- Current daemon health listener HTTP surfaces include `/v1/health/live`, `/v1/health/ready`, `/v1/health/deps`, and `/v1/metrics` (Prometheus text payload).

## Config compatibility

- Supported repository config schema: `config_version = 1` (`.claw/config.toml`).
- Unknown config versions fail to load.
- Use `claw admin migrate plan` and `claw admin migrate apply` during upgrades.

## Platform compatibility

- Prebuilt binaries are published for macOS, Linux, and Windows.
- Self-hosted daemon baseline assumes Linux service operation, but protocol behavior is platform-independent.

## Self-hosted topology compatibility

- Supported deployment tiers and support boundaries are defined in [Self-Hosted Topology Tiers and Support Boundary](topology-tiers.md).

## Operator guidance

- Keep client and daemon on the same release whenever possible.
- Use one-minor-step combinations only during planned rollouts and verify with smoke sync checks before broad deployment.
- Run smoke sync checks after every upgrade.
- Keep one known-good previous release artifact for rollback.

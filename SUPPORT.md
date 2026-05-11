# Support

Claw is pre-1.0 software. The supported path is controlled self-hosted use with
pinned client and daemon versions, backups, and rollback practice.

## Get help

- Bugs: open a GitHub issue with the bug report form.
- Usage questions: open a GitHub discussion or issue tagged `question`.
- Docs gaps: open a docs issue or send a docs PR.
- Demo problems: include the output from `examples/basic-demo/scripts/demo.sh`
  and the value of `CLAW_BIN`.
- Security reports: use GitHub Security Advisories. Do not open a public issue
  for a vulnerability.

Security policy:

- [SECURITY.md](SECURITY.md)

## What to include

For bugs and operator incidents, include:

- `claw --version`
- operating system and install channel
- exact command and full error text
- whether the command ran against CLI only, daemon, sync, Git interop, or policy
- relevant output from `claw admin preflight`
- support bundle path or contents if safe to share

Generate a support bundle from the repository root:

```bash
claw admin support-bundle
```

The bundle is written under `.claw/support/`.

For demo-only issues, run:

```bash
bash -n examples/basic-demo/scripts/demo.sh
CLAW_BIN=/path/to/claw examples/basic-demo/scripts/demo.sh
```

The demo sets `HOME` to a temporary directory so it does not use your real
agent keys.

## Support boundary

The public support boundary for `v0.1.x` is:

- self-hosted daemon deployments
- same-version CLI and daemon pairings
- documented upgrade and rollback flow
- gRPC sync and the documented daemon HTTP health/metrics surface
- policy schema, backup, preflight, and support bundle commands

Experimental surfaces, including Git interop details, can change between minor
releases. Pin versions for automation that depends on them.

Out of scope for public support in `v0.1.x`:

- unpinned mixed-version client/daemon fleets
- hosted ClawLab assumptions not named in release notes
- production use as the only source of truth without Git or another rollback
  path
- manual edits to `.claw/objects`, `.claw/refs`, or `.claw/reflogs`

## Response expectations

Claw VCS is experimental and does not have staffed support coverage.

- Security reports: best-effort acknowledgement within 7 days.
- Reproducible crash, data integrity, or release verification bugs: triage as soon as maintainer time allows.
- General usage questions and feature requests: no guaranteed response time.
- Public holidays, travel, and launch work can delay responses.

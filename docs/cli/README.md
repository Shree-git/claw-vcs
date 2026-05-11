# Claw CLI Hardening Notes

This page documents launch-facing CLI affordances for automation, onboarding, and support triage.

## Version Metadata

Use `claw version` for human output:

```console
$ claw version
claw 0.1.0
```

Use `claw version --json` for scripts. The JSON shape includes `name`, `version`, `package`, optional `git_sha`, `object_format_version`, `sync_protocol_version`, `sync_capabilities`, `build`, `os`, and `arch`.

## Command Pages

- [`admin`](admin.md)
- [`agent`](agent.md)
- [`auth`](auth.md)
- [`branch`](branch.md)
- [`checkout`](checkout.md)
- [`completions`](completions.md)
- [`daemon` / `serve`](daemon.md)
- [`diff`](diff.md)
- [`doctor`](doctor.md)
- [`init`](init.md)
- [`intent`](intent.md)
- [`change`](change.md)
- [`patch`](patch.md)
- [`plugin`](plugin.md)
- [`snapshot`](snapshot.md)
- [`ship`](ship.md)
- [`integrate`](integrate.md)
- [`policy`](policy.md)
- [`remote`](remote.md)
- [`resolve`](resolve.md)
- [`sync`](sync.md)
- [`git-export`](git-export.md)
- [`git-import`](git-import.md)
- [`git-roundtrip`](git-roundtrip.md)
- [`log`](log.md)
- [`status`](status.md)
- [`show`](show.md)
- [`version`](version.md)

## Shell Completions

Generate shell completion scripts with:

```console
claw completions bash
claw completions zsh
claw completions fish
claw completions powershell
claw completions elvish
```

`claw completion <shell>` is accepted as an alias.

## Doctor

Run `claw doctor` to inspect local CLI and repository health. It checks the binary version, Git availability, object format support, current directory, repository discovery, `.claw` layout, config loading, HEAD state, ref target validity, remote config parsing, daemon auth/TLS readiness, and basic write permissions.

Use `claw doctor --json` for a structured report. Use `claw doctor --strict` when automation should fail if any check reports an error.

## JSON Output

The following workflow commands support structured output:

```console
claw init --json
claw status --json
claw log --json
claw diff --json
claw branch --json
claw checkout --json <target>
claw snapshot --json -m "message"
claw intent --json list
claw intent --json show <intent-id>
claw change --json list
claw remote --json list
claw show --json <object-or-ref>
claw policy eval <policy-id> --revision <revision> --json
```

Global runtime errors can be emitted as a JSON envelope with:

```console
claw --error-format json <command>
```

The envelope includes `code`, `message`, `request_id`, `remediation`, `exit_code`, and `details`.

## Dry Runs

Dry-run support is available where the command can preview intent without committing its primary mutation:

```console
claw init --dry-run
claw branch create <name> --dry-run
claw branch delete <name> --dry-run
claw checkout <target> --dry-run
claw remote add <name> <url> --dry-run
claw remote remove <name> --dry-run
claw integrate --right <ref> --dry-run
claw policy apply --id <policy-id> --dry-run
claw sync push --remote origin --ref-name heads/main --dry-run
claw git-export --git-dir /tmp/exported.git --dry-run
claw git-import --git-dir /path/to/repo/.git --dry-run
```

Commands that expose both flags, such as `claw policy apply`, can combine
dry-run preview with `--json`. Other dry-run commands emit human-readable
preview text until their command-specific JSON output is implemented.

## Onboarding After Init

After `claw init`, the CLI prints the next local workflow commands:

```console
claw status
claw snapshot -m "initial snapshot"
claw intent new --title "describe the next change"
```

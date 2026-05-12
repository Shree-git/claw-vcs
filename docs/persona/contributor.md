# Contributor

Use this path when you are opening a Claw issue or PR.

## Start

- Read [CONTRIBUTING.md](../../CONTRIBUTING.md).
- Pick the GitHub issue form that matches the work.
- Keep PRs scoped to one behavior or docs change.

## Before a PR

Run the checks that match your change:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For docs-only changes, proofread links and command examples.

Docs/demo/community-only changes should also run:

```bash
bash -n examples/basic-demo/scripts/demo.sh
CLAW_BIN=target/debug/claw examples/basic-demo/scripts/demo.sh
```

Build `target/debug/claw` first with `cargo build -p claw-vcs` if it does not exist.
The demo writes to a temporary workspace and sets `HOME` to a separate temporary
directory so agent keys do not touch the contributor's real `~/.claw/` or the
demo worktree.

## Review notes

Call out any public interface change, migration need, or deprecation policy
impact in the PR body.

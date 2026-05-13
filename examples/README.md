# Claw VCS Examples

These examples are small, runnable workflows for launch evaluation and docs
screenshots. Most scripts accept `CLAW_BIN=/path/to/claw`; otherwise they use
`claw` from `PATH`.

| Example | Purpose |
|---|---|
| [`basic-demo`](basic-demo/) | End-to-end intent, change, snapshot, policy, ship, integrate, log, and show flow. |
| [`basic-human-workflow`](basic-human-workflow/) | Solo developer flow for local snapshots and history inspection. |
| [`agent-capsule`](agent-capsule/) | Agent key registration, capsule creation, evidence, and verification. |
| [`policy-gated-integration`](policy-gated-integration/) | Policy-denial cases for missing evidence, stale evidence, sensitive paths, and trust failures. |
| [`git-roundtrip`](git-roundtrip/) | Git import/export and roundtrip verification workflow. |
| [`sensitive-path`](sensitive-path/) | Sensitive-path policy handling and encrypted private field expectations. |
| [`backup-restore`](backup-restore/) | Admin backup, verification, restore, and recovery drill. |
| [`demo-media`](demo-media/) | Terminal cast and SVG command-gallery assets used by docs. |
| [`integrations`](integrations/) | CI integration sketches for GitHub Actions, GitLab CI, and Jenkins. |

Run the main demo after building the CLI:

```bash
cargo build -p claw-vcs --locked
CLAW_BIN=target/debug/claw examples/basic-demo/scripts/demo.sh
```

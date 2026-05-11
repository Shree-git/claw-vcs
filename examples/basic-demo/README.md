# Basic Demo

Run:

```bash
./scripts/demo.sh
```

Or point at a local development binary:

```bash
cargo build -p claw
CLAW_BIN="$(pwd)/target/debug/claw" ./examples/basic-demo/scripts/demo.sh
```

The script initializes a temporary Claw repository, sets `HOME` to a separate
temporary directory so agent keys stay out of the demo worktree, creates an
intent and change, records a branch snapshot, creates a policy object,
registers a demo agent, ships a capsule with smoke/lint evidence, integrates
the branch back to `main`, and prints log/status/show output.

The policy object is created to make the primitive visible. The current CLI does
not yet expose a command to attach policy refs to intents, so the demo does not
claim a fail-closed policy gate.

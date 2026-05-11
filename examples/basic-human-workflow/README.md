# Basic Human Workflow

```bash
claw init
claw intent create --title "Fix parser edge case" --goal "Handle empty input without panic"
claw change create --intent <intent-id>
$EDITOR src/parser.rs
cargo test -p parser
claw snapshot -m "handle empty parser input"
claw ship --intent <intent-id> --evidence test=pass
claw integrate --dry-run
claw integrate
```

Use this flow when a human author wants structured intent and policy-gated integration without an autonomous agent.

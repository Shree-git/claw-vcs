# Basic Human Workflow

```bash
claw init
printf 'initial\n' > README.md
claw snapshot -m "initial"
claw intent create --title "Fix parser edge case" --goal "Handle empty input without panic"
claw change create --intent <intent-id>
claw branch create parser-fix
claw checkout parser-fix
$EDITOR src/parser.rs
cargo test -p parser
claw snapshot -m "handle empty parser input"
claw ship --intent <intent-id> --revision-ref heads/parser-fix --evidence test=pass
claw checkout main
claw integrate --right heads/parser-fix --dry-run
claw integrate --right heads/parser-fix
```

Use this flow when a human author wants structured intent and policy-gated integration without an autonomous agent.

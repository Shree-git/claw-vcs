# Solo Workflow

Use this path when one developer is evaluating Claw locally.

1. Initialize metadata: `claw init`.
2. Create intent: `claw intent create --title "..." --goal "..."`.
3. Create change: `claw change create --intent <intent-id>`.
4. Edit files.
5. Snapshot: `claw snapshot --change <change-id> -m "..."`.
6. Ship: `claw ship --intent <intent-id> --revision-ref heads/main --evidence test=pass`.
7. Inspect: `claw log`, `claw show --json heads/main`, `claw status`.

Keep Git as the fallback source of truth for v0.1 experiments.

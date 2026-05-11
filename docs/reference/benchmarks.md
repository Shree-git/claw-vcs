# Benchmarks

Criterion benchmark scaffolds live in:

- `crates/claw-core/benches/cof.rs`
- `crates/claw-store/benches/snapshot_store.rs`
- `crates/claw-patch/benches/codecs.rs`
- `crates/claw-merge/benches/merge.rs`
- `crates/claw-sync/benches/protocol.rs`
- `crates/claw-git/benches/import_export.rs`

Run:

```bash
cargo bench -p claw-core
cargo bench -p claw-store
cargo bench -p claw-patch
cargo bench -p claw-merge
cargo bench -p claw-sync
cargo bench -p claw-git
```

Current coverage:

- COF encode/decode throughput
- BLAKE3 object hash throughput
- snapshot-shaped store writes for blobs, trees, revisions, and snapshot objects
- object load throughput for snapshot-shaped repositories
- `.claw` repository size scan cost
- text diff throughput
- JSON tree diff throughput
- binary replacement diff throughput
- non-overlapping text revision merge throughput
- sync capability negotiation
- dependency-first reachable-object traversal
- want/have set computation
- Git export from a Claw revision DAG
- Git import into a Claw store
- Claw-versus-Git repository size scan cost

Future benchmark targets:

- end-to-end CLI snapshot speed over real worktrees
- networked sync throughput with object streaming
- Git round-trip benchmarks driven by real `git` commands
- large repository path filtering

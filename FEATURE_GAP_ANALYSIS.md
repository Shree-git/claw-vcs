# Claw Feature Gap Analysis

Based on a thorough review of every crate, CLI command, test, and CI workflow in the repository.

---

## Part 1: Missing Features Users Would Reasonably Expect

### Tier 1 — High Impact

| # | Gap | What's Missing | Why Users Expect It | Complexity |
|---|-----|---------------|---------------------|------------|
| 1 | **No `claw stash` or undo mechanism** | There is no way to temporarily shelve uncommitted work or undo a snapshot. Git users rely on `stash` and `reset` constantly. The reflog exists but is not exposed via CLI. | Any user switching branches mid-work will lose changes or be blocked by the "uncommitted changes" guard in `checkout`. | Medium |
| 2 | **No `claw tag` command** | No way to create named, immutable markers for releases. Tags are fundamental VCS primitive and the README mentions releases but provides no tagging mechanism. | Users shipping software need release markers (v1.0.0, etc.). | Low |
| 3 | **No `claw reflog` command** | Reflog infrastructure exists (`claw-store/src/reflog.rs` — append/read implemented with tests), but there is no CLI command to view it. `claw show` can display RefLog objects, but there's no `claw reflog` subcommand. | Safety net for recovering from mistakes — critical for adoption confidence. | Low |
| 4 | **Policy `sensitive_paths` not enforced** | Policies store `sensitive_paths`, `quarantine_lane`, and `min_trust_score` fields, but `claw-policy/src/evaluator.rs` only checks visibility and required_checks. These three fields are write-only decorations. | The README advertises sensitive-path gating. Users creating policies with `--sensitive-paths` expect them to be enforced during integration. | Medium |
| 5 | **No `claw workstream` CLI command** | Workstream objects exist in the type system and are served via gRPC (`WorkstreamServiceServer`), but there is no CLI command to create, list, or manage workstreams. Changes have a `workstream_id` field that is always `None`. | The README lists Workstream as a first-class object type. Users reading the docs expect to use it. | Medium |
| 6 | **No CI/test workflow in GitHub Actions** | `.github/workflows/release.yml` handles release builds only. There is no workflow that runs `cargo test`, `cargo clippy`, or `cargo fmt --check` on PRs or pushes. | Contributors expect CI to validate their changes. The project has 10+ integration tests and 27 unit test modules with no automated gating. | Low |
| 7 | **No `claw clone` top-level command** | Clone is buried as `claw sync clone`. Every VCS user expects `claw clone <url>` at the top level. | Discoverability — the first command a new user runs after install. | Low |
| 8 | **`claw diff` has no color output** | `diff_render.rs` emits plain unified diff. No ANSI color support for terminals. | Every modern diff tool colorizes additions/deletions. Without it, diffs are hard to scan. | Low |

### Tier 2 — Medium Impact

| # | Gap | What's Missing | Why Users Expect It | Complexity |
|---|-----|---------------|---------------------|------------|
| 9 | **No garbage collection / prune** | Objects are stored forever. No `claw gc` to remove unreachable objects, compact loose objects into packs, or reclaim disk space. | Repositories grow without bound. Pack files exist (`claw-store/src/pack.rs`) but are only used for sync transport, not local compaction. | High |
| 10 | **No `claw blame` / annotate** | No way to trace line-by-line authorship. Particularly important for Claw's thesis (who wrote this — human or agent?). | Claw's core value proposition is provenance. Not being able to answer "who wrote this line?" undermines the pitch. | High |
| 11 | **No interactive conflict resolution guidance** | `claw resolve` detects conflict markers but provides no interactive merge tool integration or 3-way diff visualization. | Users encountering merge conflicts need more than "edit the file manually." | Medium |
| 12 | **Pack file delta compression missing** | `pack.rs` header comment: "MVP - no delta compression." Every object is stored full-size in packs. | Network transfer and storage efficiency will be poor at scale compared to Git's delta chains. | High |
| 13 | **No `claw config` command** | `RepoConfig` (`claw-store/src/repo.rs`) has only `version` and `name`. No CLI to set/get configuration. No user-level config (~/.clawconfig). | Users expect to configure author name, default remote, editor, diff tool, etc. | Medium |
| 14 | **No file-level add/remove tracking** | `claw snapshot` captures everything atomically (by design), but there's no `claw add` or `claw rm` for selective tracking. While intentional, users migrating from Git will look for this. | README acknowledges "no staging area — by design" but offers no selective alternative (e.g., `claw snapshot --paths`). | Medium |
| 15 | **Restricted visibility is identical to Private** | `claw-policy/src/visibility.rs:16-21` — Restricted just checks for encrypted_private, same as Private. The comment says "In MVP." | If the options exist, users expect them to behave differently. | Medium |
| 16 | **No `claw cherry-pick` or patch extraction** | Can't apply individual revisions from one branch to another without full merge. | Common workflow for hotfixes and selective backporting. | High |

### Tier 3 — Nice to Have

| # | Gap | What's Missing | Why Users Expect It | Complexity |
|---|-----|---------------|---------------------|------------|
| 17 | **No TOML/YAML codec** | Registry maps `.toml` and `.yaml` to the text/line codec. The README mentions "The architecture supports adding codecs for YAML, TOML" but none exist. | Structural merges for config files would demonstrate the codec advantage over Git. | Medium |
| 18 | **Event service uses polling, not filesystem watches** | `event_service.rs:37-39` comment: "A production implementation would use filesystem watches." Currently polls every 2 seconds. | Agent integrations expecting real-time events will experience latency and unnecessary CPU usage. | Medium |
| 19 | **No hook system** | Git has pre-commit, post-commit, pre-push hooks. Claw has no equivalent. | Developers expect to run linters, formatters, and custom scripts at lifecycle points. | Medium |
| 20 | **No `claw search` / intent query** | Can list intents but can't search by goal text, status filter, or date range from CLI. | With structured intents, users expect structured queries. | Low |

---

## Part 2: Developer Experience Improvements

### Tier 1 — High Impact

| # | Gap | Current State | Improvement | Complexity |
|---|-----|--------------|-------------|------------|
| 1 | **No CI on PRs** | Only `release.yml` exists. | Add `ci.yml` with `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt -- --check`. | Low |
| 2 | **Sparse integration tests** | One test file (`tests/integration/spec_tests.rs`) with 10 tests. No CLI-level end-to-end tests. | Add CLI integration tests that invoke the `claw` binary and verify stdout/stderr/exit codes for common workflows. | Medium |
| 3 | **No error context in anyhow chains** | Commands use `anyhow::Result` but many call sites don't add `.context()`. Errors like "No such file" give no indication of which file or which operation. | Add `.context("loading HEAD revision")` style annotations at key points. | Low |
| 4 | **`--json` output inconsistent** | `claw log` and `claw status` support `--json`. Other commands (intent list, change list, branch list, show, policy list) don't. | Add `--json` to all listing and inspection commands for scriptability. | Medium |
| 5 | **No shell completions** | Clap supports generating shell completions (`clap_complete`). Not wired up. | Add `claw completions <shell>` command generating bash/zsh/fish completions. | Low |

### Tier 2 — Medium Impact

| # | Gap | Current State | Improvement | Complexity |
|---|-----|--------------|-------------|------------|
| 6 | **No progress indicators for sync** | `claw sync push/pull/clone` print final result but nothing during transfer. | Add object count / bytes transferred progress bars for large repos. | Medium |
| 7 | **`claw init` doesn't create `.clawignore`** | Users must know to create it manually. | Generate a default `.clawignore` (like `.gitignore`) with common patterns (target/, node_modules/, .env, etc.). | Low |
| 8 | **No `--verbose` / `--quiet` global flags** | Tracing subscriber is set up but not controllable from CLI. | Add `-v`/`-q` global flags mapped to tracing levels. | Low |
| 9 | **No `claw help <concept>` for learning** | `--help` is clap-generated and terse. | Add `claw help intents`, `claw help capsules` etc. for conceptual documentation. | Low |
| 10 | **`.clawignore` doesn't support negation** | Glob patterns only. No `!pattern` for re-inclusion. No nested `.clawignore` files in subdirectories. | Support negation patterns and recursive ignore files like `.gitignore`. | Medium |
| 11 | **No property-based tests** | `proptest` is a workspace dependency but not used anywhere. | Add proptest-based fuzzing for COF encode/decode round-trips, patch apply/invert symmetry, and commute correctness. | Medium |
| 12 | **Daemon has no graceful shutdown** | `claw daemon` runs forever with no signal handling. | Handle SIGTERM/SIGINT for clean shutdown, flush pending writes. | Low |
| 13 | **No benchmarks** | No `benches/` directory. No performance baselines for core operations (hashing, COF encode/decode, tree diff, patch apply). | Add criterion benchmarks for hot paths. | Medium |

### Tier 3 — Nice to Have

| # | Gap | Current State | Improvement | Complexity |
|---|-----|--------------|-------------|------------|
| 14 | **Pack read is not mmap'd** | `read_object_from_pack` calls `std::fs::read` on the entire pack file every time. | Use memory-mapped I/O for pack reads to avoid repeated full-file loads. | Medium |
| 15 | **No LSP / editor integration** | No VS Code extension, no tree-sitter grammar, no semantic tokens. | Provide at minimum a VS Code extension for intent/change/status sidebar. | High |
| 16 | **Daemon stdio mode only supports `hello` and `refs`** | JSON-RPC methods are minimal. | Expand stdio protocol to cover intent CRUD, snapshot, and diff operations for embedded agent use. | Medium |
| 17 | **No man pages** | Only `--help` output. No `man claw` or `man claw-intent`. | Generate man pages from clap definitions during build. | Low |

---

## Summary: Top 10 by Impact-to-Complexity Ratio

Ranked by (user impact / implementation complexity), where items that deliver the most value for the least effort come first:

| Rank | Item | Category | Impact | Complexity |
|------|------|----------|--------|------------|
| 1 | Add CI workflow (test + clippy + fmt) | DX | High | Low |
| 2 | Add `claw reflog` CLI command | Feature | High | Low |
| 3 | Add `claw tag` command | Feature | High | Low |
| 4 | Add `claw clone` as top-level alias | Feature | High | Low |
| 5 | Add shell completions (`claw completions`) | DX | High | Low |
| 6 | Colorized diff output | DX | Medium | Low |
| 7 | Enforce `sensitive_paths` in policy evaluator | Feature | High | Medium |
| 8 | Add `--json` output to all list/show commands | DX | High | Medium |
| 9 | Add `claw stash` (save/pop/list) | Feature | High | Medium |
| 10 | Add `claw workstream` CLI commands | Feature | Medium | Medium |

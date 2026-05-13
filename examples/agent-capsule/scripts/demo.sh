#!/usr/bin/env bash
set -euo pipefail

CLAW_BIN="${CLAW_BIN:-claw}"
ROOT="${1:-$(mktemp -d "${TMPDIR:-/tmp}/claw-agent-capsule.XXXXXX")}"

if ! resolved_claw_bin="$(command -v "$CLAW_BIN")"; then
  printf 'error: claw binary not found: %s\n' "$CLAW_BIN" >&2
  printf 'set CLAW_BIN=/path/to/claw or put claw on PATH\n' >&2
  exit 127
fi
if [[ "$resolved_claw_bin" == */* ]]; then
  resolved_dir="$(cd "$(dirname "$resolved_claw_bin")" && pwd)"
  CLAW_BIN="$resolved_dir/$(basename "$resolved_claw_bin")"
else
  CLAW_BIN="$resolved_claw_bin"
fi

mkdir -p "$ROOT"
cd "$ROOT"

DEMO_HOME="${CLAW_AGENT_DEMO_HOME:-$(mktemp -d "${TMPDIR:-/tmp}/claw-agent-home.XXXXXX")}"
export HOME="$DEMO_HOME"
mkdir -p "$HOME"

finish() {
  status=$?
  if [[ $status -ne 0 ]]; then
    printf '\nAgent capsule demo failed. Workspace preserved at: %s\n' "$ROOT" >&2
  fi
}
trap finish EXIT

run() {
  printf '\n$'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

capture() {
  printf '\n$'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

extract_created_id() {
  awk -v prefix="$1" 'index($0, prefix ":") == 1 {print $3; exit}'
}

run "$CLAW_BIN" init

printf '# Cache refactor\n' > README.md
run "$CLAW_BIN" snapshot -m "Initial repository"

intent_output="$(capture "$CLAW_BIN" intent create \
  --title "Refactor cache" \
  --goal "Reduce duplicate cache lookups")"
printf '%s\n' "$intent_output"
intent_id="$(printf '%s\n' "$intent_output" | extract_created_id "Created intent")"
if [[ -z "$intent_id" ]]; then
  printf 'error: could not parse intent id\n' >&2
  exit 1
fi

change_output="$(capture "$CLAW_BIN" change create --intent "$intent_id")"
printf '%s\n' "$change_output"
change_id="$(printf '%s\n' "$change_output" | extract_created_id "Created change")"
if [[ -z "$change_id" ]]; then
  printf 'error: could not parse change id\n' >&2
  exit 1
fi

mkdir -p src
printf 'cache_lookup = "deduplicated"\n' > src/cache.conf
run "$CLAW_BIN" snapshot --change "$change_id" -m "Refactor cache lookup path"

run "$CLAW_BIN" agent register --name build-agent-01 --version agent-capsule-demo
run "$CLAW_BIN" agent status build-agent-01

ship_output="$(capture "$CLAW_BIN" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent build-agent-01 \
  --evidence test=pass:18 \
  --evidence lint=pass:6)"
printf '%s\n' "$ship_output"
capsule_id="$(printf '%s\n' "$ship_output" | awk '/Capsule:/ {print $2; exit}')"
if [[ -z "${capsule_id:-}" ]]; then
  printf 'error: could not parse capsule id\n' >&2
  exit 1
fi

run "$CLAW_BIN" show "$capsule_id"
run "$CLAW_BIN" log --all --limit 3

printf '\nAgent capsule demo workspace: %s\n' "$ROOT"

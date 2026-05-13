#!/usr/bin/env bash
set -euo pipefail

CLAW_BIN="${CLAW_BIN:-claw}"
ROOT="${1:-$(mktemp -d "${TMPDIR:-/tmp}/claw-policy-failures.XXXXXX")}"

if ! resolved_claw_bin="$(command -v "$CLAW_BIN")"; then
  printf 'error: claw binary not found: %s\n' "$CLAW_BIN" >&2
  exit 127
fi
if [[ "$resolved_claw_bin" == */* ]]; then
  resolved_dir="$(cd "$(dirname "$resolved_claw_bin")" && pwd)"
  resolved_claw_bin="$resolved_dir/$(basename "$resolved_claw_bin")"
fi

mkdir -p "$ROOT"
cd "$ROOT"

DEMO_HOME="${CLAW_DEMO_HOME:-$(mktemp -d "${TMPDIR:-/tmp}/claw-policy-home.XXXXXX")}"
export HOME="$DEMO_HOME"
mkdir -p "$HOME"

run() {
  printf '\n$'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

capture() {
  "$@"
}

expect_denied() {
  local label="$1"
  shift
  printf '\n# Expected denial: %s\n' "$label"
  printf '$'
  printf ' %q' "$@"
  printf '\n'
  set +e
  output="$("$@" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  if [[ $status -eq 0 ]]; then
    printf 'error: policy unexpectedly allowed %s\n' "$label" >&2
    exit 1
  fi
}

extract_created_id() {
  awk -v prefix="$1" 'index($0, prefix ":") == 1 {print $3; exit}'
}

current_revision_hex() {
  "$resolved_claw_bin" show --json heads/main | python3 -c 'import json, sys; print(json.load(sys.stdin)["object"]["hex"])'
}

run "$resolved_claw_bin" init
printf 'hello\n' > README.md
run "$resolved_claw_bin" snapshot -m "initial"

intent_output="$(capture "$resolved_claw_bin" intent create \
  --title "Policy failure examples" \
  --goal "Demonstrate denied policy decisions")"
printf '%s\n' "$intent_output"
intent_id="$(printf '%s\n' "$intent_output" | extract_created_id "Created intent")"

change_output="$(capture "$resolved_claw_bin" change create --intent "$intent_id")"
printf '%s\n' "$change_output"
change_id="$(printf '%s\n' "$change_output" | extract_created_id "Created change")"

mkdir -p secrets
printf 'feature=true\n' > app.conf
printf 'token=demo\n' > secrets/example.txt
run "$resolved_claw_bin" snapshot --change "$change_id" -m "touch app and sensitive path"
run "$resolved_claw_bin" agent register --name demo-agent --version policy-example

run "$resolved_claw_bin" policy create --id release \
  --check test \
  --check lint \
  --reviewer trusted-reviewer \
  --sensitive-path secrets/ \
  --min-trust-score 0.85

run "$resolved_claw_bin" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence lint=pass:8

expect_denied "missing test evidence" \
  "$resolved_claw_bin" policy eval release --revision heads/main --path app.conf --json

run "$resolved_claw_bin" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence test=pass:10

expect_denied "missing lint evidence" \
  "$resolved_claw_bin" policy eval release --revision heads/main --path app.conf --json

run "$resolved_claw_bin" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence test=pass:10 \
  --evidence lint=pass:8

expect_denied "missing trusted reviewer signature" \
  "$resolved_claw_bin" policy eval release --revision heads/main --signer-agent demo-agent --path app.conf --json

expect_denied "trust score below threshold" \
  "$resolved_claw_bin" policy eval release --revision heads/main --signer-agent trusted-reviewer --trust-score 0.50 --path app.conf --json

expect_denied "sensitive path lacks encrypted private fields" \
  "$resolved_claw_bin" policy eval release --revision heads/main --signer-agent trusted-reviewer --trust-score 0.95 --path secrets/example.txt --json

run "$resolved_claw_bin" policy create --id fresh-release \
  --check test \
  --require-fresh-evidence \
  --trusted-runner github-actions/release

expect_denied "freshness metadata missing from evidence" \
  "$resolved_claw_bin" policy eval fresh-release --revision heads/main --json

fresh_revision_hex="$(current_revision_hex)"
run "$resolved_claw_bin" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence test=pass:5 \
  --evidence-command "cargo test" \
  --runner github-actions/release \
  --environment-digest sha256:demo-env \
  --log-digest sha256:demo-log \
  --evidence-expires-in-ms 60000

printf 'feature=false\n' > app.conf
run "$resolved_claw_bin" snapshot --change "$change_id" -m "new revision after evidence"

expect_denied "evidence references a different revision" \
  "$resolved_claw_bin" policy eval fresh-release \
    --revision heads/main \
    --capsule "capsules/by-revision/$fresh_revision_hex" \
    --json

run "$resolved_claw_bin" ship \
  --intent "$intent_id" \
  --revision-ref heads/main \
  --agent demo-agent \
  --evidence test=pass:5 \
  --evidence-command "cargo test" \
  --runner github-actions/release \
  --environment-digest sha256:demo-env \
  --log-digest sha256:demo-log \
  --evidence-expires-in-ms 1
sleep 1

expect_denied "evidence is stale under the freshness window" \
  "$resolved_claw_bin" policy eval fresh-release --revision heads/main --json

printf '\nPolicy failure workspace: %s\n' "$ROOT"

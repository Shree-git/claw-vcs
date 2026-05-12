#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/verify-generated-protos.sh

Regenerates the tracked claw-core protobuf Rust output and fails if the checked
in files under crates/claw-core/src/generated drift from the current proto
sources and prost generator.
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"
generated_path="crates/claw-core/src/generated"

if ! command -v cargo >/dev/null 2>&1; then
  echo "missing required command: cargo" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "missing required command: git" >&2
  exit 1
fi

cargo build -p claw-vcs-core --locked

if git -C "$repo_root" diff --quiet -- "$generated_path"; then
  echo "PASS: generated protobuf output is current"
  exit 0
fi

echo "FAIL: generated protobuf output is stale" >&2
git -C "$repo_root" diff --name-only -- "$generated_path" >&2
echo "Run scripts/verify-generated-protos.sh and commit the generated diff." >&2
exit 1

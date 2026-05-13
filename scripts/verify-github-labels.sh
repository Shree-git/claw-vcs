#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/verify-github-labels.sh

Verifies that every label declared in .github/labels.yml exists on the GitHub
repository with the expected color and description. Extra GitHub default labels
are allowed.

Environment:
  CLAW_LABEL_REPO      GitHub repository to inspect (default: Shree-git/claw-vcs)
  CLAW_LABEL_MANIFEST  Label manifest path (default: .github/labels.yml)
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(cd "$script_dir/.." && pwd -P)"

repo="${CLAW_LABEL_REPO:-Shree-git/claw-vcs}"
manifest="${CLAW_LABEL_MANIFEST:-$repo_root/.github/labels.yml}"

case "$manifest" in
  /*)
    ;;
  *)
    manifest="$repo_root/$manifest"
    ;;
esac

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 127
  fi
}

require gh
require python3

if [[ ! -f "$manifest" ]]; then
  echo "missing label manifest: $manifest" >&2
  exit 2
fi

live_labels_json="$(gh label list --repo "$repo" --limit 200 --json name,color,description)"
export CLAW_LIVE_LABELS_JSON="$live_labels_json"

python3 - "$manifest" <<'PY'
import json
import os
import sys


def parse_scalar(raw):
    value = raw.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        value = value[1:-1]
    return value


def read_manifest(path):
    labels = []
    current = None

    with open(path, encoding="utf-8") as handle:
        for raw_line in handle:
            stripped = raw_line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if stripped.startswith("- name:"):
                if current is not None:
                    labels.append(current)
                current = {"name": parse_scalar(stripped.removeprefix("- name:"))}
                continue
            if current is None:
                continue
            if stripped.startswith("color:"):
                current["color"] = parse_scalar(stripped.removeprefix("color:")).lower()
            elif stripped.startswith("description:"):
                current["description"] = parse_scalar(
                    stripped.removeprefix("description:")
                )

    if current is not None:
        labels.append(current)
    return labels


manifest_path = sys.argv[1]
expected = read_manifest(manifest_path)
if not expected:
    print(f"no labels found in {manifest_path}", file=sys.stderr)
    sys.exit(2)

missing_fields = [
    f"{label.get('name', '<unnamed>')} missing {field}"
    for label in expected
    for field in ("name", "color", "description")
    if not label.get(field)
]
if missing_fields:
    for issue in missing_fields:
        print(issue, file=sys.stderr)
    sys.exit(2)

live = {
    label["name"]: {
        "color": label.get("color", "").lower(),
        "description": label.get("description", ""),
    }
    for label in json.loads(os.environ["CLAW_LIVE_LABELS_JSON"])
}

errors = []
for label in expected:
    name = label["name"]
    actual = live.get(name)
    if actual is None:
        errors.append(f"missing live label: {name}")
        continue
    for field in ("color", "description"):
        if actual[field] != label[field]:
            errors.append(
                f"{name} {field} mismatch: expected {label[field]!r}, got {actual[field]!r}"
            )

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)

print(f"verified {len(expected)} GitHub labels from {manifest_path}")
PY

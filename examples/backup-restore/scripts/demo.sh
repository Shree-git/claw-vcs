#!/usr/bin/env bash
set -euo pipefail

CLAW_BIN="${CLAW_BIN:-claw}"
case "$CLAW_BIN" in
  /*) ;;
  */*) CLAW_BIN="$(cd "$(dirname "$CLAW_BIN")" && pwd)/$(basename "$CLAW_BIN")" ;;
  *) CLAW_BIN="$(command -v "$CLAW_BIN")" ;;
esac

workdir="$(mktemp -d "${TMPDIR:-/tmp}/claw-backup-demo.XXXXXX")"
repo="$workdir/repo"

cleanup() {
  rm -rf "$workdir"
}
trap cleanup EXIT

mkdir -p "$repo"
cd "$repo"

"$CLAW_BIN" init
"$CLAW_BIN" intent create --title "backup demo" --goal "show metadata rollback"
echo "demo data" > demo.txt
"$CLAW_BIN" snapshot -m "backup baseline"

created="$("$CLAW_BIN" admin backup create)"
backup_id="$(printf '%s\n' "$created" | awk -F': ' '/Created backup:/ {print $2}')"
if [[ -z "$backup_id" ]]; then
  echo "could not parse backup id" >&2
  printf '%s\n' "$created" >&2
  exit 1
fi

"$CLAW_BIN" admin backup verify --backup-id "$backup_id"

main_ref=".claw/refs/heads/main"
original_ref="$(cat "$main_ref")"
printf 'corrupted-ref\n' > "$main_ref"

"$CLAW_BIN" admin rollback plan --backup-id "$backup_id"
"$CLAW_BIN" admin rollback execute --backup-id "$backup_id"

restored_ref="$(cat "$main_ref")"
if [[ "$restored_ref" != "$original_ref" ]]; then
  echo "rollback did not restore heads/main" >&2
  exit 1
fi

"$CLAW_BIN" admin backup verify --backup-id "$backup_id"
echo "Backup restore demo passed: $backup_id"

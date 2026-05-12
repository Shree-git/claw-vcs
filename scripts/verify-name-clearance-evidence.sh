#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/verify-name-clearance-evidence.sh [evidence-file]

Verifies that the launch name-clearance evidence file contains completed,
non-placeholder values for the owner-side launch gates. This is an offline
syntax and completeness check; it does not perform trademark, domain, social,
package-registry, or repository-setting changes.

Default evidence file:
  docs/operations/name-clearance-evidence.md
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

evidence="${1:-${CLAW_PREFLIGHT_NAME_EVIDENCE:-docs/operations/name-clearance-evidence.md}}"
failures=0

fail() {
  failures=$((failures + 1))
  printf 'FAIL: %s\n' "$*" >&2
}

field_value() {
  local label="$1"
  awk -v prefix="- ${label}:" '
    index($0, prefix) == 1 {
      value = substr($0, length(prefix) + 1)
      sub(/^[[:space:]]+/, "", value)
      sub(/[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$evidence"
}

is_placeholder() {
  local value="$1"
  local normalized

  normalized="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  case "$normalized" in
    "" | no | none | n/a | *tbd* | *todo* | *pending* | *unknown* | *"not done"* | *"not complete"* | *incomplete*)
      return 0
      ;;
  esac
  return 1
}

require_completed_field() {
  local label="$1"
  local value

  value="$(field_value "$label")"
  if is_placeholder "$value"; then
    fail "$label must contain completed evidence, not a blank or placeholder value"
  fi
}

if [[ ! -f "$evidence" ]]; then
  echo "missing name-clearance evidence file: $evidence" >&2
  exit 1
fi

for label in \
  "Date" \
  "Reviewer" \
  "Final decision" \
  "Domains checked/reserved" \
  "Social handles checked/reserved" \
  "crates.io packages reserved/published"; do
  require_completed_field "$label"
done

social_preview="$(printf '%s' "$(field_value "GitHub social preview uploaded")" | tr '[:upper:]' '[:lower:]')"
if [[ "$social_preview" != "yes" ]]; then
  fail "GitHub social preview uploaded must be yes"
fi

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

echo "verified completed name-clearance evidence: $evidence"

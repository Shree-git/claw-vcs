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

require_yyyy_mm_dd() {
  local label="$1"
  local value

  value="$(field_value "$label")"
  if ! printf '%s' "$value" | grep -Eq '^[0-9]{4}-[0-9]{2}-[0-9]{2}$'; then
    fail "$label must use YYYY-MM-DD format"
  fi
}

require_yes_no() {
  local label="$1"
  local value

  value="$(printf '%s' "$(field_value "$label")" | tr '[:upper:]' '[:lower:]')"
  case "$value" in
    yes | no)
      ;;
    *)
      fail "$label must be yes or no"
      ;;
  esac
}

require_field_mentions() {
  local label="$1"
  local needle="$2"
  local value
  local normalized_value
  local normalized_needle

  value="$(field_value "$label")"
  normalized_value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  normalized_needle="$(printf '%s' "$needle" | tr '[:upper:]' '[:lower:]')"
  case "$normalized_value" in
    *"$normalized_needle"*)
      ;;
    *)
      fail "$label must mention $needle"
      ;;
  esac
}

if [[ ! -f "$evidence" ]]; then
  echo "missing name-clearance evidence file: $evidence" >&2
  exit 1
fi

require_yyyy_mm_dd "Date"

for label in \
  "Reviewer" \
  "Trademark databases checked" \
  "Similar marks and disposition" \
  "Final decision" \
  "Domains checked/reserved" \
  "Social handles checked/reserved" \
  "crates.io packages reserved/published"; do
  require_completed_field "$label"
done

require_yes_no "Counsel review required"

for database in USPTO WIPO EUIPO; do
  require_field_mentions "Trademark databases checked" "$database"
done

for crate_name in \
  claw-vcs \
  claw-vcs-core \
  claw-vcs-store \
  claw-vcs-patch \
  claw-vcs-merge \
  claw-vcs-crypto \
  claw-vcs-policy \
  claw-vcs-sync \
  claw-vcs-git; do
  require_field_mentions "crates.io packages reserved/published" "$crate_name"
done

social_preview="$(printf '%s' "$(field_value "GitHub social preview uploaded")" | tr '[:upper:]' '[:lower:]')"
if [[ "$social_preview" != "yes" ]]; then
  fail "GitHub social preview uploaded must be yes"
fi

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

echo "verified completed name-clearance evidence: $evidence"

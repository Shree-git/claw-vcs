#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/public-launch-preflight.sh

Checks public-launch state that depends on GitHub or package-registry access:
  - GitHub repository identity, visibility, topics, and security settings
  - main branch protection and signed-commit enforcement
  - repository labels declared in .github/labels.yml
  - open Dependabot alert state
  - package-name availability/reservation signals
  - local social preview asset readiness and GitHub upload state

This script is intentionally launch-gating. It may fail until maintainer-owned
external actions are complete.

Environment:
  CLAW_PREFLIGHT_REPO            GitHub repository (default: Shree-git/claw-vcs)
  CLAW_PREFLIGHT_BRANCH          Protected branch to inspect (default: main)
  CLAW_PREFLIGHT_REQUIRE_PAGES   Set to 1 when GitHub Pages is part of launch
  CLAW_PREFLIGHT_STRICT          Set to 1 for broad-announcement readiness
  CLAW_PREFLIGHT_NAME_EVIDENCE   Completed name-clearance evidence markdown
  CLAW_PREFLIGHT_CRATESIO_OWNER  Expected crates.io owner login/team id for reserved packages
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo="${CLAW_PREFLIGHT_REPO:-Shree-git/claw-vcs}"
branch="${CLAW_PREFLIGHT_BRANCH:-main}"
require_pages="${CLAW_PREFLIGHT_REQUIRE_PAGES:-0}"
strict="${CLAW_PREFLIGHT_STRICT:-0}"
name_evidence="${CLAW_PREFLIGHT_NAME_EVIDENCE:-docs/operations/name-clearance-evidence.md}"
cratesio_owner="${CLAW_PREFLIGHT_CRATESIO_OWNER:-${CLAW_CRATESIO_EXPECTED_OWNER:-}}"

failures=0
warnings=0

pass() {
  printf 'PASS: %s\n' "$*"
}

warn() {
  warnings=$((warnings + 1))
  printf 'WARN: %s\n' "$*" >&2
}

strict_warn() {
  if [[ "$strict" == "1" ]]; then
    fail "$*"
  else
    warn "$*"
  fi
}

fail() {
  failures=$((failures + 1))
  printf 'FAIL: %s\n' "$*" >&2
}

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    fail "missing required command: $1"
    return 1
  fi
}

contains_line() {
  local needle="$1"
  shift
  printf '%s\n' "$@" | grep -Fxq "$needle"
}

gh_value() {
  local endpoint="$1"
  local query="$2"
  gh api "$endpoint" --jq "$query" 2>/dev/null || true
}

expect_value() {
  local label="$1"
  local actual="$2"
  local expected="$3"
  if [[ "$actual" == "$expected" ]]; then
    pass "$label is $expected"
  else
    fail "$label expected $expected, got ${actual:-<empty>}"
  fi
}

require gh || true
require curl || true
require git || true
require python3 || true

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

verify_cratesio_owner() {
  local crate_name="$1"
  local owners_json

  if [[ -z "$cratesio_owner" ]]; then
    strict_warn "crates.io name $crate_name exists, but CLAW_PREFLIGHT_CRATESIO_OWNER is not set; verify maintainer ownership before documenting crates.io install"
    return 0
  fi

  owners_json="$(curl -L -fsS "https://crates.io/api/v1/crates/$crate_name/owners" 2>/dev/null || true)"
  if [[ -z "$owners_json" ]]; then
    fail "could not inspect crates.io owners for $crate_name"
    return 0
  fi

  if printf '%s' "$owners_json" | python3 -c '
import json
import sys

expected = sys.argv[1]
payload = json.load(sys.stdin)
for user in payload.get("users", []):
    if user.get("login") == expected:
        sys.exit(0)
sys.exit(1)
' "$cratesio_owner"
  then
    pass "crates.io owner verified for $crate_name: $cratesio_owner"
  else
    fail "crates.io owner check failed for $crate_name: expected $cratesio_owner"
  fi
}

if ! gh auth status -h github.com >/dev/null 2>&1; then
  fail "gh must be authenticated to inspect repository settings"
fi

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

echo "Checking public-launch state for $repo ($branch)"

name_with_owner="$(gh repo view "$repo" --json nameWithOwner --jq '.nameWithOwner')"
is_private="$(gh repo view "$repo" --json isPrivate --jq '.isPrivate')"
expect_value "repository name" "$name_with_owner" "$repo"
expect_value "repository private flag" "$is_private" "false"

topics=()
while IFS= read -r topic; do
  topics+=("$topic")
done < <(gh repo view "$repo" --json repositoryTopics --jq '.repositoryTopics[].name')
for topic in \
  version-control \
  vcs \
  provenance \
  ai-agents \
  supply-chain-security \
  cli \
  rust \
  git \
  sigstore \
  slsa \
  developer-tools; do
  if contains_line "$topic" "${topics[@]}"; then
    pass "repository topic present: $topic"
  else
    fail "repository topic missing: $topic"
  fi
done

if label_output="$(CLAW_LABEL_REPO="$repo" scripts/verify-github-labels.sh 2>&1)"; then
  pass "$label_output"
else
  fail "GitHub labels do not match .github/labels.yml: $label_output"
fi

secret_scanning="$(gh_value "repos/$repo" '.security_and_analysis.secret_scanning.status // "unavailable"')"
push_protection="$(gh_value "repos/$repo" '.security_and_analysis.secret_scanning_push_protection.status // "unavailable"')"
expect_value "secret scanning" "$secret_scanning" "enabled"
expect_value "secret scanning push protection" "$push_protection" "enabled"

dependabot_enabled="$(gh_value "repos/$repo/automated-security-fixes" '.enabled')"
if [[ "$dependabot_enabled" == "true" ]]; then
  pass "Dependabot security updates are enabled"
else
  fail "Dependabot security updates expected true, got ${dependabot_enabled:-<empty>}"
fi

dependabot_alerts_file="$(mktemp)"
if gh api --paginate "repos/$repo/dependabot/alerts?state=open" \
  --jq '.[] | [.number, .security_vulnerability.package.name, .security_vulnerability.severity, .security_advisory.ghsa_id, .dependency.manifest_path] | @tsv' \
  >"$dependabot_alerts_file" 2>/dev/null; then
  if [[ -s "$dependabot_alerts_file" ]]; then
    dependabot_alert_count="$(wc -l < "$dependabot_alerts_file" | tr -d ' ')"
    dependabot_alert_summary="$(
      awk -F '\t' '{ printf "#%s %s %s %s (%s); ", $1, $2, $3, $4, $5 }' "$dependabot_alerts_file" |
        sed 's/; $//'
    )"
    fail "Dependabot has $dependabot_alert_count open alert(s): $dependabot_alert_summary"
  else
    pass "Dependabot has no open alerts"
  fi
else
  fail "could not inspect open Dependabot alerts for $repo"
fi
rm -f "$dependabot_alerts_file"

protection="repos/$repo/branches/$branch/protection"
required_reviews="$(gh_value "$protection" '.required_pull_request_reviews.required_approving_review_count // 0')"
dismiss_stale="$(gh_value "$protection" '.required_pull_request_reviews.dismiss_stale_reviews // false')"
code_owner_reviews="$(gh_value "$protection" '.required_pull_request_reviews.require_code_owner_reviews // false')"
last_push_approval="$(gh_value "$protection" '.required_pull_request_reviews.require_last_push_approval // false')"
strict_checks="$(gh_value "$protection" '.required_status_checks.strict // false')"
required_context_count="$(gh_value "$protection" '((.required_status_checks.contexts // []) | length) + ((.required_status_checks.checks // []) | length)')"
conversation_resolution="$(gh_value "$protection" '.required_conversation_resolution.enabled // false')"
allow_force_pushes="$(gh_value "$protection" '.allow_force_pushes.enabled // false')"
allow_deletions="$(gh_value "$protection" '.allow_deletions.enabled // false')"
signed_commits="$(gh_value "$protection/required_signatures" '.enabled')"
required_contexts=()
while IFS= read -r context; do
  required_contexts+=("$context")
done < <(
  gh api "$protection" --jq '(.required_status_checks.contexts // [])[], (.required_status_checks.checks // [] | .[].context)' 2>/dev/null || true
)

if [[ "${required_reviews:-0}" -ge 1 ]]; then
  pass "branch protection requires at least one approving review"
else
  fail "branch protection must require at least one approving review"
fi
expect_value "dismiss stale approvals" "$dismiss_stale" "true"
expect_value "code-owner review requirement" "$code_owner_reviews" "true"
expect_value "last-push approval requirement" "$last_push_approval" "true"
expect_value "strict required status checks" "$strict_checks" "true"
if [[ "${required_context_count:-0}" -gt 0 ]]; then
  pass "branch protection has required status checks"
else
  fail "branch protection must include required status checks"
fi
for context in \
  "Rust quality gate" \
  "Dependency Review" \
  "CodeQL analysis" \
  "Semgrep analysis"; do
  if contains_line "$context" "${required_contexts[@]}"; then
    pass "required status check present: $context"
  else
    fail "required status check missing: $context"
  fi
done
expect_value "conversation resolution requirement" "$conversation_resolution" "true"
expect_value "signed commits requirement" "$signed_commits" "true"
expect_value "force pushes allowed" "$allow_force_pushes" "false"
expect_value "branch deletions allowed" "$allow_deletions" "false"

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
  crates_status="$(curl -L -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$crate_name" || true)"
  case "$crates_status" in
    200)
      pass "crates.io name $crate_name exists"
      verify_cratesio_owner "$crate_name"
      ;;
    404)
      fail "crates.io name $crate_name is still unreserved; reserve or publish before broad announcement"
      ;;
    *)
      warn "could not determine crates.io $crate_name status, HTTP $crates_status"
      ;;
  esac
done

winget_status="$(
  gh api repos/microsoft/winget-pkgs/contents/manifests/s/ShreeGit/ClawVCS \
    --silent >/dev/null 2>&1 && printf 'present' || printf 'absent'
)"
if [[ "$winget_status" == "present" ]]; then
  pass "WinGet manifest path exists for ShreeGit.ClawVCS"
else
  warn "WinGet manifest path is absent; keep WinGet documented as planned"
fi

if gh api repos/Shree-git/homebrew-tap/contents/Formula/claw.rb --silent >/dev/null 2>&1; then
  pass "Homebrew tap formula exists"
else
  fail "Homebrew tap formula missing: Shree-git/homebrew-tap Formula/claw.rb"
fi

social_preview="docs/assets/social-preview.png"
if [[ -f "$social_preview" ]]; then
  size="$(wc -c < "$social_preview" | tr -d ' ')"
  if [[ "$size" -lt 1000000 ]]; then
    pass "social preview asset exists and is under 1 MB"
  else
    fail "social preview asset exceeds GitHub's 1 MB upload limit"
  fi
  dimensions="$(
    python3 - "$social_preview" <<'PY'
import struct
import sys

path = sys.argv[1]
with open(path, "rb") as handle:
    data = handle.read(24)
if len(data) < 24 or not data.startswith(b"\x89PNG\r\n\x1a\n"):
    print("invalid invalid")
else:
    width, height = struct.unpack(">II", data[16:24])
    print(f"{width} {height}")
PY
  )"
  social_width="${dimensions%% *}"
  social_height="${dimensions#* }"
  if [[ "$social_width" == "1280" && "$social_height" == "640" ]]; then
    pass "social preview dimensions are 1280x640"
  else
    fail "social preview dimensions must be 1280x640, got ${social_width:-<empty>}x${social_height:-<empty>}"
  fi
else
  fail "missing social preview asset: $social_preview"
fi

custom_open_graph="$(gh repo view "$repo" --json usesCustomOpenGraphImage --jq '.usesCustomOpenGraphImage' 2>/dev/null || true)"
if [[ "$custom_open_graph" == "true" ]]; then
  pass "GitHub social preview image is uploaded"
else
  strict_warn "GitHub social preview image is not uploaded; upload $social_preview in repository settings"
fi

if gh api "repos/$repo/pages" --silent >/dev/null 2>&1; then
  pages_status="$(gh api "repos/$repo/pages" --jq '.status // "configured"' 2>/dev/null || true)"
  pass "GitHub Pages is configured with status: $pages_status"
elif [[ "$require_pages" == "1" ]]; then
  fail "GitHub Pages is required but not configured for $repo"
else
  warn "GitHub Pages is not configured; leave optional unless the launch includes a public website"
fi

evidence_value() {
  local label="$1"
  awk -v prefix="- ${label}:" '
    index($0, prefix) == 1 {
      value = substr($0, length(prefix) + 1)
      sub(/^[[:space:]]+/, "", value)
      sub(/[[:space:]]+$/, "", value)
      print value
      exit
    }
  ' "$name_evidence"
}

evidence_field_complete() {
  local label="$1"
  local value
  local normalized

  value="$(evidence_value "$label")"
  normalized="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]')"
  case "$normalized" in
    "" | tbd | todo | pending | unknown | none | n/a | no | "not done" | "not complete" | incomplete)
      return 1
      ;;
  esac
  return 0
}

if [[ "$strict" == "1" ]]; then
  evidence_complete=1
  if [[ ! -f "$name_evidence" ]]; then
    evidence_complete=0
  else
    for evidence_label in \
      "Final decision" \
      "Domains checked/reserved" \
      "Social handles checked/reserved" \
      "crates.io packages reserved/published"; do
      if ! evidence_field_complete "$evidence_label"; then
        evidence_complete=0
      fi
    done
    if [[ "$(evidence_value "GitHub social preview uploaded")" != "yes" ]]; then
      evidence_complete=0
    fi
  fi

  if [[ "$evidence_complete" == "1" ]]; then
    pass "name-clearance evidence is recorded in $name_evidence"
  else
    fail "strict launch mode requires completed name/domain/social/package evidence in $name_evidence; start from docs/operations/name-clearance-evidence.template.md"
  fi
else
  warn "trademark clearance, domain checks, and social-handle checks require manual maintainer verification"
fi

echo
echo "Public-launch preflight finished with $failures failure(s), $warnings warning(s)."
if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

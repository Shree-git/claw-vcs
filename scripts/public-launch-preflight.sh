#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/public-launch-preflight.sh

Checks public-launch state that depends on GitHub or package-registry access:
  - GitHub repository identity, visibility, topics, and security settings
  - main branch protection and signed-commit enforcement
  - package-name availability/reservation signals
  - local social preview asset readiness and GitHub upload state

This script is intentionally launch-gating. It may fail until maintainer-owned
external actions are complete.

Environment:
  CLAW_PREFLIGHT_REPO            GitHub repository (default: Shree-git/claw-vcs)
  CLAW_PREFLIGHT_BRANCH          Protected branch to inspect (default: main)
  CLAW_PREFLIGHT_REQUIRE_PAGES   Set to 1 when GitHub Pages is part of launch
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

repo="${CLAW_PREFLIGHT_REPO:-Shree-git/claw-vcs}"
branch="${CLAW_PREFLIGHT_BRANCH:-main}"
require_pages="${CLAW_PREFLIGHT_REQUIRE_PAGES:-0}"

failures=0
warnings=0

pass() {
  printf 'PASS: %s\n' "$*"
}

warn() {
  warnings=$((warnings + 1))
  printf 'WARN: %s\n' "$*" >&2
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

if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

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
      pass "crates.io name $crate_name exists; verify maintainer ownership before documenting crates.io install"
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
else
  fail "missing social preview asset: $social_preview"
fi

custom_open_graph="$(gh repo view "$repo" --json usesCustomOpenGraphImage --jq '.usesCustomOpenGraphImage' 2>/dev/null || true)"
if [[ "$custom_open_graph" == "true" ]]; then
  pass "GitHub social preview image is uploaded"
else
  warn "GitHub social preview image is not uploaded; upload $social_preview in repository settings"
fi

if gh api "repos/$repo/pages" --silent >/dev/null 2>&1; then
  pages_status="$(gh api "repos/$repo/pages" --jq '.status // "configured"' 2>/dev/null || true)"
  pass "GitHub Pages is configured with status: $pages_status"
elif [[ "$require_pages" == "1" ]]; then
  fail "GitHub Pages is required but not configured for $repo"
else
  warn "GitHub Pages is not configured; leave optional unless the launch includes a public website"
fi

warn "trademark clearance, domain checks, and social-handle checks require manual maintainer verification"

echo
echo "Public-launch preflight finished with $failures failure(s), $warnings warning(s)."
if [[ "$failures" -gt 0 ]]; then
  exit 1
fi

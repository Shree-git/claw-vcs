#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/publish-cratesio.sh [--dry-run | --publish] [--package <name>] [--start-at <name>]

Publishes the Claw VCS crates.io package set in dependency order.

Default mode is --dry-run. Real publishing requires both:

  --publish
  CLAW_CRATESIO_PUBLISH=1

Options:
  --dry-run          Run cargo publish dry-runs only (default)
  --publish          Publish for real after explicit environment opt-in
  --package <name>   Check or publish one package from the known package set
  --start-at <name>  Start at this package and continue through the order
  -h, --help         Show this help

Environment:
  CLAW_CRATESIO_PUBLISH=1       Required with --publish
  CLAW_CRATESIO_EXPECTED_OWNER  Required with --publish; crates.io login/team id that must own each crate after publish
  CLAW_CRATESIO_RELEASE_TAG     Optional expected tag; defaults to v<workspace version>
  CLAW_CRATESIO_REPO_URL        Canonical git repo URL to verify the release tag (default: https://github.com/Shree-git/claw-vcs.git)
  CLAW_CRATESIO_POLL_SECONDS    Seconds between registry visibility checks (default: 15)
  CLAW_CRATESIO_POLL_ATTEMPTS   Max checks after each real publish (default: 40)

Notes:
  Cargo resolves publish dependencies from the registry during packaging. In
  default --dry-run mode, packages blocked by unpublished internal dependencies
  are skipped with an explanation. Explicit --package and --start-at dry-runs
  fail fast if the selected package cannot resolve its registry dependencies.
  Run this script in --publish mode only from the intended release commit.
USAGE
}

packages=(
  claw-vcs-core
  claw-vcs-store
  claw-vcs-patch
  claw-vcs-crypto
  claw-vcs-policy
  claw-vcs-merge
  claw-vcs-sync
  claw-vcs-git
  claw-vcs
)

mode="dry-run"
single_package=""
start_at=""
workspace_version=""

contains_package() {
  local candidate="$1"
  local package
  for package in "${packages[@]}"; do
    if [[ "$package" == "$candidate" ]]; then
      return 0
    fi
  done
  return 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      mode="dry-run"
      shift
      ;;
    --publish)
      mode="publish"
      shift
      ;;
    --package)
      single_package="${2:-}"
      if [[ -z "$single_package" ]]; then
        echo "--package requires a package name" >&2
        exit 2
      fi
      shift 2
      ;;
    --start-at)
      start_at="${2:-}"
      if [[ -z "$start_at" ]]; then
        echo "--start-at requires a package name" >&2
        exit 2
      fi
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -n "$single_package" && -n "$start_at" ]]; then
  echo "--package and --start-at cannot be combined" >&2
  exit 2
fi

if [[ -n "$single_package" ]] && ! contains_package "$single_package"; then
  echo "unknown package: $single_package" >&2
  exit 2
fi

if [[ -n "$start_at" ]] && ! contains_package "$start_at"; then
  echo "unknown package: $start_at" >&2
  exit 2
fi

if [[ "$mode" == "publish" && "${CLAW_CRATESIO_PUBLISH:-0}" != "1" ]]; then
  echo "refusing to publish without CLAW_CRATESIO_PUBLISH=1" >&2
  exit 2
fi

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 127
  fi
}

require cargo
require curl
require jq

workspace_version="$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "claw-vcs") | .version' | head -n 1)"
if [[ -z "$workspace_version" || "$workspace_version" == "null" ]]; then
  echo "could not determine claw-vcs workspace version" >&2
  exit 2
fi

if [[ "$mode" == "publish" ]]; then
  require git

  if [[ -z "${CLAW_CRATESIO_EXPECTED_OWNER:-}" ]]; then
    echo "refusing to publish without CLAW_CRATESIO_EXPECTED_OWNER=<crates.io owner login or team id>" >&2
    exit 2
  fi

  if [[ -z "${CARGO_REGISTRY_TOKEN:-}" && ! -f "$HOME/.cargo/credentials.toml" && ! -f "$HOME/.cargo/credentials" ]]; then
    echo "no crates.io credentials found; run cargo login or set CARGO_REGISTRY_TOKEN" >&2
    exit 2
  fi

  if [[ -n "$(git status --porcelain)" ]]; then
    echo "refusing to publish from a dirty working tree" >&2
    exit 2
  fi

  expected_tag="${CLAW_CRATESIO_RELEASE_TAG:-v${workspace_version}}"
  actual_tag="$(git describe --tags --exact-match HEAD 2>/dev/null || true)"
  if [[ "$actual_tag" != "$expected_tag" ]]; then
    echo "refusing to publish: HEAD must be exactly at release tag $expected_tag (got ${actual_tag:-<none>})" >&2
    exit 2
  fi

  tag_commit="$(git rev-list -n 1 "$expected_tag")"
  head_commit="$(git rev-parse HEAD)"
  if [[ "$tag_commit" != "$head_commit" ]]; then
    echo "refusing to publish: release tag $expected_tag does not resolve to HEAD" >&2
    exit 2
  fi

  repo_url="${CLAW_CRATESIO_REPO_URL:-https://github.com/Shree-git/claw-vcs.git}"
  remote_refs="$(git ls-remote --tags "$repo_url" "refs/tags/${expected_tag}" "refs/tags/${expected_tag}^{}")"
  remote_tag_commit="$(printf '%s\n' "$remote_refs" | awk -v ref="refs/tags/${expected_tag}^{}" '$2 == ref { print $1; found = 1 } END { if (!found) exit 1 }' || true)"
  if [[ -z "$remote_tag_commit" ]]; then
    remote_tag_commit="$(printf '%s\n' "$remote_refs" | awk -v ref="refs/tags/${expected_tag}" '$2 == ref { print $1; exit }')"
  fi
  if [[ "$remote_tag_commit" != "$head_commit" ]]; then
    echo "refusing to publish: remote release tag $expected_tag at $repo_url must resolve to HEAD" >&2
    echo "remote: ${remote_tag_commit:-<missing>}" >&2
    echo "HEAD:   $head_commit" >&2
    exit 2
  fi

  for package in "${packages[@]}"; do
    package_version="$(cargo metadata --format-version=1 --no-deps | jq -r --arg package "$package" '.packages[] | select(.name == $package) | .version' | head -n 1)"
    if [[ "$package_version" != "$workspace_version" ]]; then
      echo "refusing to publish: $package version ${package_version:-<missing>} does not match workspace version $workspace_version" >&2
      exit 2
    fi
  done
fi

selected_packages=()
if [[ -n "$single_package" ]]; then
  selected_packages=("$single_package")
elif [[ -n "$start_at" ]]; then
  include=0
  for package in "${packages[@]}"; do
    if [[ "$package" == "$start_at" ]]; then
      include=1
    fi
    if [[ "$include" -eq 1 ]]; then
      selected_packages+=("$package")
    fi
  done
else
  selected_packages=("${packages[@]}")
fi

wait_for_crate() {
  local package="$1"
  local attempts="${CLAW_CRATESIO_POLL_ATTEMPTS:-40}"
  local sleep_seconds="${CLAW_CRATESIO_POLL_SECONDS:-15}"
  local attempt
  local status

  for attempt in $(seq 1 "$attempts"); do
    status="$(curl -L -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$package/$workspace_version" || true)"
    if [[ "$status" == "200" ]]; then
      echo "crates.io package version visible: $package $workspace_version"
      return 0
    fi
    echo "waiting for crates.io package $package $workspace_version to become visible ($attempt/$attempts, HTTP $status)"
    sleep "$sleep_seconds"
  done

  echo "timed out waiting for $package $workspace_version on crates.io" >&2
  return 1
}

crate_version_exists() {
  local package="$1"
  local status

  status="$(curl -L -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$package/$workspace_version" || true)"
  [[ "$status" == "200" ]]
}

verify_crate_owner() {
  local package="$1"
  local expected_owner="${CLAW_CRATESIO_EXPECTED_OWNER:-}"
  local owners_json

  owners_json="$(curl -L -fsS "https://crates.io/api/v1/crates/$package/owners")"
  if ! printf '%s' "$owners_json" | jq -e --arg owner "$expected_owner" '
    .users[]? | select(.login == $owner)
  ' >/dev/null; then
    echo "crates.io owner login check failed for $package: expected owner $expected_owner" >&2
    exit 1
  fi

  echo "crates.io owner verified for $package: $expected_owner"
}

internal_deps() {
  case "$1" in
    claw-vcs-core)
      ;;
    claw-vcs-store | claw-vcs-patch | claw-vcs-crypto | claw-vcs-policy)
      echo "claw-vcs-core"
      ;;
    claw-vcs-merge)
      echo "claw-vcs-core claw-vcs-patch claw-vcs-store"
      ;;
    claw-vcs-sync)
      echo "claw-vcs-core claw-vcs-store claw-vcs-crypto"
      ;;
    claw-vcs-git)
      echo "claw-vcs-core claw-vcs-store"
      ;;
    claw-vcs)
      echo "claw-vcs-core claw-vcs-store claw-vcs-patch claw-vcs-merge claw-vcs-crypto claw-vcs-policy claw-vcs-sync claw-vcs-git"
      ;;
  esac
}

missing_registry_deps() {
  local package="$1"
  local dep
  local missing=()

  for dep in $(internal_deps "$package"); do
    if ! crate_version_exists "$dep"; then
      missing+=("$dep")
    fi
  done

  if [[ "${#missing[@]}" -gt 0 ]]; then
    printf '%s\n' "${missing[*]}"
  fi
}

skipped=0
for package in "${selected_packages[@]}"; do
  echo "== $package ($mode)"
  if [[ "$mode" == "dry-run" ]]; then
    missing_deps="$(missing_registry_deps "$package")"
    if [[ -n "$missing_deps" ]]; then
      if [[ -n "$single_package" || -n "$start_at" ]]; then
        echo "cannot dry-run $package until registry dependencies are live: $missing_deps" >&2
        exit 1
      fi
      echo "skipping dry-run for $package until registry dependencies are live: $missing_deps"
      skipped=$((skipped + 1))
      continue
    fi
    cargo publish -p "$package" --dry-run --locked --allow-dirty --registry crates-io
  else
    for dep in $(internal_deps "$package"); do
      verify_crate_owner "$dep"
    done
    cargo publish -p "$package" --locked --registry crates-io
    wait_for_crate "$package"
    verify_crate_owner "$package"
  fi
done

if [[ "$mode" == "dry-run" && "$skipped" -gt 0 ]]; then
  echo "dry-run skipped $skipped package(s) blocked by unpublished internal crates"
fi

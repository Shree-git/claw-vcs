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

if [[ "$mode" == "publish" ]]; then
  if [[ -z "${CARGO_REGISTRY_TOKEN:-}" && ! -f "$HOME/.cargo/credentials.toml" && ! -f "$HOME/.cargo/credentials" ]]; then
    echo "no crates.io credentials found; run cargo login or set CARGO_REGISTRY_TOKEN" >&2
    exit 2
  fi
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
    status="$(curl -L -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$package" || true)"
    if [[ "$status" == "200" ]]; then
      echo "crates.io package visible: $package"
      return 0
    fi
    echo "waiting for crates.io package $package to become visible ($attempt/$attempts, HTTP $status)"
    sleep "$sleep_seconds"
  done

  echo "timed out waiting for $package on crates.io" >&2
  return 1
}

crate_exists() {
  local package="$1"
  local status

  status="$(curl -L -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/$package" || true)"
  [[ "$status" == "200" ]]
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
    if ! crate_exists "$dep"; then
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
    cargo publish -p "$package" --dry-run --locked --allow-dirty
  else
    cargo publish -p "$package" --locked
    wait_for_crate "$package"
  fi
done

if [[ "$mode" == "dry-run" && "$skipped" -gt 0 ]]; then
  echo "dry-run skipped $skipped package(s) blocked by unpublished internal crates"
fi

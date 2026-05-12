#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/verify-release-channel.sh <release-tag>

Verifies the release channels that can be checked from the current Unix host:
  - GitHub release archive for this OS/architecture
  - shell installer inside an isolated temporary HOME
  - cargo install from Git for the exact release tag

Set CLAW_VERIFY_HOMEBREW=1 on macOS to also verify the Homebrew tap. Windows
PowerShell installer and MSI checks run in .github/workflows/release-channel-smoke.yml.

Environment:
  CLAW_RELEASE_REPO              GitHub repo to verify (default: Shree-git/claw-vcs)
  CLAW_RELEASE_VERIFY_WORKDIR    Existing work directory to reuse
  CLAW_SKIP_CARGO_INSTALL=1      Skip the cargo install --git check
  CLAW_KEEP_RELEASE_VERIFY=1     Keep the temporary work directory
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

tag="${1:-}"
if [[ -z "$tag" ]]; then
  usage >&2
  exit 2
fi

repo="${CLAW_RELEASE_REPO:-Shree-git/claw-vcs}"
workdir="${CLAW_RELEASE_VERIFY_WORKDIR:-$(mktemp -d)}"
if [[ "${CLAW_KEEP_RELEASE_VERIFY:-0}" != "1" && -z "${CLAW_RELEASE_VERIFY_WORKDIR:-}" ]]; then
  trap 'rm -rf "$workdir"' EXIT
fi

require() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 127
  fi
}

require gh
require tar
require shasum

expected_version="${tag#v}"

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) archive="claw-aarch64-apple-darwin.tar.xz" ;;
  Darwin:x86_64) archive="claw-x86_64-apple-darwin.tar.xz" ;;
  Linux:x86_64) archive="claw-x86_64-unknown-linux-gnu.tar.xz" ;;
  Linux:aarch64 | Linux:arm64) archive="claw-aarch64-unknown-linux-gnu.tar.xz" ;;
  *)
    echo "unsupported host for archive verification: $(uname -s) $(uname -m)" >&2
    exit 2
    ;;
esac

assets="$workdir/assets"
archive_dir="$workdir/archive"
installer_home="$workdir/installer-home"
cargo_root="$workdir/cargo-install"

mkdir -p "$assets" "$archive_dir" "$installer_home"

echo "Verifying $repo release $tag in $workdir"

gh release download "$tag" --repo "$repo" \
  --pattern "$archive" \
  --pattern "sha256.sum" \
  --pattern "claw-installer.sh" \
  --dir "$assets"

verify_sha256_entry() {
  local file="$1"
  local sums="$assets/sha256.sum"
  local expected
  local actual

  expected="$(awk -v file="$file" 'index($0, file) { print $1; exit }' "$sums")"
  if [[ -z "$expected" ]]; then
    echo "missing checksum entry for $file in sha256.sum" >&2
    exit 1
  fi

  actual="$(shasum -a 256 "$assets/$file" | awk '{ print $1 }')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for $file" >&2
    echo "expected: $expected" >&2
    echo "actual:   $actual" >&2
    exit 1
  fi
}

verify_sha256_entry "$archive"
verify_sha256_entry "claw-installer.sh"

smoke_repo() {
  local binary="$1"
  local repo_dir="$2"
  local actual_version

  actual_version="$("$binary" --version | awk '{ print $2 }')"
  if [[ "$actual_version" != "$expected_version" ]]; then
    echo "version mismatch for $binary: expected $expected_version, got ${actual_version:-<empty>}" >&2
    exit 1
  fi
  "$binary" doctor
  mkdir -p "$repo_dir"
  (
    cd "$repo_dir"
    "$binary" init
    "$binary" status
    "$binary" doctor
  )
}

tar -xJf "$assets/$archive" -C "$archive_dir"
archive_binary="$(find "$archive_dir" -type f -name claw | head -n 1)"
if [[ -z "$archive_binary" ]]; then
  echo "archive did not contain a claw binary" >&2
  exit 1
fi
smoke_repo "$archive_binary" "$workdir/archive-repo"

HOME="$installer_home" bash "$assets/claw-installer.sh"
installer_binary=""
for candidate in "$installer_home/.local/bin/claw" "$installer_home/.cargo/bin/claw"; do
  if [[ -x "$candidate" ]]; then
    installer_binary="$candidate"
    break
  fi
done
if [[ -z "$installer_binary" ]]; then
  echo "installer did not create a claw binary in the isolated HOME" >&2
  exit 1
fi
smoke_repo "$installer_binary" "$workdir/installer-repo"

if [[ "${CLAW_SKIP_CARGO_INSTALL:-0}" != "1" ]]; then
  require cargo
  cargo install --git "https://github.com/${repo}.git" --tag "$tag" --package claw-vcs --locked --root "$cargo_root"
  smoke_repo "$cargo_root/bin/claw" "$workdir/cargo-repo"
else
  echo "Skipping cargo install --git check because CLAW_SKIP_CARGO_INSTALL=1"
fi

if [[ "$(uname -s)" == "Darwin" && "${CLAW_VERIFY_HOMEBREW:-0}" == "1" ]]; then
  require brew
  brew tap shree-git/tap
  brew install shree-git/tap/claw
  smoke_repo "$(command -v claw)" "$workdir/homebrew-repo"
else
  echo "Skipping Homebrew check; set CLAW_VERIFY_HOMEBREW=1 on macOS to enable it."
fi

echo "Release-channel verification passed for $repo $tag"

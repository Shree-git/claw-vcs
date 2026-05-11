#!/bin/sh
set -eu

repo="${CLAW_REPO:-/var/lib/claw/repo}"

if [ "${1:-}" = "daemon" ]; then
  mkdir -p "$repo"
  cd "$repo"
  if [ ! -d ".claw" ]; then
    /usr/local/bin/claw init
  fi
fi

exec /usr/local/bin/claw "$@"

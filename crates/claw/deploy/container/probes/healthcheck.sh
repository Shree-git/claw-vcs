#!/bin/sh
set -eu

host="${HEALTHCHECK_HOST:-127.0.0.1}"
port="${HEALTHCHECK_PORT:-50052}"
path="${HEALTHCHECK_PATH:-/v1/health/live}"
timeout_secs="${HEALTHCHECK_TIMEOUT:-2}"

wget -q -T "${timeout_secs}" -O /dev/null "http://${host}:${port}${path}"

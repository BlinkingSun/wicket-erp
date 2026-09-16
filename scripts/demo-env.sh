#!/usr/bin/env bash
# Export demo environment. Usage: source scripts/demo-env.sh (bash) or scripts/demo-env.sh
set -eu
_SCRIPT="${BASH_SOURCE[0]:-${0:-scripts/demo-env.sh}}"
DEMO_ENV_ROOT="$(cd "$(dirname "$_SCRIPT")/.." && pwd)"
# shellcheck disable=SC1091
source "$DEMO_ENV_ROOT/dev/demo.env"
export WICKET_PROFILE WICKET_BIND WICKET_DATABASE_URL WICKET_MIGRATE_DATABASE_URL
export WICKET_BOOTSTRAP_URL WICKET_DEMO_USER WICKET_DEMO_PASSWORD WICKET_DEMO_BASE_URL
export WICKET_BLOB_ROOT="$DEMO_ENV_ROOT/dev/demo-blobs"
mkdir -p "$WICKET_BLOB_ROOT"

#!/usr/bin/env bash
# Reset the standing demo database (wicket_demo). Same SQL as just db-reset, demo URLs.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/demo-env.sh"
export WICKET_BOOTSTRAP_URL WICKET_DATABASE_URL WICKET_MIGRATE_DATABASE_URL
exec just -f "$ROOT/justfile" db-reset

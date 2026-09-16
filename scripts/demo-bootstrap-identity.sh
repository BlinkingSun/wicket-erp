#!/usr/bin/env bash
# Apply dev/demo/bootstrap-identity.sql (requires migrate + Argon2id hash).
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/demo-env.sh"

if command -v brew >/dev/null 2>&1 && [ -x "$(brew --prefix postgresql@17)/bin/psql" ]; then
  psql="$(brew --prefix postgresql@17)/bin/psql"
else
  psql="$(command -v psql)"
fi

login_hash="$(python3 - <<'PY'
import argon2
h = argon2.PasswordHasher(time_cost=2, memory_cost=19456, parallelism=1, hash_len=32, salt_len=16)
print(h.hash("demo-login"))
PY
)"

"$psql" "$WICKET_DATABASE_URL" -v ON_ERROR_STOP=1 -v login_hash="$login_hash" \
  -f "$ROOT/dev/demo/bootstrap-identity.sql"

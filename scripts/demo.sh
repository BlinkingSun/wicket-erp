#!/usr/bin/env bash
# One-command local demo: Postgres, migrate, demo login, seed, serve.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/demo-env.sh"

die() {
  echo "demo: $*" >&2
  exit 1
}

if command -v brew >/dev/null 2>&1 && [ -x "$(brew --prefix postgresql@17)/bin/psql" ]; then
  psql="$(brew --prefix postgresql@17)/bin/psql"
else
  psql="$(command -v psql)"
fi

if ! command -v docker >/dev/null 2>&1; then
  prefix="$(brew --prefix postgresql@17 2>/dev/null)/bin"
  if [ ! -x "${prefix}/pg_isready" ]; then
    prefix="$(dirname "$(command -v pg_isready 2>/dev/null || echo "")")"
  fi
  if [ -z "$prefix" ] || ! "${prefix}/pg_isready" -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
    die "postgres is not accepting connections on 127.0.0.1:5432 (run: brew services start postgresql@17 or use Docker)"
  fi
fi

echo "demo: bringing up postgres (if needed)..."
if command -v docker >/dev/null 2>&1; then
  docker compose -f "$ROOT/dev/compose.yml" up -d >/dev/null 2>&1 || true
fi

if ! "$psql" "$WICKET_DATABASE_URL" -v ON_ERROR_STOP=1 -c 'SELECT 1' >/dev/null 2>&1; then
  echo "demo: database not reachable; running demo-db-reset..."
  bash "$ROOT/scripts/demo-db-reset.sh"
fi

echo "demo: migrating..."
cargo run -q -p wicket-server --manifest-path "$ROOT/Cargo.toml" -- \
  migrate --profile "$WICKET_PROFILE"

echo "demo: bootstrapping demo login (dev SQL; see docs/14-first-run.md)..."
bash "$ROOT/scripts/demo-bootstrap-identity.sh"

SERVER_PID=""
if ! curl -sf "$WICKET_DEMO_BASE_URL/health" >/dev/null 2>&1; then
  echo "demo: starting engine on $WICKET_BIND ..."
  cargo run -q -p wicket-server --manifest-path "$ROOT/Cargo.toml" -- \
    serve --profile "$WICKET_PROFILE" --bind "$WICKET_BIND" &
  SERVER_PID=$!
  for _ in $(seq 1 120); do
    if curl -sf "$WICKET_DEMO_BASE_URL/health" >/dev/null 2>&1; then
      break
    fi
    sleep 0.5
  done
  curl -sf "$WICKET_DEMO_BASE_URL/health" >/dev/null 2>&1 || die "engine failed to start on $WICKET_DEMO_BASE_URL"
else
  echo "demo: engine already listening on $WICKET_DEMO_BASE_URL"
fi

echo "demo: seeding through HTTP API..."
bash "$ROOT/scripts/seed-demo-api.sh"

ITEM_ID=""
if [ -f "$ROOT/dev/demo-seed-item-id" ]; then
  ITEM_ID="$(cat "$ROOT/dev/demo-seed-item-id")"
fi

cat <<EOF

Wicket demo is ready.

  Engine:  $WICKET_DEMO_BASE_URL
  Health:  $WICKET_DEMO_BASE_URL/health
  Login:   username $WICKET_DEMO_USER  password $WICKET_DEMO_PASSWORD

  UI (separate terminal):
    export WICKET_API_ORIGIN=$WICKET_DEMO_BASE_URL
    npm run dev --prefix apps/wicket-web

  Reset database: just db-up && just demo-db-reset && just demo

EOF

if [ -n "$SERVER_PID" ]; then
  echo "demo: engine running in background (pid $SERVER_PID). Stop with: kill $SERVER_PID"
fi

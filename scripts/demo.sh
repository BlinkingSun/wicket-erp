#!/usr/bin/env bash
# One-command local demo: Postgres, migrate, demo login, seed, serve (SPA when Node is present).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/demo-env.sh"

die() {
  echo "demo: $*" >&2
  exit 1
}

# migrate loads Config and would fail if a caller left WICKET_UI_ROOT pointing at
# a missing dist. The demo sets it only after a successful UI build.
unset WICKET_UI_ROOT

listen_port() {
  local bind="${WICKET_BIND:-127.0.0.1:8080}"
  echo "${bind##*:}"
}

engine_up() {
  curl -sf "$WICKET_DEMO_BASE_URL/health" >/dev/null 2>&1
}

spa_up() {
  local ct
  ct="$(curl -s -o /dev/null -w '%{content_type}' "$WICKET_DEMO_BASE_URL/" 2>/dev/null || true)"
  case "$ct" in
    text/html*) return 0 ;;
    *) return 1 ;;
  esac
}

stop_engine() {
  local port pids
  port="$(listen_port)"
  pids="$(lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)"
  if [ -z "$pids" ]; then
    return 0
  fi
  echo "demo: stopping existing engine on port $port (pid $pids) so the SPA can be served from the same origin"
  # shellcheck disable=SC2086
  kill $pids 2>/dev/null || true
  for _ in $(seq 1 40); do
    if ! engine_up; then
      return 0
    fi
    sleep 0.25
  done
  die "engine still listening on $WICKET_DEMO_BASE_URL. Stop it (lsof -tiTCP:$port -sTCP:LISTEN | xargs kill) and re-run: just demo"
}

start_engine() {
  echo "demo: starting engine on $WICKET_BIND ..."
  if [ -n "${WICKET_UI_ROOT:-}" ]; then
    echo "demo: WICKET_UI_ROOT=$WICKET_UI_ROOT"
  fi
  cargo run -q -p wicket-server --manifest-path "$ROOT/Cargo.toml" -- \
    serve --profile "$WICKET_PROFILE" --bind "$WICKET_BIND" &
  SERVER_PID=$!
  for _ in $(seq 1 120); do
    if engine_up; then
      return 0
    fi
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
      die "engine process exited. If port $(listen_port) is in use: lsof -tiTCP:$(listen_port) -sTCP:LISTEN | xargs kill"
    fi
    sleep 0.5
  done
  die "engine failed to start on $WICKET_DEMO_BASE_URL"
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
UI_READY=0
UI_SKIP_REASON=""

if command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  echo "demo: building UI (just ui-install && just ui-build)..."
  if (cd "$ROOT" && just ui-install && just ui-build); then
    if [ -f "$WICKET_DEMO_UI_ROOT/index.html" ]; then
      export WICKET_UI_ROOT="$WICKET_DEMO_UI_ROOT"
      UI_READY=1
    else
      die "UI build did not produce $WICKET_DEMO_UI_ROOT/index.html"
    fi
  else
    die "UI build failed (just ui-install && just ui-build)"
  fi
else
  UI_SKIP_REASON="Node is not installed"
  echo "demo: $UI_SKIP_REASON; skipping the UI. The engine API will still come up."
  echo "demo: install Node 22 or newer and re-run just demo to serve the SPA from the engine origin."
fi

if engine_up; then
  if [ "$UI_READY" -eq 1 ] && ! spa_up; then
    stop_engine
    start_engine
  else
    echo "demo: engine already listening on $WICKET_DEMO_BASE_URL"
  fi
else
  start_engine
fi

if [ "$UI_READY" -eq 1 ] && ! spa_up; then
  die "engine is up but GET $WICKET_DEMO_BASE_URL/ is not the SPA. Stop the process on port $(listen_port) and re-run: just demo"
fi

echo "demo: seeding through HTTP API..."
bash "$ROOT/scripts/seed-demo-api.sh"

ITEM_ID=""
if [ -f "$ROOT/dev/demo-seed-item-id" ]; then
  ITEM_ID="$(cat "$ROOT/dev/demo-seed-item-id")"
fi

cat <<EOF

Wicket demo is ready.

  Database: wicket_demo
  Engine:   $WICKET_DEMO_BASE_URL
  Health:   $WICKET_DEMO_BASE_URL/health
  Login:    username $WICKET_DEMO_USER  password $WICKET_DEMO_PASSWORD
EOF

if [ -n "$ITEM_ID" ]; then
  echo "  Item:     $WICKET_DEMO_BASE_URL/office/items/$ITEM_ID"
fi

if [ "$UI_READY" -eq 1 ]; then
  echo "  SPA:      served from $WICKET_UI_ROOT"
else
  echo "  SPA:      skipped ($UI_SKIP_REASON). Engine API only."
fi

echo "  Reset:    just db-up && just demo-db-reset && just demo"

if [ -n "$SERVER_PID" ]; then
  echo "  Stop:     kill $SERVER_PID"
fi

echo
if [ "$UI_READY" -eq 1 ]; then
  echo "$WICKET_DEMO_BASE_URL/"
else
  echo "$WICKET_DEMO_BASE_URL/health"
fi

#!/usr/bin/env bash
# Seed demo data through the HTTP API (plain-shop). Idempotent.
set -eu
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck disable=SC1091
source "$ROOT/scripts/demo-env.sh"

command -v curl >/dev/null 2>&1 || {
  echo "seed-demo-api: curl is required" >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo "seed-demo-api: jq is required" >&2
  exit 1
}

COOKIE_JAR="$(mktemp "${TMPDIR:-/tmp}/wicket-demo-cookies.XXXXXX")"
cleanup() { rm -f "$COOKIE_JAR"; }
trap cleanup EXIT

BASE="$WICKET_DEMO_BASE_URL"
idem_key() { uuidgen | tr '[:upper:]' '[:lower:]'; }

qty() {
  local amount="$1" unit="$2" dim="$3"
  jq -nc --arg a "$amount" --argjson u "$unit" --arg d "$dim" \
    '{amount:$a, unit:$u, dimension:$d}'
}

login() {
  local resp csrf
  resp="$(curl -sf -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
    -X POST "$BASE/api/v1/identity/login" \
    -H 'content-type: application/json' \
    -H "idempotency-key: $(idem_key)" \
    -d "{\"username\":\"$WICKET_DEMO_USER\",\"password\":\"$WICKET_DEMO_PASSWORD\"}")" \
    || {
      echo "seed-demo-api: login failed (is the engine up? did demo bootstrap run?)" >&2
      exit 1
    }
  csrf="$(printf '%s' "$resp" | jq -r .csrf)"
  if [ -z "$csrf" ] || [ "$csrf" = null ]; then
    echo "seed-demo-api: login response missing csrf" >&2
    exit 1
  fi
  export DEMO_CSRF="$csrf"
}

api_get() {
  curl -sf -b "$COOKIE_JAR" -c "$COOKIE_JAR" \
    -H "x-csrf-token: $DEMO_CSRF" \
    "$BASE$1"
}

api_post() {
  local path="$1" body="$2"
  curl -sf -b "$COOKIE_JAR" -c "$COOKIE_JAR" \
    -X POST "$BASE$path" \
    -H 'content-type: application/json' \
    -H "x-csrf-token: $DEMO_CSRF" \
    -H "idempotency-key: $(idem_key)" \
    -d "$body"
}

api_post_if_match() {
  local path="$1" body="$2" ver="$3"
  curl -sf -b "$COOKIE_JAR" -c "$COOKIE_JAR" \
    -X POST "$BASE$path" \
    -H 'content-type: application/json' \
    -H "x-csrf-token: $DEMO_CSRF" \
    -H "idempotency-key: $(idem_key)" \
    -H "if-match: \"$ver\"" \
    -d "$body"
}

find_item_id() {
  local prefix="$1"
  api_get "/api/v1/items?number_prefix=${prefix}&limit=10" \
    | jq -r --arg n "$prefix" '.data[] | select(.number==$n) | .id' | head -1
}

wait_for_engine() {
  local i
  for i in $(seq 1 60); do
    if curl -sf "$BASE/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
  done
  echo "seed-demo-api: engine not reachable at $BASE/health" >&2
  exit 1
}

wait_for_engine
login

existing="$(find_item_id "MDS-450-M4x12" || true)"
if [ -n "$existing" ]; then
  echo "seed-demo-api: demo data already present (item MDS-450-M4x12 id=$existing)"
  printf '%s\n' "$existing" >"$ROOT/dev/demo-seed-item-id"
  exit 0
fi

screw_body="$(jq -nc \
  --arg n "MDS-450-M4x12" \
  --argjson std "$(qty "0.250000" 1 "Count")" \
  '{
    number: $n,
    revision: "C",
    description: "Cortical bone screw, Ti-6Al-4V ELI, M4 x 12",
    kind: "make",
    stock_uom: 1,
    stock_scale: 0,
    residual_tolerance: "0",
    cost_method: "STANDARD",
    standard: {amount: "0.250000", currency: 840}
  }')"
screw="$(api_post "/api/v1/items" "$screw_body" | jq -r .id)"

bar_body="$(jq -nc \
  '{
    number: "RM-TI-BAR-12",
    revision: "A",
    description: "Titanium bar, stocked in mm of length",
    kind: "buy",
    stock_uom: 2,
    stock_scale: 0,
    residual_tolerance: "0",
    cost_method: "FIFO"
  }')"
bar="$(api_post "/api/v1/items" "$bar_body" | jq -r .id)"

qloc="$(api_post "/api/v1/locations" '{"code":"WH-Q","name":"Quarantine","kind":"warehouse"}' | jq -r .id)"
aloc="$(api_post "/api/v1/locations" '{"code":"WH-A","name":"Available","kind":"warehouse"}' | jq -r .id)"
fg="$(api_post "/api/v1/locations" '{"code":"WH-FG","name":"Finished goods","kind":"warehouse"}' | jq -r .id)"

heat="$(api_post "/api/v1/lots" "$(jq -nc --arg i "$bar" '{item_id:$i, identifier:"HT-ATI-24-8831"}')" | jq -r .id)"
bar_lot="$(api_post "/api/v1/lots" "$(jq -nc --arg i "$bar" \
  '{item_id:$i, identifier:"LOT-BAR-24-4412", heat:"HT-ATI-24-8831", expiry:{value:"2027-09",precision:"month"}}')" \
  | jq -r .id)"

case_lot="$(api_post "/api/v1/lots" "$(jq -nc --arg i "$screw" '{item_id:$i, identifier:"LOT-CASE-48"}')" | jq -r .id)"

case_ids=()
for n in 1 2; do
  pkg="$(api_post "/api/v1/lots/$case_lot/packages" "$(jq -nc \
    --argjson q "$(qty "24" 1 "Count")" \
    --arg label "CASE-$n" \
    '{level:"case", contained:$q, label_ref:$label}')")"
  cid="$(printf '%s' "$pkg" | jq -r .id)"
  case_ids+=("$cid")
  for _ in $(seq 1 24); do
    api_post "/api/v1/lots/$case_lot/packages" "$(jq -nc \
      --arg pid "$cid" \
      --argjson q "$(qty "1" 1 "Count")" \
      '{level:"each", parent_id:$pid, contained:$q}')" >/dev/null
  done
done

api_post "/api/v1/inventory/receipts" "$(jq -nc \
  --arg loc "$qloc" \
  --arg screw "$screw" \
  --arg lot "$case_lot" \
  --arg p0 "${case_ids[0]}" \
  --arg p1 "${case_ids[1]}" \
  '{
    location_id: $loc,
    lines: [
      {item_id: $screw, lot_id: $lot, package_id: $p0},
      {item_id: $screw, lot_id: $lot, package_id: $p1}
    ]
  }')" >/dev/null

api_post "/api/v1/inventory/receipts" "$(jq -nc \
  --arg item "$bar" \
  --arg lot "$bar_lot" \
  --arg loc "$qloc" \
  --argjson q "$(qty "258000" 2 "Length")" \
  '{
    item_id: $item,
    lot_id: $lot,
    location_id: $loc,
    purchase_order: "PO-2024-0841",
    quantity: $q,
    entered: $q,
    unit_cost: {amount: "0.007884", currency: 840}
  }')" >/dev/null

lot_ver="$(api_get "/api/v1/lots/$bar_lot" | jq -r .version)"
api_post_if_match "/api/v1/inventory/releases" "$(jq -nc \
  --arg lot "$bar_lot" \
  --arg from "$qloc" \
  --arg to "$aloc" \
  --argjson q "$(qty "258000" 2 "Length")" \
  '{lot_id:$lot, from_location_id:$from, to_location_id:$to, entered:$q}')" "$lot_ver" >/dev/null

screw_ver="$(api_get "/api/v1/items/$screw" | jq -r .version)"
api_post_if_match "/api/v1/items/$screw/release" '{}' "$screw_ver" >/dev/null

wo="$(api_post "/api/v1/work-orders" "$(jq -nc \
  --arg item "$screw" \
  --argjson q "$(qty "5" 1 "Count")" \
  '{item_id:$item, revision:"C", quantity:$q}')")"
wo_id="$(printf '%s' "$wo" | jq -r .id)"
wo_ver="$(printf '%s' "$wo" | jq -r .version)"
wo="$(api_post_if_match "/api/v1/work-orders/$wo_id/release" '{}' "$wo_ver")"
wo_ver="$(printf '%s' "$wo" | jq -r .version)"
wo="$(api_post_if_match "/api/v1/work-orders/$wo_id/issue" "$(jq -nc \
  --arg from "$aloc" \
  --arg item "$bar" \
  --arg lot "$bar_lot" \
  --argjson q "$(qty "3000" 2 "Length")" \
  '{from_location_id:$from, lines:[{item_id:$item, lot_id:$lot, entered:$q}]}')" "$wo_ver")"
wo_ver="$(printf '%s' "$wo" | jq -r .version)"
done="$(api_post_if_match "/api/v1/work-orders/$wo_id/complete" "$(jq -nc \
  --arg loc "$fg" \
  --argjson q "$(qty "5" 1 "Count")" \
  '{quantity:$q, finished_lot_number:"LOT-WO-1847", location_id:$loc, serial_template:"SN-450-{000000}"}')" "$wo_ver")"
fin_lot="$(printf '%s' "$done" | jq -r '.finished_lot.id')"

api_get "/api/v1/genealogy/trace?from_lot_id=$heat&direction=forward" >/dev/null
api_get "/api/v1/genealogy/trace?from_lot_id=$fin_lot&direction=backward" >/dev/null

printf '%s\n' "$screw" >"$ROOT/dev/demo-seed-item-id"
echo "seed-demo-api: ok (screw item id=$screw, heat lot=$heat, finished lot=$fin_lot)"

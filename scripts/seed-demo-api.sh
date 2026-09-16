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

# Stable keys so a second seed run replays instead of duplicating documents.
DEMO_ON_HAND_RECEIPT_IDEM="00000000-0000-4000-8000-00000000d001"

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

api_post_idem() {
  local path="$1" body="$2" key="$3"
  curl -sf -b "$COOKIE_JAR" -c "$COOKIE_JAR" \
    -X POST "$BASE$path" \
    -H 'content-type: application/json' \
    -H "x-csrf-token: $DEMO_CSRF" \
    -H "idempotency-key: $key" \
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

find_location_id() {
  local code="$1"
  api_get "/api/v1/locations?limit=50" \
    | jq -r --arg c "$code" '.data[] | select(.code==$c) | .id' | head -1
}

find_lot_id() {
  local item_id="$1" identifier="$2"
  api_get "/api/v1/lots?item_id=${item_id}&limit=50" \
    | jq -r --arg id "$identifier" '.data[] | select(.identifier==$id) | .id' | head -1
}

find_completed_work_order_id() {
  local item_id="$1"
  api_get "/api/v1/work-orders?limit=20" \
    | jq -r --arg i "$item_id" \
      '.data[] | select(.item_id==$i and .status=="completed") | .id' | head -1
}

on_hand_qty() {
  local item_id="$1"
  api_get "/api/v1/inventory/on-hand?item_id=${item_id}" | jq -r .on_hand
}

is_positive_decimal() {
  local v="$1"
  awk -v x="$v" 'BEGIN { if (x+0 > 0) exit 0; exit 1 }'
}

write_demo_id_files() {
  local item_id="$1" lot_id="$2" wo_id="$3"
  printf '%s\n' "$item_id" >"$ROOT/dev/demo-seed-item-id"
  printf '%s\n' "$lot_id" >"$ROOT/dev/demo-seed-lot-id"
  printf '%s\n' "$wo_id" >"$ROOT/dev/demo-seed-work-order-id"
}

# Item-level getOnHand only folds lot-less postings; release finished lot for available
# and post one lot-less EA so the inventory tab shows non-zero on hand.
ensure_item_master_inventory() {
  local screw="$1" fin_lot="$2"
  local aloc fg lot_ver status qty_body

  aloc="$(find_location_id "WH-A")"
  fg="$(find_location_id "WH-FG")"
  if [ -z "$aloc" ] || [ -z "$fg" ]; then
    echo "seed-demo-api: demo locations missing (WH-A / WH-FG)" >&2
    exit 1
  fi

  status="$(api_get "/api/v1/lots/$fin_lot" | jq -r .status)"
  if [ "$status" = "quarantine" ]; then
    lot_ver="$(api_get "/api/v1/lots/$fin_lot" | jq -r .version)"
    api_post_if_match "/api/v1/inventory/releases" "$(jq -nc \
      --arg lot "$fin_lot" \
      --arg from "$fg" \
      --arg to "$aloc" \
      --argjson q "$(qty "5" 1 "Count")" \
      '{lot_id:$lot, from_location_id:$from, to_location_id:$to, entered:$q}')" "$lot_ver" >/dev/null
  fi

  if ! is_positive_decimal "$(on_hand_qty "$screw")"; then
    qty_body="$(qty "1" 1 "Count")"
    api_post_idem "/api/v1/inventory/receipts" "$(jq -nc \
      --arg item "$screw" \
      --arg loc "$aloc" \
      --argjson q "$qty_body" \
      '{
        item_id: $item,
        location_id: $loc,
        quantity: $q,
        entered: $q,
        unit_cost: {amount: "0.250000", currency: 840}
      }')" "$DEMO_ON_HAND_RECEIPT_IDEM" >/dev/null
  fi

  if ! is_positive_decimal "$(on_hand_qty "$screw")"; then
    echo "seed-demo-api: on-hand still zero for item $screw" >&2
    exit 1
  fi
}

finalize_demo_seed() {
  local screw="$1" fin_lot="$2" wo_id="$3"
  ensure_item_master_inventory "$screw" "$fin_lot"
  api_get "/api/v1/genealogy/trace?from_lot_id=$fin_lot&direction=backward" >/dev/null
  write_demo_id_files "$screw" "$fin_lot" "$wo_id"
  echo "seed-demo-api: ok (item=$screw, lot=$fin_lot, work_order=$wo_id)"
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

screw="$(find_item_id "MDS-450-M4x12" || true)"
if [ -n "$screw" ]; then
  fin_lot="$(find_lot_id "$screw" "LOT-WO-1847" || true)"
  wo_id="$(find_completed_work_order_id "$screw" || true)"
  if [ -z "$fin_lot" ] || [ -z "$wo_id" ]; then
    echo "seed-demo-api: item present but demo lot/work order missing (reset demo DB?)" >&2
    exit 1
  fi
  echo "seed-demo-api: demo data already present (item MDS-450-M4x12 id=$screw)"
  finalize_demo_seed "$screw" "$fin_lot" "$wo_id"
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

finalize_demo_seed "$screw" "$fin_lot" "$wo_id"

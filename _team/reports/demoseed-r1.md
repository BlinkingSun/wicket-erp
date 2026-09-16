# demoseed-r1

## Changes

- `scripts/seed-demo-api.sh`: after the work-order complete path, release finished lot `LOT-WO-1847` from quarantine and post a stable-idempotency lot-less receipt so `getOnHand` (item id only) is non-zero; write `dev/demo-seed-lot-id` and `dev/demo-seed-work-order-id`; idempotent re-run refreshes the three id files without duplicating documents.
- `docs/14-first-run.md`: points readers at the three id files.
- `dev/.gitignore`: ignore lot and work-order id files.

## Verification (clean `wicket_demo`: drop DB, `just demo-db-reset`, `just demo`)

First run ended with:

```
seed-demo-api: ok (item=01a0a8c4-1927-729b-89e2-bc03a36c330f, lot=01a0a8c4-2238-756e-b025-1e9bbaed550b, work_order=01a0a8c4-2196-7697-b0bb-674cfe3c1de5)
```

### `GET /api/v1/inventory/on-hand?item_id=<demo-seed-item-id>`

```json
{"available":"6.00000000","on_hand":"1.00000000"}
```

### `GET /api/v1/genealogy/trace?from_lot_id=<demo-seed-lot-id>&direction=backward`

```json
{"direction":"backward","nodes":[{"amount":"0","amount_currency":840,"children":[],"edge_quantity":{"amount":"0","dimension":"Count","unit":1},"item":null,"location":"01a0a8c4-19b5-723a-bb91-0ef71a8c6620","lot":"01a0a8c4-2238-756e-b025-1e9bbaed550b","occurred_at":null,"posting":20,"quantity":{"amount":"0","dimension":"Count","unit":1},"serial":null},{"amount":"0","amount_currency":840,"children":[],"edge_quantity":{"amount":"0","dimension":"Count","unit":1},"item":null,"location":"01a0a8c4-19b5-723a-bb91-0ef71a8c6620","lot":"01a0a8c4-2238-756e-b025-1e9bbaed550b","occurred_at":null,"posting":25,"quantity":{"amount":"0","dimension":"Count","unit":1},"serial":null},{"amount":"0","amount_currency":840,"children":[{"amount":"23.652000","amount_currency":840,"children":[],"edge_quantity":{"amount":"3000.00000000","dimension":"Length","unit":2},"item":"01a0a8c4-195c-709a-a697-30b61577e340","location":"01a0a8c4-197d-73ee-a957-bc13dcb64a1a","lot":"01a0a8c4-19fe-738d-8d45-c911503806c0","occurred_at":null,"posting":14,"quantity":{"amount":"3000.00000000","dimension":"Length","unit":2},"serial":null},{"amount":"23.652000","amount_currency":840,"children":[],"edge_quantity":{"amount":"3000.00000000","dimension":"Length","unit":2},"item":"01a0a8c4-195c-709a-a697-30b61577e340","location":"01a0a8c4-197d-73ee-a957-bc13dcb64a1a","lot":"01a0a8c4-19fe-738d-8d45-c911503806c0","occurred_at":null,"posting":14,"quantity":{"amount":"3000.00000000","dimension":"Length","unit":2},"serial":null}],"edge_quantity":{"amount":"0","dimension":"Count","unit":1},"item":null,"location":"01a0a8c4-19b5-723a-bb91-0ef71a8c6620","lot":"01a0a8c4-2238-756e-b025-1e9bbaed550b","occurred_at":null,"posting":19,"quantity":{"amount":"0","dimension":"Count","unit":1},"serial":null},{"amount":"0","amount_currency":840,"children":[{"amount":"1.250000","amount_currency":840,"children":[],"edge_quantity":{"amount":"5.00000000","dimension":"Count","unit":1},"item":"01a0a8c4-1927-729b-89e2-bc03a36c330f","location":"01a0a8c4-19b5-723a-bb91-0ef71a8c6620","lot":"01a0a8c4-2238-756e-b025-1e9bbaed550b","occurred_at":null,"posting":19,"quantity":{"amount":"5.00000000","dimension":"Count","unit":1},"serial":null}],"edge_quantity":{"amount":"0","dimension":"Count","unit":1},"item":null,"location":"01a0a8c4-19b5-723a-bb91-0ef71a8c6620","lot":"01a0a8c4-2238-756e-b025-1e9bbaed550b","occurred_at":null,"posting":24,"quantity":{"amount":"0","dimension":"Count","unit":1},"serial":null}]}
```

### `GET /api/v1/work-orders/<demo-seed-work-order-id>`

```json
{"application_version":"0.1.0 (7098942de641)","completed_at":"2026-09-16T05:50:21.222166+00:00","configuration_version":"1.0.0","id":"01a0a8c4-2196-7697-b0bb-674cfe3c1de5","item_id":"01a0a8c4-1927-729b-89e2-bc03a36c330f","number":"WO-0001","quantity":{"amount":"5.00000000","dimension":"Count","unit":1},"released_at":"2026-09-16T05:50:21.106743+00:00","revision":"C","status":"completed","version":4,"wip_location_id":"01a0a8c4-21b9-7216-97e2-39b9d1243936"}
```

## Second `just demo` (idempotent)

```
demo: engine already listening on http://127.0.0.1:8080
demo: seeding through HTTP API...
seed-demo-api: demo data already present (item MDS-450-M4x12 id=01a0a8c4-1927-729b-89e2-bc03a36c330f)
seed-demo-api: ok (item=01a0a8c4-1927-729b-89e2-bc03a36c330f, lot=01a0a8c4-2238-756e-b025-1e9bbaed550b, work_order=01a0a8c4-2196-7697-b0bb-674cfe3c1de5)

Wicket demo is ready.
```

`just ci` exit 0 (without demo `WICKET_DATABASE_URL` exported).

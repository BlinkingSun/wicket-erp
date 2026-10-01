# UI wrapper contract

Audience: anyone building a Wicket UI. Status: derived from this tree.

**Conforms to:** [ADR 0008](adr/0008-single-tenant.md), [ADR 0009](adr/0009-ui-stack.md), [ADR 0010](adr/0010-one-registry.md), [ADR 0011](adr/0011-openapi-schemas.md); `docs/10-api-conventions.md`; `docs/06-regulatory.md`.

This is the written HTTP contract a competent stranger can build a Wicket UI against without reading Rust. Every factual claim is taken from this repository. Paths in backticks are the source. Where the live engine is ugly, unfinished, or disagrees with `docs/10-api-conventions.md`, this file describes **the engine as it is**.

A paste-in file for a coding agent is [`AGENT-UI-CONTEXT.md`](AGENT-UI-CONTEXT.md).

There is no tenant identifier in any path, header, or body (`docs/10-api-conventions.md` §intro; ADR 0008). One installation is one customer.

---

## 1. The shape of the system

One engine process, one HTTP base URL, many clients.

The engine is a modular monolith: one binary, one PostgreSQL, one transaction boundary (`docs/02-architecture.md` §1; ADR 0001). It binds a TCP address and serves HTTP+JSON. Default bind is `0.0.0.0:8080` (`crates/wicket-server/src/config.rs`, `WICKET_BIND` / `--bind` / TOML `bind`). Public routes live under `/api/v1/` except `GET /health` (`crates/wicket-server/src/capabilities.rs`).

The UI is a **wrapper**: a client of that HTTP surface and nothing else (ADR 0009). It must not link a kernel crate, must not open the database, and must not mint identifiers. That is what the wrapper method buys:

- The same engine is reachable from a browser on the office LAN, a floor tablet, a desktop shell, and later a phone. The engine does not care which.
- A breaking change on the wire breaks every wrapper the same way.
- Regulatory record properties (audit, signatures, ledger) stay in the engine. A screen change cannot change a historical document (ADR 0009: the UI is not a renderer).

The first-party application is `apps/wicket-web/` (TypeScript, React, TanStack Query / Table / Router). ADR 0009 binds **one generated TypeScript client** from the served OpenAPI document. That client is `apps/wicket-web/src/api/generated/openapi.ts`, generated from `crates/wicket-server/tests/fixtures/openapi-document.json` by `just openapi-client`. The first-party screens still perform `fetch` in `apps/wicket-web/src/api/http.ts`.

The engine serves the SPA when `WICKET_UI_ROOT` is set to a directory that contains `index.html` (`docs/12-configuration.md`; `crates/wicket-server/src/http.rs:148-152`). Unset, `ui_root()` is absent and a non-API path, including `index.html`, is 404: API only. CORS headers are ABSENT (no `tower_http::cors`, no `Access-Control-*` in `crates/wicket-server`).

---

## 2. Discovery

Do not hardcode the operation set. Two documents tell a client what this deployment can do.

### 2.1 The capability table

The registry is `crates/wicket-server/src/capabilities.rs`. It is 72 rows: 42 `kernel(` rows in `KERNEL` and 30 `module(` rows in `MODULE` (`table_has_the_mounted_count` at `capabilities.rs:830-834`; `crates/wicket-server/tests/slice.rs:1917`). Each row is an OpenAPI `operationId`, HTTP method, path, permission key (empty string means unauthenticated), and, for state-machine edges, `edge` + `doc_type`.

`crates/wicket-server/src/http.rs` builds the Axum router by walking that table. `crates/wicket-server/src/openapi.rs` builds the served document from the same walk (ADR 0010).

The table is **not** filtered by installation profile. `plain-shop` disables `mod-calibration` (`profiles/plain-shop.toml`) and `regulated-device` enables it (`profiles/regulated-device.toml`), but `approveCalibration` is still mounted on both. Architecture prose says disabling a module makes routes stop resolving (`docs/02-architecture.md` §4). The live router does not unmount them.

### 2.2 The served OpenAPI document

```
GET /api/v1/openapi.json
```

Operation id `getOpenApi`. Unauthenticated (`permission` is `""` in `capabilities.rs`). Returns OpenAPI 3.0.3 (`crates/wicket-server/src/openapi.rs`):

| Member | What is actually there |
|---|---|
| `info.title` | `"Wicket HTTP API"` |
| `info.version` | `CARGO_PKG_VERSION` |
| `paths.{path}.{method}.operationId` | capability `id` |
| `paths.{path}.{method}.x-wicket-permission` | capability `permission` (empty string if unauthenticated) |
| `paths.{path}.{method}.x-wicket-signature` | `{ meaning, permission }` when the profile has a `SignatureEdge::Required` for that `(doc_type, edge)` |
| `paths.{path}.{method}.parameters` | path placeholders, the query list in §4.3, and required headers `Idempotency-Key` / `If-Match` / `X-Wicket-Signature` where the per-id lists in `openapi.rs` say so |
| `paths.{path}.{method}.requestBody` | an `application/json` schema on 28 operations; absent on the rest |
| `paths.{path}.{method}.responses` | 72 of 72 operations are typed per the seam-v2 rule (below). `health` is 200 `text/plain` with a string schema; `logout` is 204 with no `content` and no `200`. Other operations use 200 `application/json` where applicable. Other statuses are bare descriptions |
| `components.schemas` | 113 schemas, including `ErrorEnvelope` |

72 of 72 operations are typed on both `plain-shop` and `regulated-device`. An operation counts as typed when it has a 200 `application/json` schema object and an `application/json` `requestBody` iff the handler parses JSON; `health` counts when 200 carries `text/plain` `{"type":"string"}` and no `application/json`; `logout` counts when 204 has a description and no `content`, `requestBody` is absent, and `responses["200"]` is absent. `served_openapi_carries_wave1_body_schemas` enforces the set equals all 72 operation ids with an empty remainder on each profile. The counts are the committed fixtures `crates/wicket-server/tests/fixtures/openapi-document.json` and `openapi-document-regulated.json` (the served document matches those fixtures, `crates/wicket-server/tests/openapi_document.rs:30-43`). The generated client is `apps/wicket-web/src/api/generated/openapi.ts`.

`setLotStatus` never receives `x-wicket-signature` in the document: `openapi.rs` `required_signature` returns `None` for that id because one handler serves release/hold/reject.

### 2.3 Profiles change the answer

`--profile` / `WICKET_PROFILE` / TOML `profile` selects `plain-shop` or `regulated-device` (`crates/wicket-server/src/config.rs`). A profile is runtime enablement of compiled-in modules, not a second build (`PLAN.md` §1a).

What actually differs on the wire:

| Surface | `plain-shop` | `regulated-device` |
|---|---|---|
| `GET /api/v1/navigation` | `visible`: items, locations, lots, inventory, production, genealogy. `hidden`: validation, iq, calibration | `visible` adds calibration, validation, iq. `hidden`: `[]` |
| Signature gate | `NoSignatures` (`profiles/plain-shop.toml`) | `wicket-esign`, `continuous_session = "off"` |
| Seeded role bundles | operator / admin (`profiles/plain-shop.toml`) | operator / quality / admin (`profiles/regulated-device.toml`) |
| `GET /api/v1/iq/manifest` | hashed module set + `signature_edges` generated from the live registry (`crates/wicket-module/src/kernel.rs` `refresh_signature_edges`) | same shape; Required edges appear when the registry declares them |

`GET /api/v1/navigation` requires `identity.session`. Body:

```json
{ "visible": ["items", "locations", "..."], "hidden": ["calibration"] }
```

Those strings are profile navigation keys, not capability ids and not permission keys (`crates/wicket-server/src/handlers/mod.rs` `navigation`; `profiles/*.toml` `[navigation]`).

A UI that assumes a fixed operation set, a fixed nav list, or a fixed signature requirement is wrong for one of the two supported installs.

---

## 3. Session and identity

Mechanism: `wicket-identity` via `crates/wicket-server/src/session.rs` and `crates/wicket-server/src/handlers/mod.rs` (`login` / `logout`).

### 3.1 Login

```
POST /api/v1/identity/login
Idempotency-Key: <uuid>
Content-Type: application/json
```

```json
{ "username": "mreyes", "password": "<login password>" }
```

No session is required (`permission` is `""`). CSRF is not checked (there is not yet a session to double-submit). `Idempotency-Key` **is** required (`openapi.rs` `requires_idempotency_key` includes `"login"`; `login_inner` calls `idempotency::require_key`).

Response **200**:

```json
{
  "session_id": "<uuid>",
  "principal_id": "<uuid>",
  "display_name": "M. Reyes",
  "csrf": "<uuid-v7 string>"
}
```

Two `Set-Cookie` headers (`login_cookies` / `set_cookie` in `handlers/mod.rs`):

| Cookie | Value | Attributes |
|---|---|---|
| `wicket_session` | `session_id` | `Path=/; SameSite=Lax; HttpOnly`; `Secure` only when request header `X-Forwarded-Proto` is `https` (case-insensitive) |
| `wicket_csrf` | `csrf` | `Path=/; SameSite=Lax`; **not** `HttpOnly` (the browser must read it to send `X-CSRF-Token`); `Secure` under the same `X-Forwarded-Proto` rule |

No `Max-Age` or `Expires` is set on either cookie. The server-side session row expires 12 hours after mint (`now() + interval '12 hours'` in `crates/wicket-identity/src/session.rs`). `load_from_pool` accepts a session only while `expires_at > now()` (`session.rs`). `last_seen_at` is written at login and is not updated on later requests.

The session id is **not** a UUID v7. Identity mints with `gen_random_uuid()` (`wicket-identity/src/session.rs`). The CSRF value is a UUID v7 string (`Uuid::now_v7()` in `login_inner`).

Login does not return the permission snapshot. The snapshot is stored on `server_transient.http_session.permissions` (`session.rs` `store`) and is used only for server-side `HttpSession::allows`.

Invalid credentials, inactive principal, and lockout all map to 401 `UNAUTHENTICATED` (`crates/wicket-server/src/error.rs` `envelope`).

### 3.2 What the client holds afterwards

Two equivalent ways to authenticate a later request (`session_id_from_headers`):

1. **Cookie.** Send `Cookie: wicket_session=<session_id>`. The browser does this automatically when `credentials: "include"` and the request is same-site.
2. **Bearer.** Send `Authorization: Bearer <session_id>`. The token **is** the session UUID. There is no separate API-token resource.

If both are present, Bearer wins for the session id. CSRF is skipped whenever `Authorization` starts with `Bearer ` (`used_cookie` in `session.rs`).

`GET /api/v1/identity/me` (`getOwnProfile`, permission `identity.session`) returns the principal, not the session:

```json
{
  "id": "<uuid>",
  "principal_kind": "User",
  "username": "mreyes",
  "display_name": "M. Reyes",
  "status": "Active",
  "created_at": "2026-03-14T15:02:11Z",
  "deactivated_at": null
}
```

(`principal_body` in `crates/wicket-server/src/handlers/identity.rs`. `principal_kind` is `"User"` / `"Service"` / `"Migration"`; `status` is `"Active"` / `"Inactive"`.)

### 3.3 CSRF

Cookie-authenticated **mutations** must double-submit (`session.rs` `check_csrf`, cited as docs/10 §7).

| Rule | Live behaviour |
|---|---|
| When | `require_mutation` is called, which every mounted mutating handler uses **except login**. |
| Not when | The request has `Authorization` starting with `Bearer `, or there is no `wicket_session` cookie. GET handlers call `require_permission`, which does not check CSRF. |
| Header | `X-CSRF-Token` (read as `x-csrf-token`). |
| Value | Exact match of `HttpSession.csrf` (the login body's `csrf` / cookie `wicket_csrf`). |
| Missing header | 403 `FORBIDDEN`, message `"X-CSRF-Token is required for cookie authentication"`. |
| Mismatch | 403 `FORBIDDEN`, message `"CSRF token mismatch"`. |

A cross-device browser wrapper that uses cookies **must** read `wicket_csrf` (or the login JSON `csrf`) and send `X-CSRF-Token` on every POST, PUT, and PATCH. A native wrapper that uses Bearer must not.

Login itself is a POST without CSRF. Logout is a POST **with** CSRF (cookie) or without (Bearer).

### 3.4 Logout

```
POST /api/v1/identity/logout
Idempotency-Key: <uuid>
```

Permission `identity.session`. Cookie clients send `X-CSRF-Token`. Body may be empty. Response **204**. The server deletes `server_transient.http_session` (`drop_session`). It does **not** emit `Set-Cookie` to clear `wicket_session` or `wicket_csrf`. The client must drop its own cookies / stored bearer.

### 3.5 What is not mounted

`docs/10-api-conventions.md` §7 names session lock, inactivity timeout, and optional OIDC as `wicket-identity` routes under `/api/v1/identity/`. The capability table has login, logout, principals, `/me`, and credential writes. There is no lock route, no inactivity timeout on `load_from_pool`, and no OIDC route. Floor auto-lock is a wrapper concern until a route exists.

The two-person identity reset pair is deliberately not mounted (`handlers/identity.rs` crate comment).

---

## 4. Request and response conventions

### 4.1 Common headers

| Header | Direction | Rule in this tree |
|---|---|---|
| `X-Request-Id` | both | Middleware `extract.rs` `request_id_mw` reads `x-request-id`. If it is a parseable UUID, that value is used; otherwise the server mints a UUID v7. It is echoed on the response as `x-request-id`. |
| `Idempotency-Key` | request | Required on the POST/PUT ids listed in `openapi.rs` `requires_idempotency_key`. Value must parse as a UUID (`idempotency.rs` `require_key`). Missing: 400 `VALIDATION`, `field` `"Idempotency-Key"`. Replay same key + same body hash: original status and body. Same key + different body: 409 `IDEMPOTENCY_CONFLICT`. |
| `If-Match` | request | Required on the ids in `openapi.rs` `requires_if_match`. Format: quoted integer, e.g. `If-Match: "3"`. Quotes are stripped. `*` is 400 `VALIDATION`. Stale: 409 `CONFLICT`, `field` `"version"` (`extract.rs` `require_if_match` / `check_version`). |
| `X-Wicket-Signature` | request | UUID of a minted signature. Read as `x-wicket-signature` (`handlers/mod.rs` `bind_esign_header`, `required_edge_token`). Declared required in OpenAPI only when `x-wicket-signature` is stamped. |
| `X-CSRF-Token` | request | See §3.3. |
| `Authorization` | request | `Bearer <session uuid>`. |
| `X-Forwarded-Proto` | request | `https` (case-insensitive) causes `Secure` on login cookies. |
| `X-Forwarded-For` | request | First comma-separated hop stored as `WriteContext.source_ip`. |
| `User-Agent` | request | Truncated to 80 chars as `client_app`. |
| `WWW-Authenticate` | response | The `Error` `IntoResponse` impl would set `Bearer, Cookie` on 401. Mounted handlers return `envelope::error_response`, which does **not** set this header. |

`docs/10-api-conventions.md` §4.2 says every POST carries `Idempotency-Key`. The live list is per-operation. `esignChallenge` is POST, is a mutation (CSRF applies), and is **not** in `requires_idempotency_key`.

### 4.2 JSON and identifiers

JSON keys are `snake_case`. Clients never mint durable ids: a client-supplied `id` on item create / work-order create / principal create is 400 `VALIDATION` on `id`.

Wire quantity (`crates/wicket-server/src/wire.rs` `QuantityBody`; `wicket_core::AnyQuantity` uses `rust_decimal::serde::str` for `amount`):

```json
{ "amount": "258000.00000000", "unit": 2, "dimension": "Length" }
```

`dimension` values from `parse_dim`: `Count`, `Length`, `Mass`, `Time`, `Volume`, `Area`. `unit` is a catalog `UnitId` (JSON number). A JSON number for `amount` fails serde because the field is a string.

Wire money (`MoneyBody`):

```json
{ "amount": "2034.000000", "currency": 840 }
```

`currency` is ISO 4217 numeric.

Expiry (`modules/lots/src/api.rs` `ExpiryWire`):

```json
{ "value": "2027-09", "precision": "month" }
```

`precision` is `day` / `month` / `year` (`ExpiryPrecision` `rename_all = "lowercase"`). The field is `value`, not `date`.

Enumeration casing is **not** uniform. `DimensionKind` serializes as `"Count"`. Item kind/status use lowercase (`"make"`, `"released"`). Location kind uses the SQL label (`"warehouse"`). Lot status uses lowercase (`"quarantine"`). Work-order status uses snake_case (`"in_process"`). Job state uses the Rust variant name (`"Succeeded"`). Do not guess; copy the field.

Actor is the session. A body field `actor_id` on a receipt is 400 `VALIDATION` on `actor_id`. Time of record is server-stamped.

### 4.3 Lists

Envelope (`crates/wicket-server/src/envelope.rs` `ListBody`):

```json
{ "data": [], "next_cursor": "01932c5a-...", "has_more": false }
```

`next_cursor` is `null` when `has_more` is false. An empty first page is 200 with empty `data`, not 404.

Query `limit`: optional. When omitted, item and work-order lists default to 50 (`modules/items/src/store.rs` `DEFAULT_LIMIT`; `modules/production_min/src/store.rs`). When present, it must be an integer in `1..=200` or the handler returns 400 `VALIDATION` on `limit` (`parse_limit`). OpenAPI advertises the same range (`openapi.rs` `limit_schema`).

Query `cursor`: the client sends back exactly `next_cursor`. Item and work-order handlers parse it as a UUID (`parse_uuid` on `cursor`). It is the last id of the previous page, not a signed blob.

Query parameters actually declared in `openapi.rs` `query_parameters`:

| operationId | Query |
|---|---|
| `listItems` | `limit`, `cursor`, `kind`, `status`, `number_prefix` |
| `listLocations`, `listLots` | `limit`, `cursor` |
| `listLocationTree` | `include_inactive` (boolean, default false; handler also accepts `"1"` / `"0"`) |
| `listWorkOrders` | `limit`, `cursor`, `status` |
| `getOnHand` | `item_id` (required UUID), `location_id`, `lot_id` |
| `traceGenealogy` | `from_lot_id` (required UUID), `direction` (`forward` / `backward` / `both`, default `forward`) |
| `listCustomFieldDefinitions` | `entity` (required) |

`sort` is specified in `docs/10-api-conventions.md` §2.5 and is not implemented: no handler reads `sort`, and OpenAPI does not declare it.

These list operations return the envelope shape but do not paginate:

- `listPackages` — `next_cursor: null`, `has_more: false` always (`handlers/mod.rs`).
- `listCustomFieldDefinitions` — same.
- `listPrintTemplates` — `ListBody` with `next_cursor: None`, `has_more: false`.
- `listSerials` — the module can paginate; the HTTP handler calls it with `Some(200)` and `None` cursor and exposes no query params.

### 4.4 Errors

Every error body (`envelope.rs` `ErrorBody`):

```json
{
  "error": {
    "code": "VALIDATION",
    "message": "limit must be between 1 and 200",
    "field": "limit",
    "request_id": "01932c5a-8b10-7001-8000-00000000000d"
  }
}
```

`field` is `null` when the error is not about one field. `request_id` is echoed in `X-Request-Id` via `error_response`.

Codes actually constructed in `crates/wicket-server/src/error.rs` and helpers:

| `code` | HTTP | Where |
|---|---|---|
| `VALIDATION` | 400 | `Error::validation`; identity/module mapping |
| `UNAUTHENTICATED` | 401 | missing/expired session; bad login |
| `FORBIDDEN` | 403 | missing permission; CSRF |
| `NOT_FOUND` | 404 | missing resource |
| `CONFLICT` | 409 | stale version; consumed signature; already reversed |
| `IDEMPOTENCY_CONFLICT` | 409 | key reused with a different body |
| `SIGNATURE_REQUIRED` | 401 or 403 | missing token → 401; signer not permitted / other invalid → 403 (`from_signature`) |
| `SIGNATURE_NO_PROVIDER` | 409 | `SignatureError::NoProvider` |
| `TIMEOUT` | 504 | `limits_mw` after 30 s. The middleware passes request id `"timeout"` (not a UUID). |
| `REFUSED` | 409 | location on-hand / protected (`map_locations_err`) |
| `INTERNAL` | 500 | unmapped crate errors |

`RATE_LIMITED` and `PAYLOAD_TOO_LARGE` are named in `docs/10-api-conventions.md` and in module-local OpenAPI sketches. `RATE_LIMITED` is never emitted (`AGENTS.md`; `GOALS.md` AG-9). Body size is capped at 1 MiB (`http.rs` `DefaultBodyLimit::max(1024 * 1024)`); the 413 body is Axum's, not verified as `ErrorEnvelope`.

### 4.5 Success statuses the handlers actually set

Handlers pick 200, 201, or 204 explicitly (`json_status`, `StatusCode::NO_CONTENT`). OpenAPI lists `201` on all 72 operations and `200` on every operation except `logout` (which has no `200`; see §2.2). Logout, principal rename/deactivate/credential writes return 204 and an empty body.

`GET /health` returns the crate version as **plain text**, not JSON (`handlers/kernel.rs` `health`).

---

## 5. Permissions

The UI should hide what the user cannot do instead of discovering it via 403. The engine does not currently give the UI a permission list.

What exists:

1. **Server-side snapshot.** At login, `granted_permissions` walks `KNOWN_PERMISSIONS` in `session.rs` and stores the keys this principal holds. Every mutating and most GET handlers call `session.allows(permission)` via `require_permission` / `require_mutation`. Missing key → 403 `FORBIDDEN`, message `missing permission {key}`.
2. **OpenAPI `x-wicket-permission`.** Per operation, the permission string the handler will check. Empty string means unauthenticated. A wrapper can intersect this with a permission list **if it had one**.
3. **`GET /api/v1/navigation`.** Profile-level module visibility, not per-user RBAC. Two users on `regulated-device` see the same `visible` array even if one is `operator` and one is `admin`.

What does not exist:

- Login body does not include `permissions`.
- `GET /api/v1/identity/me` does not include `permissions`.
- There is no `GET /api/v1/session` / introspection route (ADR 0010 step 6 is ABSENT).

`KNOWN_PERMISSIONS` (`session.rs`) is the snapshot vocabulary. A key that a module declares but that list omits is never snapshotted, so `allows` is false even if RBAC would grant it on a live check. Wrappers must not treat seeded bundles in `profiles/*.toml` as an API.

Until an introspection route exists, a wrapper can (a) hide by profile navigation, (b) hide by 403, or (c) fail closed and show only what navigation names. It cannot honestly hide per-user.

---

## 6. Long-running work

Genealogy trace is the template. `DESIGN.md` §2: planning, genealogy traversal, and report rendering are background jobs with visible progress.

### 6.1 Request

```
GET /api/v1/genealogy/trace?from_lot_id=<uuid>&direction=forward
```

Permission `genealogy.view`. No idempotency key (GET). CSRF not required.

The HTTP handler (`handlers/mod.rs` `trace_inner`) accepts only `from_lot_id` (required) and `direction` (`forward` default, or `backward` / `both`). The genealogy crate can start from a serial or a posting and can cap depth (`modules/genealogy/src/domain.rs` `TraceOrigin`, `TraceRequest`). Those query names appear in the **module's** unused `openapi_document` (`modules/genealogy/src/api.rs`) and do **not** appear in the served document (`openapi.rs` `query_parameters`). Sending `lot` / `serial` / `posting` / `depth` / `format` does nothing.

ADR 0009 notes this gap: `traceGenealogy` needs widening to `serial` / `lot` / `posting`. Human-identifier lookup (scan a part number, not a UUID) is also ABSENT.

### 6.2 Inline versus job

`modules/genealogy/src/store.rs` `trace`:

- If a cache hit exists, return `TraceOutcome::Inline`.
- Count unique postings. Default inline threshold is 32, overridable with `WICKET_GENEALOGY_INLINE_MAX` (`domain.rs` `DEFAULT_INLINE_MAX_POSTINGS` / `INLINE_MAX_ENV`; `docs/12-configuration.md`).
- If the forest is larger than the threshold (or the threshold is 0), enqueue job kind `genealogy.trace` and return `TraceOutcome::Accepted { job_id, result_url }`.
- `result_url` is a **path**: `/api/v1/genealogy/jobs/{id}`.

The module helper `trace_http_status` would return 202 for Accepted (`modules/genealogy/src/api.rs`). The **HTTP handler does not use it**. Both outcomes go through `Json(v).into_response()`, which is **200**. A client that branches on HTTP 202 will never see a job.

Distinguish by body:

**Job (still HTTP 200):**

```json
{
  "job_id": "01932c5a-8b10-7001-8000-0000000000aa",
  "result_url": "/api/v1/genealogy/jobs/01932c5a-8b10-7001-8000-0000000000aa"
}
```

**Inline, one direction** (`TraceBody::One` — untagged):

```json
{
  "direction": "forward",
  "nodes": [ { "posting": 1, "item": "...", "lot": "...", "serial": null, "location": null, "quantity": { "amount": "1.0", "unit": 1, "dimension": "Count" }, "amount": "0.00", "amount_currency": 840, "occurred_at": null, "edge_quantity": { "amount": "1.0", "unit": 1, "dimension": "Count" }, "children": [] } ]
}
```

**Inline, both** (`TraceBody::Both`):

```json
{
  "backward": { "direction": "backward", "nodes": [] },
  "forward": { "direction": "forward", "nodes": [] }
}
```

`TreeNode` fields: `posting` (i64), `item`, `lot`, `serial`, `location` (UUIDs or null), `quantity` and `edge_quantity` (`AnyQuantity`), `amount` (decimal **string**, not a `MoneyWire` object), `amount_currency` (number), `occurred_at`, `children` (nested `TreeNode`). This is not the `root` / `nodes` / `edges` sketch in `docs/10-api-conventions.md` §9.6.

### 6.3 Poll

```
GET /api/v1/genealogy/jobs/{id}
```

Permission `genealogy.view`. 404 if missing. Body is `wicket_jobs::JobStatus` (`crates/wicket-jobs/src/queue.rs`):

| Field | Type on the wire |
|---|---|
| `id` | UUID string (`JobId` newtype over `Identifier`) |
| `kind` | `"genealogy.trace"` |
| `payload` | JSON object |
| `state` | `"Queued"` / `"Running"` / `"Succeeded"` / `"Failed"` / `"Cancelled"` (serde default of the Rust enum; **not** the lowercase SQL labels) |
| `attempts`, `max_attempts` | integers (`max_attempts` default 25) |
| `run_after` | RFC 3339 |
| `progress_pct` | 0–100 (`i16`) |
| `progress_note` | string or null |
| `result` | the `TraceBody` JSON when `Succeeded`; otherwise null |
| `last_error` | string or null |

Poll until `Succeeded` (render `result`) or `Failed` / `Cancelled` (show `last_error`). `progress_pct` / `progress_note` are the visible progress `DESIGN.md` requires. Do not spin with an unexplained spinner (`DESIGN.md` §9).

A naive client that always treats the trace response as a tree, or that waits for 202, is wrong.

---

## 7. Electronic signatures

Cite: `docs/06-regulatory.md` §3.4–3.5; `docs/10-api-conventions.md` §5; ADR 0005. Getting this wrong is a regulatory problem.

Wicket does not implement the 21 CFR 11.200(a)(1)(i) continuous-session relaxation (`docs/06-regulatory.md` §3.4; `research/decisions/audit-persistence.md` §9). A session cookie on a shared floor tablet is not a component "designed to be used only by the individual". `regulated-device` sets `continuous_session = "off"`. Every signing uses all identification components every time.

The login session is **not** a signing component. The signing credential is separable from the login credential (`docs/06-regulatory.md` §3.3; PLAN invariant 14). Using the session password as the signing secret is rejected by `wicket-esign` as validation on identification.

### 7.1 What the client must show

`DESIGN.md` §6: every state-changing action that requires an electronic signature must say so **before** the user commits, and the dialog must state the meaning in plain words.

11.50(a) requires three things associated with the signing, in any human-readable form of the record (`docs/06-regulatory.md` §3.4): printed name of the signer, date and time, and meaning. Snapshot the printed name; do not join to a live user table at render time. `signed_at` is server time plus the signer's IANA zone (`signed_at_zone`, `signed_at_local` on the manifestation).

The UI may preview. The archival copy is server-rendered PDF (`wicket-print`; ADR 0009). The UI must not be the record.

### 7.2 Challenge, then mint, then transition

**Challenge** (what to collect):

```
POST /api/v1/esign/challenges
```

Permission `identity.session`. CSRF applies. `Idempotency-Key` is **not** required. Body empty. 200, `wicket_esign::Challenge` (`crates/wicket-esign/src/session.rs`):

```json
{
  "components_required": ["code", "secret"],
  "signing_session_expires_at": null,
  "credential_kind": "signing_password"
}
```

With continuous session off, `components_required` is always `code` and `secret`. Collect both. Do not skip the dialog because the user is logged in.

**Mint:**

```
POST /api/v1/esign/signatures
Idempotency-Key: <uuid>
```

```json
{
  "meaning": "Released",
  "reason": "optional free text",
  "record": {
    "table": "sm.instance",
    "id": "<uuid>",
    "version": 3
  },
  "identification": {
    "code": "MREYES",
    "secret": "<signing password, not the login password>"
  },
  "doc_type": "production"
}
```

Field names: `meaning`, `reason` (optional), `record.table`, `record.id`, `record.version`, `identification.code`, `identification.secret`, `doc_type` (optional) (`EsignMintBody` in `handlers/mod.rs`). Response **201** wrapping `Manifestation` (`crates/wicket-esign/src/read.rs`):

```json
{
  "signature": {
    "id": "<uuid>",
    "signer_id": "<uuid>",
    "printed_name": "M. Reyes",
    "meaning": "Released",
    "reason": null,
    "signed_at": "2026-03-14T15:02:11Z",
    "signed_at_zone": "America/New_York",
    "signed_at_local": "2026-03-14T11:02:11-04:00",
    "record": { "table": "sm.instance", "doc_type": "production", "id": "<uuid>", "version": 3 },
    "record_content_hash": "<hex sha-256>",
    "credential_kind": "signing_password",
    "components_used": ["code", "secret"],
    "superseded": false,
    "superseded_by_version": null
  }
}
```

Show `printed_name`, `meaning`, `signed_at` / `signed_at_local` to the user. `docs/10` §5.3 is a subset of this object; the live type also has `reason`, zone fields, `doc_type`, `credential_kind`, `components_used`, `superseded`.

**Transition** carries the minted id, not the secret:

```
POST /api/v1/work-orders/{id}/release
Idempotency-Key: <uuid>
If-Match: "3"
X-Wicket-Signature: <signature.id>
X-CSRF-Token: <csrf>
```

No signature material in the body. Meaning on the mint must match the edge. Failed mints are security events, not business audit rows (`docs/10` §5.2).

`GET /api/v1/esign/signatures/{id}` returns the same manifestation (permission `identity.session`). `GET /api/v1/esign/signatures/{id}/bundle` is the archival bundle (permission `esign.bundle.read`).

### 7.3 When the gate refuses

`plain-shop` binds `NoSignatures`. A Required edge does not skip the check: 409 `SIGNATURE_NO_PROVIDER` (`error.rs` `from_signature`; `docs/10` §5.4 as amended by the live crate). The row is unchanged.

Missing `X-Wicket-Signature` on a Required edge: 401 `SIGNATURE_REQUIRED` (`SignatureError::Invalid("missing token")`). Signer lacks the snapshotted permission: 403 `SIGNATURE_REQUIRED`. Consumed token: 409 `CONFLICT`.

`docs/10-api-conventions.md` §5.4 still says `POST /api/v1/esign/signatures` does not exist until Wave 2b and calling it is 404. That sentence is stale. The route is mounted (`esignMint` in `capabilities.rs`).

The signing identifier the user types is **not** taken from the session username automatically; the client must collect `identification.code`.

---

## 8. Packaging

ADR 0009 already chose the stack. This section does not re-litigate it.

**One application, three interaction modes** (office, shop floor, quality/planning) over one component library and one token set. Not three codebases. Documents that must archive are server-rendered PDF; the application never prints the DOM.

**Tauri v2** is the desktop/mobile shell technology for macOS, Linux, Windows, Android, and iOS. It is **not** required to use the product. A browser on the LAN is a complete client for v1. Scanners present as keyboards. The shell's remaining jobs, when a shop wants them: an icon and a window that is not browser chrome; locking a floor terminal down more tightly than a kiosk tab; later, hardware a browser cannot reach (serial gage, label printer, offline cache).

### 8.1 Base URL

| Environment | How the client finds the engine |
|---|---|
| First-party Vite dev | Browser origin is the Vite server. `/api` is proxied to `WICKET_API_ORIGIN` or `http://127.0.0.1:8080` (`apps/wicket-web/vite.config.ts`). `VITE_API_BASE` defaults to `""` (`apps/wicket-web/src/api/http.ts`), so `fetch` is same-origin. |
| Engine bind | `WICKET_BIND` / `--bind` / TOML `bind`, default `0.0.0.0:8080` (`config.rs`). |
| Production browser | `WICKET_UI_ROOT` unset: the engine does not serve `index.html` (API only). Set: the engine serves the SPA from that directory (`http.rs:148-152`; `docs/12-configuration.md`). CORS is ABSENT. A reverse proxy that serves the UI and `/api/v1` from one origin still works with cookies. |
| Native / Tauri | Must be configured with the engine origin. LAN discovery (mDNS, USB, QR) is ABSENT. |
| Other-device browser | A page whose origin is not the engine cannot `fetch` it: no `Access-Control-Allow-Origin`. |

### 8.2 What changes per platform

| Concern | Browser (office / floor kiosk) | Tauri desktop | Android / iOS (later, same web UI) |
|---|---|---|---|
| Base URL | Same-origin via proxy, or operator-configured origin behind a reverse proxy | Operator-configured; persist in app settings. Discovery ABSENT | Same as desktop. Deep links ABSENT |
| TLS / certificates | LAN often HTTP (`docs/10` §7; `docs/02-architecture.md` §6). `Secure` cookies are off unless `X-Forwarded-Proto: https`. Custom CA / pinning ABSENT | Same. System webview trust store. Pinning ABSENT | Same |
| Session storage | `wicket_session` HttpOnly cookie + `wicket_csrf` readable cookie. `credentials: "include"` | Prefer `Authorization: Bearer <session_id>` so CSRF is skipped (`used_cookie`). Persist `session_id` in OS-appropriate secret storage; do not write it to a world-readable file. Cookie jar in a webview also works if `X-CSRF-Token` is sent | Same as desktop. Platform keychain. Session id is a 12-hour credential |
| CSRF | Required on mutations if cookies are used | Not applicable for Bearer | Not applicable for Bearer |
| Deep links | ABSENT | ABSENT | ABSENT |
| Offline | ABSENT (ADR 0009 names an offline floor cache as a future Tauri job) | ABSENT | ABSENT |

TLS is recommended and not assumed (`docs/10` §7). Bearer tokens on HTTP are visible on the wire, same as cookies.

Floor terminals: auto-lock is named in `docs/02-architecture.md` §8 and is not an HTTP route. A kiosk wrapper that locks the screen on idle is doing work the engine does not yet do.

---

## 9. Capability catalog

Source: `crates/wicket-server/src/capabilities.rs`. Kind `T` = `Transition` (expects `If-Match` only when the operation is also in `requires_if_match`). Permission `""` = unauthenticated.

Idempotency (`I`) and If-Match (`M`) columns are the live lists in `openapi.rs`, not "all POST" / "all Transition".

### Kernel

| operationId | Method | Path | Permission | I | M |
|---|---|---|---|---|---|
| `health` | GET | `/health` | | | |
| `getOpenApi` | GET | `/api/v1/openapi.json` | | | |
| `getValidationManifest` | GET | `/api/v1/iq/manifest` | `validation.manifest.read` | | |
| `exportAudit` | GET | `/api/v1/audit` | `audit.export` | | |
| `getNavigation` | GET | `/api/v1/navigation` | `identity.session` | | |
| `login` | POST | `/api/v1/identity/login` | | yes | |
| `logout` | POST | `/api/v1/identity/logout` | `identity.session` | yes | |
| `createPrincipal` | POST | `/api/v1/identity/principals` | `identity.manage` | yes | |
| `getPrincipal` | GET | `/api/v1/identity/principals/{id}` | `identity.manage` | | |
| `renamePrincipal` | POST | `/api/v1/identity/principals/{id}/rename` | `identity.manage` | yes | |
| `deactivatePrincipal` | POST | `/api/v1/identity/principals/{id}/deactivate` | `identity.manage` | yes | |
| `resetLoginCredential` | POST | `/api/v1/identity/principals/{id}/login-credential` | `identity.manage` | yes | |
| `getOwnProfile` | GET | `/api/v1/identity/me` | `identity.session` | | |
| `changeOwnLoginCredential` | POST | `/api/v1/identity/me/login-credential` | `identity.session` | yes | |
| `setOwnSigningCredential` | POST | `/api/v1/identity/me/signing-credential` | `identity.session` | yes | |
| `approveCalibration` | POST | `/api/v1/calibration/certificates/{id}/approve` | `calibration.approve` | yes | yes |
| `esignChallenge` | POST | `/api/v1/esign/challenges` | `identity.session` | | |
| `esignMint` | POST | `/api/v1/esign/signatures` | `identity.session` | yes | |
| `getEsignSignature` | GET | `/api/v1/esign/signatures/{id}` | `identity.session` | | |
| `getEsignBundle` | GET | `/api/v1/esign/signatures/{id}/bundle` | `esign.bundle.read` | | |
| `defineCustomField` | POST | `/api/v1/customfields/definitions` | `customfields.define` | yes | |
| `listCustomFieldDefinitions` | GET | `/api/v1/customfields/definitions` | `customfields.view` | | |
| `retireCustomField` | POST | `/api/v1/customfields/definitions/{id}/retire` | `customfields.retire` | yes | yes |
| `setItemCustomFields` | PUT | `/api/v1/items/{id}/custom-fields` | `customfields.set` | yes | |
| `getItemCustomFields` | GET | `/api/v1/items/{id}/custom-fields` | `customfields.view` | | |
| `createDocument` | POST | `/api/v1/documents` | `documents.edit` | yes | |
| `getDocument` | GET | `/api/v1/documents/{id}` | `documents.view` | | |
| `createDocumentRevision` | POST | `/api/v1/documents/{id}/revisions` | `documents.edit` | yes | |
| `submitDocument` | POST | `/api/v1/documents/{id}/submit` | `documents.edit` | yes | yes |
| `approveDocument` | POST | `/api/v1/documents/{id}/approve` | `documents.approve` | yes | yes |
| `listPrintTemplates` | GET | `/api/v1/print/templates` | `print.templates` | | |
| `renderPrint` | POST | `/api/v1/print/render` | `print.render` | yes | |
| `archivePrint` | POST | `/api/v1/print/archive` | `print.archive` | yes | |
| `releaseFromQuarantine` | POST | `/api/v1/inventory/releases` | `lots.release` | yes | yes |
| `reverseIssue` | POST | `/api/v1/inventory/reversals` | `inventory.adjust` | yes | |

### Mounted modules

| operationId | Method | Path | Permission | I | M |
|---|---|---|---|---|---|
| `listItems` | GET | `/api/v1/items` | `items.view` | | |
| `createItem` | POST | `/api/v1/items` | `items.edit` | yes | |
| `getItem` | GET | `/api/v1/items/{id}` | `items.view` | | |
| `updateItem` | PATCH | `/api/v1/items/{id}` | `items.edit` | yes | yes |
| `releaseItem` | POST | `/api/v1/items/{id}/release` | `items.release` | yes | yes |
| `listLocations` | GET | `/api/v1/locations` | `locations.view` | | |
| `createLocation` | POST | `/api/v1/locations` | `locations.edit` | yes | |
| `getLocation` | GET | `/api/v1/locations/{id}` | `locations.view` | | |
| `listLocationTree` | GET | `/api/v1/locations/tree` | `locations.view` | | |
| `deactivateLocation` | POST | `/api/v1/locations/{id}/deactivate` | `locations.edit` | yes | yes |
| `listLots` | GET | `/api/v1/lots` | `lots.view` | | |
| `createLot` | POST | `/api/v1/lots` | `lots.edit` | yes | |
| `getLot` | GET | `/api/v1/lots/{id}` | `lots.view` | | |
| `setLotStatus` | POST | `/api/v1/lots/{id}/status` | `lots.release` | yes | yes |
| `listPackages` | GET | `/api/v1/lots/{id}/packages` | `lots.view` | | |
| `createPackage` | POST | `/api/v1/lots/{id}/packages` | `lots.edit` | yes | |
| `listSerials` | GET | `/api/v1/lots/{id}/serials` | `lots.view` | | |
| `createSerials` | POST | `/api/v1/lots/{id}/serials` | `lots.edit` | yes | |
| `createReceipt` | POST | `/api/v1/inventory/receipts` | `inventory.receive` | yes | |
| `createCount` | POST | `/api/v1/inventory/counts` | `inventory.count` | yes | |
| `getOnHand` | GET | `/api/v1/inventory/on-hand` | `inventory.view` | | |
| `listWorkOrders` | GET | `/api/v1/work-orders` | `production.view` | | |
| `createWorkOrder` | POST | `/api/v1/work-orders` | `production.create` | yes | |
| `getWorkOrder` | GET | `/api/v1/work-orders/{id}` | `production.view` | | |
| `releaseWorkOrder` | POST | `/api/v1/work-orders/{id}/release` | `production.release` | yes | yes |
| `issueWorkOrder` | POST | `/api/v1/work-orders/{id}/issue` | `production.issue` | yes | yes |
| `completeWorkOrder` | POST | `/api/v1/work-orders/{id}/complete` | `production.complete` | yes | yes |
| `traceGenealogy` | GET | `/api/v1/genealogy/trace` | `genealogy.view` | | |
| `getImpact` | GET | `/api/v1/genealogy/impact/{lot}` | `genealogy.view` | | |
| `getGenealogyJob` | GET | `/api/v1/genealogy/jobs/{id}` | `genealogy.view` | | |

Path placeholders `{id}` and `{lot}` are UUIDs in the served document (`path_parameters`).

Request and response **schemas** are not in the served document (ADR 0011). Verified handler bodies for the operations a first screen presses:

**Item GET/list** (`modules/items/src/api.rs` `ItemBody`): `id`, `number`, `revision`, `description`, `kind` (`make`/`buy`/`service`/`phantom`), `stock_uom` (number), `stock_scale`, `residual_tolerance` (decimal string), `cost_method` (`FIFO`/`MOVING_AVG`/`STANDARD`), `status` (`draft`/`released`/`obsolete`), `version`, `application_version`, `configuration_version`, `created_at`. This is not the `stocking_quantity` / `type` / `lifecycle_status` sketch in `docs/10` §9.1.

**Item create** (`ItemCreate`): `number`, `revision`, `description`, optional `kind` or `type` (alias), `stock_uom`, optional `stock_scale`, `residual_tolerance`, `cost_method`, `standard` (`MoneyBody`), `id` (must be omitted). 201 + `ItemBody`.

**Work order GET** (`wo_json`): `id`, `number` (null until release), `item_id`, `quantity` (`QuantityBody`), `status` (`draft`/`released`/`in_process`/`completed`/`cancelled`), `revision`, `wip_location_id`, `version`, `application_version`, `configuration_version`, `released_at`, `completed_at`.

**Work order create**: `item_id`, `quantity` (`QuantityBody`), `revision`. 201.

**Work order complete** body: `quantity`, optional `scrap`, optional `finished_lot_number`, `location_id`, optional `serial_from` / `serial_template`. Success includes `finished_lot.id` and `group_id`; it does not include the serial range object in `docs/10` §9.5.

**Lot GET** (`LotBody`): `id`, `identifier`, `item_id`, `supplier_lot`, `heat`, `expiry` (`{value, precision}`), `cert_ref`, `status` (`quarantine`/`available`/`hold`/`rejected`), `udi_device_identifier`, `version`, `created_at`.

**Receipt create**: `item_id`, `lot_id`, `location_id`, `purchase_order`, `quantity` and/or `entered` (`QuantityBody`), `unit_cost` (`MoneyBody`); or a `lines` array. Rejects `actor_id`. 201 `DocumentBody`: `id`, `kind`, `status`, `reference`, `posted_group_id`, `version`, `lines[]` with `entered` / `canonical` quantities.

**Location create/get**: `id`, `code`, `name`, `kind` (`warehouse`/`area`/`bin`/`wip` from `as_sql`), `version`. Create body: `code`, `name`, optional `kind` (default `warehouse`).

---

## 10. House rules for any wrapper

From `DESIGN.md` (binding on every interface lane) and this contract:

- Dark mode is the default. Light mode is opt-in only if a user asks.
- No emojis anywhere: not in navigation, status, empty states, or toasts.
- Never display data the engine did not return. No lorem ipsum, no invented balances, no fake traces, no placeholder part numbers on a live screen.
- Status is a word plus a shape; color is never the only signal.
- Part numbers, lot numbers, serials, and quantities are monospace and tabular-figure aligned.
- Floor targets at least 64×64 px with 16 px separation. Floor screens have no navigation rail.
- Signature meaning is stated before commit (§7).
- Archival documents come from `renderPrint` / `wicket-print`, not from `window.print`.
- Tokens, never inline hex (`DESIGN.md` §3 / §10).

---

## 11. Staleness

If the contract has moved, re-read, in this order:

1. `crates/wicket-server/src/capabilities.rs` — operation ids, methods, paths, permissions.
2. `crates/wicket-server/src/openapi.rs` — parameters, idempotency / If-Match / signature headers, served document shape.
3. `crates/wicket-server/src/session.rs` and `handlers/mod.rs` (`login`, `logout`, `check_csrf`) — cookies, bearer, CSRF.
4. `crates/wicket-server/src/envelope.rs` and `error.rs` — envelopes and codes.
5. `modules/genealogy/src/store.rs` and `handlers/mod.rs` `genealogy_trace` / `get_genealogy_job` — job vs inline.
6. `docs/10-api-conventions.md` — convention; where it disagrees with the files above, the code wins.
7. This file and `docs/AGENT-UI-CONTEXT.md`.

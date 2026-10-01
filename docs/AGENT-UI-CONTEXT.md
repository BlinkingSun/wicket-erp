# AGENT-UI-CONTEXT — build a Wicket UI from this file

Paste this into a coding agent. It is a tool, not an essay. Do not invent operations, headers, or fields. If a name is not here, look it up in the staleness list or omit it.

Authority: live engine in this repo. Human contract: `docs/13-ui-wrapper-contract.md`.

## Staleness

Re-read these if anything here might have moved:

1. `crates/wicket-server/src/capabilities.rs` — operationId, method, path, permission
2. `crates/wicket-server/src/openapi.rs` — query/header params, Idempotency-Key / If-Match / X-Wicket-Signature lists
3. `crates/wicket-server/src/session.rs` + `handlers/mod.rs` login/logout — cookies, Bearer, CSRF
4. `crates/wicket-server/src/envelope.rs` + `error.rs` — envelopes and error codes
5. `modules/genealogy/src/store.rs` + `handlers/mod.rs` `genealogy_trace` / `get_genealogy_job`
6. `docs/10-api-conventions.md` — conventions; **code wins** on disagreement
7. This file and `docs/13-ui-wrapper-contract.md`

## Base URL

One engine process. Default bind `0.0.0.0:8080` (`WICKET_BIND` / `--bind` / TOML `bind` in `crates/wicket-server/src/config.rs`).

- First-party Vite: `VITE_API_BASE` defaults to `""`. Dev server proxies `/api` to `WICKET_API_ORIGIN` or `http://127.0.0.1:8080` (`apps/wicket-web/vite.config.ts`, `apps/wicket-web/src/api/http.ts`).
- Production: the engine serves the SPA when `WICKET_UI_ROOT` is set to a directory that contains `index.html` (`docs/12-configuration.md`; `crates/wicket-server/src/http.rs:148-152`). Unset, it does not serve `index.html` and is API only. CORS headers are ABSENT. Same-origin via that root, a reverse proxy, or native HTTP (Bearer). Do not assume `fetch` from another origin works.
- LAN discovery, mDNS, deep links: ABSENT. Operator configures the origin.
- TLS is not assumed. `Secure` cookies only when request has `X-Forwarded-Proto: https`.

No tenant id in path, header, or body. All public JSON routes are under `/api/v1/` except `GET /health`.

## House rules

- Dark mode default. Light mode only if the user asks (`DESIGN.md`).
- No emojis anywhere.
- Never fake unbacked data. Render only what the engine returned. No lorem ipsum, no invented quantities, no placeholder lots on a live screen.
- Status = word + shape; color is never the only signal.
- Identifiers and quantities: monospace, tabular figures.
- Floor: no nav rail; targets ≥ 64×64 px, 16 px gap.
- Do not `window.print`. Archival documents: `POST /api/v1/print/render`.
- Do not mint UUIDs for records. Do not send `actor_id` / `user_id` / `posted_by`.
- Do not hardcode the operation set. Profiles `plain-shop` and `regulated-device` change navigation and signature gates.

Stack (ADR 0009): TypeScript, React, TanStack Query / Table / Router. One app, three mode route trees. Tauri v2 later for mac/linux/windows/android/ios; a browser on the LAN is a complete v1 client. The generated OpenAPI TypeScript client is `apps/wicket-web/src/api/generated/openapi.ts`, generated from `crates/wicket-server/tests/fixtures/openapi-document.json` by `just openapi-client`. `apps/wicket-web/src/api/http.ts` still performs `fetch`.

## Auth handshake

`POST /api/v1/identity/login`

Headers: `Idempotency-Key: <uuid>`, `Content-Type: application/json`. No session. No CSRF.

```json
{ "username": "mreyes", "password": "<login password>" }
```

200:

```json
{ "session_id": "<uuid>", "principal_id": "<uuid>", "display_name": "M. Reyes", "csrf": "<uuid>" }
```

`Set-Cookie`: `wicket_session=<session_id>; Path=/; SameSite=Lax; HttpOnly` and `wicket_csrf=<csrf>; Path=/; SameSite=Lax` (not HttpOnly). `Secure` only if `X-Forwarded-Proto: https`. No Max-Age. Server expiry is 12 hours. Session id is `gen_random_uuid()`, not UUID v7.

Hold afterwards, pick one:

- Browser same-origin: cookies + `credentials: include`. On every POST/PUT/PATCH send `X-CSRF-Token: <csrf>` matching cookie `wicket_csrf` / login `csrf`. Mismatch or missing → 403 `FORBIDDEN`.
- Native / machine: `Authorization: Bearer <session_id>`. CSRF is skipped when `Authorization` starts with `Bearer `. Persist `session_id` in secret storage.

If both cookie and Bearer are sent, Bearer wins for the session id and CSRF is skipped.

`POST /api/v1/identity/logout` — permission `identity.session`, `Idempotency-Key` required, CSRF if cookies, 204. Server deletes the session row and does **not** clear cookies. Drop them yourself.

`GET /api/v1/identity/me` returns principal (`id`, `principal_kind` `User|Service|Migration`, `username`, `display_name`, `status` `Active|Inactive`, `created_at`, `deactivated_at`). It does **not** return permissions.

Login / `/me` / navigation do not return the permission snapshot. Hide UI by `GET /api/v1/navigation` (profile modules) and by 403, or fail closed. Do not invent a permission list from `profiles/*.toml`.

## Headers on every call

| Header | When |
|---|---|
| `X-Request-Id` | Optional UUID. Server mints UUID v7 if missing/invalid. Echoed on the response. |
| `Idempotency-Key` | UUID. Required on the POST/PUT ids in the catalog `I` column. Replay same body → original status/body. Different body → 409 `IDEMPOTENCY_CONFLICT`. |
| `If-Match` | `"<integer version>"` on catalog `M` column. `*` rejected. Stale → 409 `CONFLICT` field `version`. |
| `X-Wicket-Signature` | UUID of minted signature, on Required edges. |
| `X-CSRF-Token` | Cookie-auth mutations only. |
| `Authorization` | `Bearer <session_id>` for non-cookie clients. |

`esignChallenge` is POST and CSRF-gated and does **not** take `Idempotency-Key`.

## Envelopes

List:

```json
{ "data": [], "next_cursor": null, "has_more": false }
```

`limit` query: omit (default 50) or integer 1..200. `cursor`: send back `next_cursor` unchanged (item/WO cursors are UUIDs). Empty page is 200, not 404.

Error (only this object):

```json
{ "error": { "code": "VALIDATION", "message": "…", "field": "limit", "request_id": "<uuid>" } }
```

`field` may be `null`. Branch on `code`. Live codes: `VALIDATION` 400, `UNAUTHENTICATED` 401, `FORBIDDEN` 403, `NOT_FOUND` 404, `CONFLICT` 409, `IDEMPOTENCY_CONFLICT` 409, `SIGNATURE_REQUIRED` 401 or 403, `SIGNATURE_NO_PROVIDER` 409, `TIMEOUT` 504, `REFUSED` 409, `INTERNAL` 500. `RATE_LIMITED` is named in docs and never emitted.

JSON keys: snake_case. Quantity: `{ "amount": "1.00000000", "unit": 1, "dimension": "Count" }` — `amount` is a **string**. Money: `{ "amount": "2034.000000", "currency": 840 }`. Expiry: `{ "value": "2027-09", "precision": "month" }` with `precision` `day|month|year`.

Enum casing is mixed. Copy the field: dimensions `Count`, item kind `make`, lot status `quarantine`, WO status `in_process`, job state `Succeeded`.

`GET /health` returns crate version as **plain text**.

## Discovery

1. `GET /api/v1/openapi.json` — unauthenticated. OpenAPI 3.0.3. 72 operations. Per operation: `operationId`, `x-wicket-permission`, parameters, optional `x-wicket-signature: { meaning, permission }`. 70 of 72 have a 200 `application/json` schema. `health` and `logout` do not: `health`'s 200 is description `ok` with no `content`, and `logout` has no JSON schema and no `requestBody`. 28 operations carry `requestBody`. `components.schemas` holds 114 schemas, including `ErrorEnvelope`. `plain-shop` advertises 0 signatures; `regulated-device` advertises 3 (`approveCalibration`, `approveDocument`, `releaseFromQuarantine`).
2. `GET /api/v1/navigation` — `{ "visible": ["items", …], "hidden": ["calibration"] }`. Profile, not per-user.
3. Do not assume the path set is stable across profiles. The router still mounts all 72 rows (42 `kernel(`, 30 `module(`) including disabled-module routes.

`x-wicket-permission` empty string = unauthenticated.

## Job polling (genealogy is the template)

`GET /api/v1/genealogy/trace?from_lot_id=<uuid>&direction=forward`

`direction`: `forward` (default) | `backward` | `both`. Only `from_lot_id` is accepted. Do not send `lot`/`serial`/`posting`/`depth`/`format`.

Handler always returns **HTTP 200**. Branch on body:

- If `job_id` and `result_url` present: job. `result_url` is a path `/api/v1/genealogy/jobs/{id}`.
- Else if `backward` and `forward` keys: both-direction trees.
- Else `{ "direction": "forward"|"backward", "nodes": [ TreeNode ] }`.

`TreeNode`: `posting` (i64), `item`, `lot`, `serial`, `location`, `quantity`, `amount` (string), `amount_currency` (number), `occurred_at`, `edge_quantity`, `children` (nested). This is **not** `{ root, nodes, edges }`.

Poll `GET {result_url}` until `state` is `Succeeded` (render `result` as the same tree shape), `Failed`, or `Cancelled`. Fields: `id`, `kind`, `payload`, `state` (`Queued|Running|Succeeded|Failed|Cancelled`), `attempts`, `max_attempts`, `run_after`, `progress_pct` (0–100), `progress_note`, `result`, `last_error`. Show progress. Do not use an unexplained spinner.

Inline threshold default 32 postings (`WICKET_GENEALOGY_INLINE_MAX`).

## Electronic signatures

Before a Required transition, tell the user the meaning in plain words, then collect identification. The login session is not a signing component. Signing password ≠ login password. Continuous-session relaxation is off.

1. `POST /api/v1/esign/challenges` → `{ "components_required": ["code","secret"], "signing_session_expires_at": null, "credential_kind": "signing_password" }`. CSRF yes, Idempotency-Key no.
2. `POST /api/v1/esign/signatures` with `Idempotency-Key`:

```json
{
  "meaning": "Released",
  "record": { "table": "sm.instance", "id": "<uuid>", "version": 3 },
  "identification": { "code": "MREYES", "secret": "<signing password>" },
  "doc_type": "production"
}
```

201 `{ "signature": { "id", "signer_id", "printed_name", "meaning", "reason", "signed_at", "signed_at_zone", "signed_at_local", "record": { "table", "doc_type", "id", "version" }, "record_content_hash", "credential_kind", "components_used", "superseded", "superseded_by_version" } }`. Display printed_name, meaning, signed_at to the user.

3. Transition POST with `X-Wicket-Signature: <signature.id>` and `If-Match`. No secret in the transition body.

`plain-shop` gate is `NoSignatures`: Required edges return 409 `SIGNATURE_NO_PROVIDER`. Missing token on a Required edge: 401 `SIGNATURE_REQUIRED`.

## Catalog

I = Idempotency-Key required. M = If-Match required. Permission blank = unauthenticated.

| id | Method path | perm | I | M |
|---|---|---|---|---|
| health | GET /health | | | |
| getOpenApi | GET /api/v1/openapi.json | | | |
| getValidationManifest | GET /api/v1/iq/manifest | validation.manifest.read | | |
| exportAudit | GET /api/v1/audit | audit.export | | |
| getNavigation | GET /api/v1/navigation | identity.session | | |
| login | POST /api/v1/identity/login | | I | |
| logout | POST /api/v1/identity/logout | identity.session | I | |
| createPrincipal | POST /api/v1/identity/principals | identity.manage | I | |
| getPrincipal | GET /api/v1/identity/principals/{id} | identity.manage | | |
| renamePrincipal | POST /api/v1/identity/principals/{id}/rename | identity.manage | I | |
| deactivatePrincipal | POST /api/v1/identity/principals/{id}/deactivate | identity.manage | I | |
| resetLoginCredential | POST /api/v1/identity/principals/{id}/login-credential | identity.manage | I | |
| getOwnProfile | GET /api/v1/identity/me | identity.session | | |
| changeOwnLoginCredential | POST /api/v1/identity/me/login-credential | identity.session | I | |
| setOwnSigningCredential | POST /api/v1/identity/me/signing-credential | identity.session | I | |
| approveCalibration | POST /api/v1/calibration/certificates/{id}/approve | calibration.approve | I | M |
| esignChallenge | POST /api/v1/esign/challenges | identity.session | | |
| esignMint | POST /api/v1/esign/signatures | identity.session | I | |
| getEsignSignature | GET /api/v1/esign/signatures/{id} | identity.session | | |
| getEsignBundle | GET /api/v1/esign/signatures/{id}/bundle | esign.bundle.read | | |
| defineCustomField | POST /api/v1/customfields/definitions | customfields.define | I | |
| listCustomFieldDefinitions | GET /api/v1/customfields/definitions | customfields.view | | |
| retireCustomField | POST /api/v1/customfields/definitions/{id}/retire | customfields.retire | I | M |
| setItemCustomFields | PUT /api/v1/items/{id}/custom-fields | customfields.set | I | |
| getItemCustomFields | GET /api/v1/items/{id}/custom-fields | customfields.view | | |
| createDocument | POST /api/v1/documents | documents.edit | I | |
| getDocument | GET /api/v1/documents/{id} | documents.view | | |
| createDocumentRevision | POST /api/v1/documents/{id}/revisions | documents.edit | I | |
| submitDocument | POST /api/v1/documents/{id}/submit | documents.edit | I | M |
| approveDocument | POST /api/v1/documents/{id}/approve | documents.approve | I | M |
| listPrintTemplates | GET /api/v1/print/templates | print.templates | | |
| renderPrint | POST /api/v1/print/render | print.render | I | |
| archivePrint | POST /api/v1/print/archive | print.archive | I | |
| releaseFromQuarantine | POST /api/v1/inventory/releases | lots.release | I | M |
| reverseIssue | POST /api/v1/inventory/reversals | inventory.adjust | I | |
| listItems | GET /api/v1/items | items.view | | |
| createItem | POST /api/v1/items | items.edit | I | |
| getItem | GET /api/v1/items/{id} | items.view | | |
| updateItem | PATCH /api/v1/items/{id} | items.edit | I | M |
| releaseItem | POST /api/v1/items/{id}/release | items.release | I | M |
| listLocations | GET /api/v1/locations | locations.view | | |
| createLocation | POST /api/v1/locations | locations.edit | I | |
| getLocation | GET /api/v1/locations/{id} | locations.view | | |
| listLocationTree | GET /api/v1/locations/tree | locations.view | | |
| deactivateLocation | POST /api/v1/locations/{id}/deactivate | locations.edit | I | M |
| listLots | GET /api/v1/lots | lots.view | | |
| createLot | POST /api/v1/lots | lots.edit | I | |
| getLot | GET /api/v1/lots/{id} | lots.view | | |
| setLotStatus | POST /api/v1/lots/{id}/status | lots.release | I | M |
| listPackages | GET /api/v1/lots/{id}/packages | lots.view | | |
| createPackage | POST /api/v1/lots/{id}/packages | lots.edit | I | |
| listSerials | GET /api/v1/lots/{id}/serials | lots.view | | |
| createSerials | POST /api/v1/lots/{id}/serials | lots.edit | I | |
| createReceipt | POST /api/v1/inventory/receipts | inventory.receive | I | |
| createCount | POST /api/v1/inventory/counts | inventory.count | I | |
| getOnHand | GET /api/v1/inventory/on-hand | inventory.view | | |
| listWorkOrders | GET /api/v1/work-orders | production.view | | |
| createWorkOrder | POST /api/v1/work-orders | production.create | I | |
| getWorkOrder | GET /api/v1/work-orders/{id} | production.view | | |
| releaseWorkOrder | POST /api/v1/work-orders/{id}/release | production.release | I | M |
| issueWorkOrder | POST /api/v1/work-orders/{id}/issue | production.issue | I | M |
| completeWorkOrder | POST /api/v1/work-orders/{id}/complete | production.complete | I | M |
| traceGenealogy | GET /api/v1/genealogy/trace | genealogy.view | | |
| getImpact | GET /api/v1/genealogy/impact/{lot} | genealogy.view | | |
| getGenealogyJob | GET /api/v1/genealogy/jobs/{id} | genealogy.view | | |

Query params declared in OpenAPI: `listItems` `limit,cursor,kind,status,number_prefix`; `listLocations`/`listLots` `limit,cursor`; `listLocationTree` `include_inactive`; `listWorkOrders` `limit,cursor,status`; `getOnHand` `item_id` (required), `location_id`, `lot_id`; `traceGenealogy` `from_lot_id` (required), `direction`; `listCustomFieldDefinitions` `entity` (required).

Verified item body fields: `id,number,revision,description,kind,stock_uom,stock_scale,residual_tolerance,cost_method,status,version,application_version,configuration_version,created_at`. Work-order body: `id,number,item_id,quantity,status,revision,wip_location_id,version,application_version,configuration_version,released_at,completed_at`. Lot body: `id,identifier,item_id,supplier_lot,heat,expiry,cert_ref,status,udi_device_identifier,version,created_at`.

`sort` is unspecified in OpenAPI and unread by handlers. Several lists hard-code `next_cursor: null` (`listPackages`, `listCustomFieldDefinitions`, `listPrintTemplates`). `listSerials` ignores query pagination.

## Worked example — login to a genealogy trace on screen

Engine at `http://127.0.0.1:8080`. Cookie client.

1. `POST /api/v1/identity/login` with `Idempotency-Key` and `{ "username", "password" }`. Store `csrf`. Keep cookies.
2. `GET /api/v1/openapi.json`. Confirm `traceGenealogy` exists; read `x-wicket-permission` (`genealogy.view`).
3. `GET /api/v1/navigation`. If `genealogy` is not in `visible`, do not show the screen.
4. Operator supplies a lot UUID (human-identifier lookup is ABSENT).
5. `GET /api/v1/genealogy/trace?from_lot_id=<uuid>&direction=forward` with cookies. Expect 200.
6. If the JSON has `job_id`:
   - Show `progress_pct` / `progress_note` from `GET <result_url>` every second.
   - On `Succeeded`, take `result` as the tree. On `Failed`, show `last_error`.
7. If the JSON has `nodes` (and not `job_id`): that is the tree. If it has `backward` and `forward`, render both forests.
8. Walk `nodes[]`. Each node has nested `children`. Draw `lot` / `serial` / `quantity.amount` + `quantity.dimension`. Do not look for `root` or `edges`.
9. Dark canvas, no emojis, monospace identifiers, status as words. If the engine returned no nodes, show an empty state with words, not invented lots.

Do not treat HTTP 202 as the job signal. Do not render a tree from a job body.

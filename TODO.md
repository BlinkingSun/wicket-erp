# TODO

**Conforms to:** [GOALS.md](GOALS.md); [ADR 0010](docs/adr/0010-one-registry.md) (Proposed);
`docs/07-roadmap.md` (wave structure); `PLAN.md` §5, §6, §11.

*No dates. No durations. Progress is a gate that passes (`docs/07-roadmap.md` §5.4).
Sizes are S, M, L and describe scope, not calendar time.*

This backlog exists because the four goals in `GOALS.md` were added to a tree that did
not have them. It was produced by five independent analyses of the repository, each on a
separate slice, and then deduplicated. Where several analyses reached the same work from
different directions, the item says so, because that convergence is the evidence that
the item is real.

---

## How to use this file

**Stage 0 is the contribution product, and it comes first.** It needs no decision
from the owner. It stops documents from lying to contributors now. Do it first.
Most of Stage 0 landed in the 2026-09-14 swarm. T-01 and T-17 closed 2026-09-15.
**Remaining: T-13 (licence-identifier backfill), T-105 (lint the npm allowlists), T-106 (drop the npm peer-resolution escape hatch), T-107 (`just ci-db` fails closed when the migrate URL is unset), T-108 (database-suite throughput ceiling), and T-110 (item-master part number is monospace).**

**Stage 1 is the keystone, and it is blocked on accepting
[ADR 0010](docs/adr/0010-one-registry.md).** Seven separate items from four analyses
collapse into it. T-20 through T-27 have landed. Stage 2 (T-30) is the next coverage wave.

Stages 2 through 5 are ordered by dependency, not by importance. Stage 5 is the largest
and can only begin once Stage 2 gives it a catalogue to map onto.

---

## Stage 0 — Truth and intake

No dependencies. Every item here is small, and several fix statements in the repository
that are currently false.

| ID | Work | Size | Done when |
|---|---|---|---|
| T-01 | **DONE.** Fill the conduct and security contacts | S | `CODE_OF_CONDUCT.md:39` and `SECURITY.md:9` hold real addresses instead of placeholders; SECURITY states whether GitHub private vulnerability reporting is enabled |
| T-02 | **DONE.** Pull request template carrying the four-goal gate | S | `.github/pull_request_template.md` requires an answer or a reasoned "not applicable" for each goal, plus sign-off and local gate confirmation |
| T-03 | **DONE.** Issue templates and a published label set | S | Bug, feature, module proposal and request-for-comment forms exist; the label set is documented in `CONTRIBUTING.md` |
| T-04 | **DONE.** Sign-off check in CI | S | `scripts/check-dco.sh` is wired in `.github/workflows/ci.yml` job `lint-policy`. A pull request whose commits lack `Signed-off-by` fails the build |
| T-05 | **DONE.** Request-for-comment path | S | A form and an index exist; `CONTRIBUTING.md` states which diffs require one before a pull request |
| T-06 | **DONE.** Goal-heading lint on pull request bodies | S | `scripts/check-pr-goals.sh` fails a pull request whose body omits a goal heading |
| T-07 | **DONE.** Repair dangling `_team/` citations in tracked files | M | No tracked file cites a path under `_team/` (other than this backlog's own history of the defect). Cites were retargeted to `research/` and `docs/` or restated inline |
| T-08 | **DONE.** Move the `_team` exclusion into the repository | S | `.gitignore` excludes `_team/` |
| T-09 | **DONE.** Correct the supported PostgreSQL floor | S | `docs/01-vision-and-scope.md` states 17 minimum, 18 tested, matching `docs/09-workspace-contract.md` |
| T-10 | **DONE.** Close the settled open questions in the vision | S | `docs/01-vision-and-scope.md` §8 records licence and DCO as settled by [ADR 0006](docs/adr/0006-license.md). The one-constant name claim is restated as false since the rename |
| T-11 | **DONE.** Stamp audience and status on every document | S | Product docs, ADRs, HANDOFF (historical), DESIGN, and research/README declare audience and status. HANDOFF no longer claims "no code has been written" as present tense |
| T-12 | **DONE.** Correct the stale hosted-CI wording | S | HANDOFF, README, and CONTRIBUTING agree that hosted Actions are on. Slice acceptance is `just ci-db` |
| T-13 | Licence identifier lint and backfill | S | Every source file carries its identifier and a lint enforces it. `CONTRIBUTING.md` §9 requires this today and no file complies. Backfill in one maintainer change before enabling the lint |
| T-14 | **DONE.** Unimplemented-macro scanner | S | `scripts/lint-unimplemented.sh` fails CI on a placeholder macro outside compile-fail fixtures |
| T-15 | **DONE.** The acceptance suite is outside the default gate | S | `README.md` §5 and `CONTRIBUTING.md` name `just ci` as the offline lint/lib gate and `just ci-db` as slice acceptance. `just ci` still ends in `test-lib` (`--lib`); that is now documented rather than mislabeled |
| T-16 | **DONE.** The build file describes a passing recipe as expected to fail | S | `justfile` no longer says ci-db is "expected RED until harness lands". Public CI's `just ci-db` is green |
| T-17 | **DONE.** Repository settings contradict the merge policy | S | GitHub `allow_squash_merge` is false, matching `CONTRIBUTING.md` §7 and `GOALS.md` GOV-5. Branch protection on `main` either enforces fast-forward or is recorded as ABSENT. Private vulnerability reporting is on, or `SECURITY.md:9-11` states it is off (see T-01). Closed 2026-09-15: `allow_squash_merge` and `allow_rebase_merge` set false; private vulnerability reporting enabled; `CONTRIBUTING.md` §7 and `SECURITY.md` updated to match. Two residuals recorded rather than hidden: `allow_merge_commit` stays true because GitHub refuses to disable every merge strategy (it is the only one that does not rewrite signed commits), and branch protection on `main` is **ABSENT** because GitHub has no fast-forward-only mode |
| T-18 | **DONE.** One writer per file per wave | S | `CONTRIBUTING.md` §7 and `AGENTS.md` state one writer per file per wave. Historical `integrate: merge` overlapping-ownership commits remain in git history; new waves must not add more |
| T-105 | **Lint the npm allowlists** | S | A script fails when `apps/wicket-web/package.json` gains a `dependencies` entry without an accepted request for comment, and when a `devDependencies` entry is imported by shipped code. GOV-7. ABSENT |
| T-106 | **Drop the npm peer-resolution escape hatch** | S | `apps/wicket-web/.npmrc` no longer sets `legacy-peer-deps`, because `openapi-typescript` declares a peer range that includes the TypeScript major this app is on. Until then peer resolution is off workspace-wide |
| T-107 | `just ci-db` fails closed when `WICKET_MIGRATE_DATABASE_URL` is unset | S | The recipe prints the unset name and exits non-zero before `cargo test`, instead of panicking inside `wicket_test::require_postgres` (`crates/wicket-test/src/lib.rs:78-80`) |
| T-108 | Database-suite throughput ceiling | M | The bootstrap connect timeout is configurable, and a test does not force one Postgres checkpoint. Today the timeout is a hardcoded 2 s (`crates/wicket-test/src/lib.rs:63` and `:397`) and each test drops its database with `DROP DATABASE … WITH (FORCE)` (`lib.rs:187`, `lib.rs:390`) |
| T-110 | Item-master part number is monospace with tabular figures | S | `h2.item-master__number` (`apps/wicket-web/src/features/items/ItemMasterScreen.tsx:43`) uses a monospace face and tabular figures. Today `apps/wicket-web/src/features/items/items.css:24` sets size, weight, and letter-spacing only, and does not use `.mono` (`apps/wicket-web/src/styles/global.css:40`) |

---

## Stage 1 — The keystone: one registry

**Depends on:** nothing. **Blocks:** Stages 2, 3 and 5.

Governed by [ADR 0010](docs/adr/0010-one-registry.md). Three analyses found the manifest
drift independently; three found the dropped route method independently.

| ID | Work | Size | Depends | Done when |
|---|---|---|---|---|
| T-20 | **DONE.** Single manifest source. `compiled_in()` reads each `module.toml` rather than embedding a copy | M | — | No inline manifest strings remain in `crates/wicket-module/src/manifest.rs`; first-party modules are `include_str!` of `modules/*/module.toml`; calibration (no crate) is a fixture. A test fails on drift between the compiled-in catalogue and the file on disk |
| T-21 | **DONE.** Routes carry their method | S | T-20 | `[[routes]]` requires `method`; `ManifestRoute` and `ModuleRoute` store it; the local re-parse in `modules/items/src/lib.rs` is deleted |
| T-22 | **DONE.** Machines survive registration | M | T-20 | Declared machines are the machines the engine freezes, including not-required reasons; `register()` no longer clears them |
| T-23 | **DONE.** Mount reverse-diff lint | S | — | Landed as the interim step; T-24 replaced the lint so it binds capability ids rather than grepping `.route("` vs `MOUNTED` |
| T-24 | **DONE.** The capability table, and a router generated from it | L | T-21, T-23 | `crates/wicket-server/src/capabilities.rs` is the only route source. `http.rs` binds handlers by capability id. `scripts/lint-mounts.sh` fails a string-literal `.route("` and an unbound id |
| T-25 | **DONE.** Document generated from the table; parity test reads the router | M | T-24 | OpenAPI is generated from the capability table. `openapi_listed_paths_are_not_bare_404` asserts the served document equals the table and probes the live router |
| T-26 | **DONE.** Collapse the duplicate work-order namespace | S | T-23 | `/api/v1/work-orders` is the only prefix. `/api/v1/production/work-orders` is gone, not aliased |
| T-27 | **DONE.** Module manifest lint | M | T-20 | `scripts/lint-module-manifests.sh` fails when a first-party module's crate name, identifier, schema, canonical order, or either profile disagree. Wired into `just ci` |

---

## Stage 2 — Coverage

**Depends on:** Stage 1. Serves Goal 2.

Everything here is a capability that already exists in Rust and cannot be reached over
the wire. None of it is new functionality.

| ID | Work | Size | Depends | Done when |
|---|---|---|---|---|
| T-30 | **DONE.** Mount the module reads and writes that already have handlers | M | T-24 | Item list and patch, location list, tree, patch and deactivate, lot list, serial creation, work-order list, and the genealogy impact and job routes all respond |
| T-31 | Inventory writes | L | T-24 | Issues, moves, adjustments, document read and void are mounted with tests, or their declarations are removed in the same change. `modules/inventory/src/api.rs:65` declares operation id `getDocument` on `GET /api/v1/inventory/documents/{id}`; kernel `GET /api/v1/documents/{id}` already uses `getDocument` (`crates/wicket-server/src/capabilities.rs:364`). The inventory route is unmounted. The mount-time name is `getInventoryDocument`. Wave ui2 renames it |
| T-32 | Remaining machine edges | M | T-24 | Item obsolete, work-order cancel, and the document lifecycle edges each have an operation with optimistic concurrency |
| T-33 | Job status, enqueue and cancel | M | T-24 | A job identifier returned by a trace resolves at `getGenealogyJob` (`crates/wicket-server/src/http.rs:99`). Enqueue and cancel are still unmounted |
| T-34 | Kernel administration over HTTP | L | T-24 | Module registry, principals and roles, numbering, units of measure, and audit verification and export are reachable without linking the crate |
| T-35 | **Wave 0 DONE** (inputs described; signature meaning reads `signature_edges`). Typed bodies: 70 of 72. Document schemas and per-edge signature meaning | L | T-25 | Every operation carries request and response schemas. 70 of 72 do, on both profiles; `health` and `logout` do not. Signature meaning reads the edge (`crates/wicket-server/src/openapi.rs:50-68`); `setLotStatus` is omitted by id (`openapi.rs:55-57`) |
| T-109 | Schema registration rejects a differently-shaped duplicate name | S | T-25 | A second `JsonSchema` type with the same schemars name and a different shape is a hard error. Today `merge_type` keeps the first writer (`entry().or_insert` at `crates/wicket-server/src/schemas/mod.rs:120` and `:126`) and drops the later type |
| T-36 | Cursor pagination and filtering on list operations | M | T-30 | Limit and cursor are honoured; no handler hard-codes a null cursor on a non-empty page |
| T-37 | Rate limit and method-not-allowed envelope | S | — | The documented burst limit returns its error code, and an unsupported method returns the envelope. Both are specified in `docs/10-api-conventions.md` and neither is implemented |
| T-38 | CLI twin table and allowlist | M | T-34 | Every subcommand except process lifecycle has an operation or an allowlist row with a reason |
| T-39 | Custom fields for every entity that has definitions | M | T-24 | No longer restricted to items |
| T-40 | Document attach, link, history and legal hold | M | T-32 | Each public function has an operation or an allowlist row |
| T-41 | Print log and template version bump | S | T-24 | Routed, or made crate-private |
| T-42 | Ledger reversal, balance and projection verification | M | T-31 | No public ledger operation remains without a row or an allowlist entry |
| T-43 | **The Goal 2 gate: capability coverage test** | M | T-24, T-38 | Boots both profiles, walks engine edges, job kinds, module routes and CLI subcommands against the table, and fails on any capability with no row and no allowlist entry |
| T-44 | **DONE.** Golden OpenAPI fixture fails the build on path drift | S | T-23 | Closed 2026-09-15. `crates/wicket-server/tests/fixtures/openapi-operations.txt` holds the 72 **operations** (one comment line plus 72 `METHOD path` lines), and `scripts/lint-openapi-fixture.sh` (wired into `just ci`) set-diffs it against the capability table, naming extras and missings. `just openapi-fixture` regenerates and is never a `ci` dependency; every sort is `LC_ALL=C` so regeneration is byte-stable across locales. The fixture keys on `(method, path)`, **not** paths: 72 operations span 62 unique paths because 10 paths carry two methods, so a path-keyed fixture would miss method drift. A slice test also diffs the **served** document against the same fixture. The self-referential parity test named below is kept — its live-router bare-404 probe is independent — but it is no longer the only check |

---

### T-35 progress (2026-09-15)

**Wave 0 landed** (`10bf296`): 31 path parameters, 19 query fields from the handler structs, and
required headers declared **per route** — there is no `CapabilityKind`-based rule, and asserting
one was wrong in both directions. Signature meaning now reads the frozen engine
(`state.kernel().profile.signature_edges`), so `document(&AppState)` finally uses its state.

The document had stamped `meaning: "Released"` on all 10 transitions in **both** profiles. The
truth is **0 signed rows on `plain-shop`** and **3 on `regulated-device`** — every signature
claim in the plain-shop document was false, which for a Part 11 product is a claim a client would
have acted on. `setLotStatus` is omitted by id: it joins `(lot, release)` with
`releaseFromQuarantine`, but its handler also serves `hold` and `reject`.

**Wave 1 typed bodies are partial.** 70 of 72 operations carry a 200 `application/json` schema
on both profiles (fixtures `openapi-document.json` and `openapi-document-regulated.json`).
`health` and `logout` do not, so this row stays open. Residual from Wave 0: the header
allowlists are hand-maintained and can drift from handlers; a derivation or a drift test is a
follow-up.

---

### Not in this backlog, and a first UI screen needs it

**Human-identifier lookup.** Item number, work-order number, lot identifier and serial
identifier resolved to ids. `design/mockup-shop-floor.png` has a scan box; an operator scans a
part number, not a UUID. No Stage 2 item delivers this.

Scoped 2026-09-15. **Shape: per-entity `GET /api/v1/{collection}/by-number/{n}`**, not a root
`/api/v1/resolve`. `docs/10:21` lets a module register only under its own prefix, so a
polymorphic resolver would have to be a kernel/composition-root operation fanning into four
module crates — and worse, the schema does not make a scanned string unique across kinds, and
**serials are not unique even within kind**, so a `{kind, id}` response would be a lie without an
owner-defined collision policy. Per-entity routes sidestep that entirely.

**Mixed cost, not one job.** `lots` already has a resolver, exported and unmounted — mounting it
is one `[[routes]]` row, one capability, one handler: hours. `items`, work orders and serials
need new crate functions. `location_id_by_code` / `site_id_by_code` are likewise crate-ready and
unmounted (shop-floor chrome shows `WC-LATHE-03`). Not kernel-crate work either way.

**Blocking permission gap, and it is a profile edit rather than a new key.** The seeded
`operator` bundle in **both** profiles has `items.view`, `production.view` and `genealogy.view`
but **not `lots.view`** (`profiles/plain-shop.toml:109-127`,
`profiles/regulated-device.toml:121-135`; the admin bundle has the same hole). Once by-number is
mounted, a seeded operator resolves a traveler number and an item number and then **403s on lot
and serial lookup — which is exactly the genealogy mockup's search box**. Granting `lots.view`
to `operator` is an owner decision about the permission model, not something a lane may do.

Evidence: `_team/reports/spike-ui-gap.md`, `_team/reports/spike-identifier-lookup.md`.

---

## Wrapper engine findings (verified)

Source: `_team/reports/FINDINGS-wrapper.md` (`wrapper-r1`) and `FINDINGS.md` (W6–W15,
`backlog-r1`). Re-verified against the tree before recording here (2026-09-16, `docs10-r1`;
W6–W17 citations checked on landing). These are not T- ids; they track gaps between the
running engine and what operators and UIs need.

### W5 — same origin for API and UI (decide before wrapper packaging)

The Vite dev server proxies `/api` to the engine on loopback, so first-party development is
same-origin today. A UI opened on another shop-floor device is cross-origin: the engine emits
no CORS headers, and cookie session auth does not cross origins. That blocks the stated goal
of one engine serving UIs on many devices without a deliberate deployment choice.

**Smallest fix is a product decision, not a one-liner:** either the engine serves the built
SPA from the same origin, or the engine grows an explicit CORS policy (and likely moves
machine clients to bearer tokens). Record the choice before packaging the UI wrapper.

| ID | What is wrong | Why it matters | Smallest fix |
|---|---|---|---|
| W1 | `POST /api/v1/identity/logout` returns `204` with no `Set-Cookie`; login set `wicket_session` (httpOnly) and `wicket_csrf` (`handlers/mod.rs:222-262`, `logout` `266-297`) while `session::drop_session` deletes the server row (`session.rs:149`) | Dead cookies linger on shared terminals; JS cannot clear httpOnly session cookies, so the next user sees confusing 401s instead of a clean login | Logout emits both cookies with `Max-Age=0` on the same path/flags as login; test the response headers |
| W2 | Login returns no permission snapshot; `GET /api/v1/navigation` is profile-level, not per-user; no `getOwnPermissions` operation | UIs cannot grey out forbidden actions; every control is try-and-403, which is a shop-floor training problem | Design decision: permission list on login and/or a dedicated introspection route — record before hard-coding role checks in the UI |
| W3 | **DONE in docs (2026-09-16).** `docs/10-api-conventions.md` §9.6 sketched `root` / flat `nodes` / `edges` and implied 202 for large traces; the engine emits nested `TreeNode` / `children` and returns job acceptance at HTTP 200 with `job_id` / `result_url` (`domain.rs` `TreeNode`, `handlers/mod.rs` `genealogy_trace`) | Wrong authoritative contract misled specs and clients (including UI fixtures) | Docs corrected at source; optional follow-up: handler calls `trace_http_status` or OpenAPI `oneOf` at 200 |
| W4 | `RATE_LIMITED`, list `sort`, session lock, OIDC, and human-identifier lookup appear in prose (`docs/10`, navigation mockups) without a mounted operation — scan-box lookup is the painful gap (`TODO.md` human-identifier note) | Clients and operators assume capabilities that 404 or never existed | Mount or tag ABSENT per item; human-identifier: per-entity `by-number` routes per scoped note above |
| W5 | No CORS; engine does not serve the built SPA (see subsection above) | Cross-device UIs cannot authenticate against a remote engine on the LAN | Choose same-origin static hosting vs CORS policy before wrapper packaging |
| W6 | `trace_http_status` in `modules/genealogy/src/api.rs:75-79` is never called; `genealogy_trace` returns HTTP 200 for both inline trees and accepted jobs (`crates/wicket-server/src/handlers/mod.rs:2363-2372`, `2416-2423`) | Fossil of an intended 202; clients today must parse the body (`oneOf` at 200) to know inline vs job | After `t35w1-r2` lands, decide once: delete the dead helper and document `oneOf` at 200, or wire `trace_http_status` and make job acceptance 202 — do not flip status mid-wave |
| W7 | Mounted `traceGenealogy` accepts only `from_lot_id` (`handlers/mod.rs:2354-2408`); the module documents `lot`, `serial`, and `posting` origins (`modules/genealogy/src/api.rs:116-120`) | Shop-floor scan box needs serial, lot, or part number — not a lot UUID; this gap is why the scan box does not exist | Widen the handler to match module origins (hours, not a new module) |
| W8 | `crates/wicket-server/src/error.rs:189` maps `Items::NotFound`; by-number resolvers return `Items::UnknownNumber` (`modules/items/src/error.rs:56`) with no arm, so misses hit the catch-all at `error.rs:282-287` as HTTP 500 | Unmounted today; mount wave would turn a routine bad scan into "internal server error" and pollute monitoring | **Required in the mount wave:** map `UnknownNumber` (and check `production_min::NotFound`) to 404 with a test — not a follow-up |
| W9 | **FIXED (`genfix-r1`).** UI had required `{root, nodes, edges}` from the old `docs/10` sketch; engine emits nested `nodes` / `children`. Hand-written `trace.json` fixtures encoded the same mistake, so tests stayed green | Flagship genealogy screen failed on first live engine contact; green unit tests meant nothing | Keep the rule: engine-response fixtures must be **captured** responses (`_team/artifacts/trace-{backward,forward,both}.json`), not authored from docs |
| W10 | **Minor, not urgent.** OpenAPI `AnyQuantity` / `MoneyWire` schemas (`crates/wicket-core/src/quantity.rs:301`, `money.rs:322`) use plain `string` for `amount` without the scale caps in `docs/10` §3.1–3.2 (`QUANTITY_MAX_SCALE`, `MONEY_MAX_SCALE`) | Honest under-promise: clients learn scale violations from `VALIDATION` on submit, not from the schema | Add `pattern` (or equivalent) when next editing these schemas — not its own wave |
| W12 | Demo seed posts a **lot-less receipt** so item-level `getOnHand` reads non-zero — a workaround for W11, documented in `demoseed-r1` | With correct item-level `on_hand`, the artificial untracked unit is unrealistic for a lot-controlled device | After W11 fix lands: remove the lot-less receipt from the seed so the demo shows real lot-tracked stock only |
| W13 | **Confirmed** on captured backward trace (`_team/reports/demoseed-r1.md`): under one parent, two child nodes are **byte-identical** in every exposed field (same `posting`, lot, item, qty, amount) — a consumer summing edges double-counts (e.g. 6000 mm vs 3000 mm seeded) | Recall and material-consumption views must not over-state input; regulated genealogy (`docs/06-regulatory.md`) | **Cause unknown** (duplicate emission vs two ledger rows the API cannot distinguish); fixes differ — reproduce on a live engine, then investigate (`genedup-r1`); do not guess a fix |

### `balance_at`: `None` lot or serial means untracked postings only (W11, W14, W15)

`wicket_ledger::balance_at` filters with `AND p.lot_id IS NOT DISTINCT FROM $4` and
`AND p.serial_id IS NOT DISTINCT FROM $5` (`crates/wicket-ledger/src/projections.rs:235-236`).
A `None` lot or serial therefore matches **`lot_id IS NULL` / `serial_id IS NULL`**, not "any
lot" or "any serial". Inventory tests almost always pass explicit `lot: Some(...)` (often with
`location: Some(...)`), e.g. `modules/inventory/tests/inventory.rs:38-44` and `:438-444`, and
`allocated` tests likewise (`inventory.rs:100-145`) — so the item-level HTTP paths stayed
unexercised until live measurement or a deliberate code read.

| ID | What is wrong | Why it matters | Smallest fix |
|---|---|---|---|
| W11 | **Confirmed on live demo engine.** Item-level `GET /api/v1/inventory/on-hand?item_id=…` reported `on_hand: 1` while lot-scoped query showed 5 and `available` showed 6 — one lot held 5 units plus 1 untracked. `on_hand` passes `query.lot` into `balance_at` (`modules/inventory/src/store.rs:531-566`) with the filter above | Planners, buyers, and the Item Master Inventory tab read this number; under-reporting causes phantom shortages and a shelf picture that does not match physical stock for lot-controlled devices | Failing test first: item-level `on_hand` must sum lot-tracked stock like `available` already does at item level (`store.rs:654-688`) |
| W14 | Same `lot: None` → untracked-only filter as W11, in `allocated` (`store.rs:570-596`). **Not on HTTP** — `getOnHand` exposes only `on_hand` and `available`; nothing in the UI reads `allocated` today | Lower urgency than W11 was, but same defect and same test blind spot (`case_c_issue_one_bar_to_wip_with_explicit_lot_pick_contributes_consumption` always passes `lot: Some(...)`) | Fix when touching allocated; add an item-level test. Note: `cycle_count` calling `on_hand` may already count lot-tracked stock by design — confirm no caller depended on the old number |
| W15 | **Not reproduced on a running engine** — code-level only. Same trap for `serial: None` (`projections.rs:236`); postings bind `serial_id` (`crates/wicket-ledger/src/post.rs:273-274`, migration `crates/wicket-ledger/migrations/00000000000001_ledger.up.sql:135`); document lines carry `serial_id` (`modules/inventory/src/domain.rs:123-134`, `modules/inventory/src/api.rs:177`, `modules/inventory/src/store.rs:760`); every `BalanceSlice` in `store.rs` passes `serial: None` (no `serial: Some` in this module). Lot-scoped slices still use `serial: None`, so lot+serial postings stay invisible; `available` skips serial-only layers at `store.rs:669` | Serialised devices are core to this product; stock the ledger can post but balances cannot see is worse than no serialisation | **Do not fix on this evidence alone — confirm first:** post a serialised receipt, query `on_hand` at item level, and check whether tests/seed can even produce serial-bearing postings today |
| W16 | Shop-floor kiosk error state surfaces `Request failed (404) for GET /api/v1/...` from `apps/wicket-web/src/api/http.ts:21` via `ShopFloorTerminal.tsx:70-73` | Same wire-format leak the floor rework removed elsewhere; operators should not see HTTP paths | Operator-facing copy (e.g. "Work order not found"); keep method/path in console or `title` |
| W17 | Genealogy backward trace renders **one card per posting**, so multiple top-level nodes share the same truncated lot id and look like duplicates (`FIDELITY.md` capture after `genpolish-r1`) | Recall investigators think in lots and movements; repetition reads as a broken screen | **Design decision:** group by lot with postings inside, or show posting id / `occurred_at` on each card — decide deliberately; not a default |

---

**`traceGenealogy` origin widening.** The mounted operation is narrower than the crate's real
origins (`serial` / `lot` / `posting`). A handler gap, hours not a module, and genealogy is the
best candidate for the first painted screen.

---

## Stage 3 — Agent affordances

**Depends on:** Stage 1. Serves Goal 1.

| ID | Work | Size | Depends | Done when |
|---|---|---|---|---|
| T-50 | Domain error tokens | M | — | An agent can distinguish a held lot from a malformed identifier without parsing English. Additive as a detail code; the envelope class is unchanged |
| T-51 | Legal next actions for a record | M | T-24 | Given a live record, the response lists exactly the outgoing edges legal from its current state, each with permission and signature requirement |
| T-52 | A named agent principal | L | — | A non-human session stamps an actor kind that is not a user, and the audit trail names the agent. Today every bearer session is stamped as a user (`crates/wicket-server/src/session.rs:219-225`) |
| T-53 | Dry run | L | T-51 | An illegal transition returns a structured refusal without writing; a legal one reports what would happen without persisting. **Open question first:** a dry run that skips hooks lies, and one that runs them may write. Decide the hook phase before building |
| T-54 | Module conformance suite | L | T-21, T-22 | An empty module fails it and an existing module passes it. Today a new module is judged by imitation |
| T-55 | Module scaffold generator | M | T-54 | Generates a module that fails its own conformance suite until filled in, and generates no directory the module contract cannot honour |
| T-56 | Publish and hash the introspection snapshot | M | T-24, T-51 | Modules, machines, edges, events, jobs and permissions are enumerable, and the snapshot is hashed into the configuration manifest |
| T-57 | Tool-call surface, optional | M | T-56 | Every tool name equals a mounted operation identifier. Do not start before T-56 or it will duplicate the document badly |
| T-58 | **DONE.** Extend the session-protocol lint to modules | S | — | `just lint-sql` first scan covers `modules/` as well as `crates/` |
| T-59 | **DONE.** Reconcile the module dependency rule with reality | S | T-05 | `PLAN.md` crate graph: Cargo manifests won. Modules depend on `wicket-module` AND the listed kernel crates. Enforcing "wicket-module published interfaces only" would reject every first-party module |
| T-60 | **DONE.** Resolve the phantom UI slot in the module contract | S | — | `docs/03-module-system.md` no longer requires a `ui/` tree. A UI tree is ABSENT (promised: Wave 3 UI) |

---

## Stage 4 — Operator documentation

**Depends on:** Stage 0 for truth repair. Mostly independent of Stages 1 to 3, so it can
run in parallel. Serves Goal 4a.

| ID | Work | Size | Depends | Done when |
|---|---|---|---|---|
| T-70 | **DONE.** Classify every environment variable the source reads | S | — | `docs/12-configuration.md` lists each with its path, classified as required at boot, optional at runtime, test-only or compile-time. `WICKET_REQUIRE_PG` is test-harness only. `WICKET_BLOB_ROOT` is the only hard production boot failure |
| T-71 | Operator documentation index | S | T-11 | An index maps the topics below to files, and the readme points operators there before the plan and design documents |
| T-72 | Prerequisites | M | T-71, T-09 | Operating systems, database version, and the contributor-only tools named as such |
| T-73 | Provisioning: database, the five roles, and the grants | M | T-72 | Reproduces the development role split with production credentials, and includes a check proving the application role cannot write the audit tables |
| T-74 | **DONE.** Configuration, and an example file that can actually boot | M | T-70 | `.env.example` includes `WICKET_BLOB_ROOT` and is a subset of `docs/12-configuration.md`. Development passwords only |
| T-75 | Choosing a profile | M | T-71 | A table generated from the profile files, stating that a required signature edge under a no-signature gate fails at startup |
| T-76 | First boot | M | T-73, T-74, T-75 | Each step carries a command, an expected result and an independent check. Distinguishes the product's migrate subcommand from the contributor recipe, which are different paths and desynchronise schema history if mixed |
| T-77 | Day one on the shop floor | L | T-76 | Receive, build, complete and trace, using only routes that exist, each with an expected status |
| T-78 | Day two: backup, restore, upgrade | M | T-76 | Uses the tools that exist today and names both the cluster and the blob root. A database-only restore is stated to be incomplete. The promised backup command is tagged absent |
| T-79 | Qualification, without overclaiming | M | T-76 | Maps the boot integrity check and the named tests onto qualification vocabulary while quoting the regulatory document's own refusal to claim validation |
| T-80 | Hardening and observability | M | T-73 | Trust model, listening address, blob permissions; and a troubleshooting table keyed by real symptoms |
| T-81 | Documentation lints | M | T-74, T-77 | CI fails when a variable in the source is missing from the configuration canon, when a procedure names a command that does not exist and is not tagged absent, or when a document claims validation |

---

## Stage 5 — Migration

**Depends on:** Stage 2 for a stable API to load through, and a published catalogue to
map onto. Serves Goal 3. This is the largest stage and the only one starting from
nothing.

| ID | Work | Size | Depends | Done when |
|---|---|---|---|---|
| T-90 | Mapping file schema and an empty first pack | S | — | Files validate against a schema; CI fails when a source field has no stated disposition |
| T-91 | Staging schema | M | T-90 | Raw payload, source key, freeze timestamp and extract run identifier, with forward and reverse migrations |
| T-92 | Incumbent extractor | L | T-91 | Authenticates, pins the endpoint and contract version, pages by key, and loads fixtures into staging without needing a live tenant |
| T-93 | Point-in-time quantity snapshot | M | T-92 | A freeze-window snapshot is captured as expected-balance rows |
| T-94 | Load units, items, sites and locations through the public API | L | T-92, T-43 | A fixture round-trips into the real tables through HTTP |
| T-95 | Load lots and serials with the identifier law enforced | M | T-94 | An identifier that fails a kernel law never lands in the kernel field; the original is preserved in a cross-reference |
| T-96 | Opening balances | L | T-93, T-95 | On-hand read back through the API matches the snapshot |
| T-97 | Open work orders | L | T-96 | Counts and issued quantities match the extract; the numbering counter is advanced past the imported maximum |
| T-98 | Custom field mapping | M | T-94 | Every incumbent attribute appears in the pack, mapped or explicitly dropped |
| T-99 | Imported evidence path | M | T-94 | Incumbent attachments and sign-off records land as evidence documents. **Zero rows are created in the signature table.** This is the line that must not be crossed |
| T-100 | Reconciliation report | M | T-96, T-97 | Quantity, open work orders, lot status and genealogy completeness produce a pass or fail; under the regulated profile an unexplained genealogy gap fails |
| T-101 | Cutover runbook | S | T-100 | Dry run, parallel run with the incumbent still the system of record, freeze, go-live, and a rollback that is simply staying on the incumbent |
| T-102 | Out-of-scope register | S | T-90 | Published in the run manifest up front. Includes the general ledger, credentials, live signatures, and every catalog-later module |
| T-103 | Adapter boundary and a second-incumbent skeleton | M | T-92 | Incumbent-specific strings appear only under that adapter's directory, enforced by a lint; the first incumbent is the only complete implementation |
| T-104 | Optional history replay | L | T-96, T-100 | Behind a flag, off by default except where the regulated profile requires genealogy on stock still held |

---

## Decisions the owner still owes this backlog

These block specific items and cannot be resolved by a contributor.

1. **Accept or reject [ADR 0010](docs/adr/0010-one-registry.md).** All of Stage 1 hangs
   on it. Rejecting it means Goals 1 and 2 are pursued separately and some work is done
   twice; that is a legitimate choice, but it should be a chosen one.
2. **The agent actor model.** A new actor kind is clearer in a regulatory review; reusing
   the service principal with an acting-for field is a smaller change. Blocks T-52.
3. **Which internal functions are exempt from Goal 2.** The allowlist is the honesty
   mechanism. If it becomes a junk drawer, Goal 2 is dead. Blocks T-43.
4. **Whether install-time subcommands belong on the API at all,** given they run before
   the server binds. Blocks T-38.
5. **Opening balances only, or full history replay,** as the migration default. Regulated
   shops need genealogy for stock they still hold, and full replay may be too large to
   post through the API. Blocks T-104.
6. **Whether to wait for a bill-of-materials module** before claiming manufacturing
   migration. Without one, product structure cannot come across at all. This is the
   largest functional hole in Goal 3.
7. **How an outside pull request reaches the private mirror,** given the standing law
   that the mirror receives landings first. Blocks the merge policy in T-05.
8. **Repository merge settings.** `CONTRIBUTING.md` §7 and `GOALS.md` GOV-5 state
   squash-merge is disabled. The GitHub repository has `allow_squash_merge`,
   `allow_rebase_merge` and `allow_merge_commit` all true. The documents describe a
   rule the platform does not enforce. This is an owner Settings change, not a file
   change. Also decide: branch protection on `main` so the fast-forward rule is
   real, and whether private vulnerability reporting is on. Blocks T-17.
9. **Whether `just ci` must run the acceptance suite.** Resolved 2026-09-14 by
   the docs path (T-15): `just ci` is the offline lint/lib gate; `just ci-db` is
   slice acceptance. Reopen only if `ci` itself should grow a no-database
   integration-test recipe.

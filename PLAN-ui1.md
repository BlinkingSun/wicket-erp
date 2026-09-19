# PLAN — Wave ui1: reach a usable first-party UI

Audience: contributor and agent. Status: draft, pending plan audit.
Task `erp`, wave `ui1`, roster rev 6, check profile `production`.
Base: `main` @ `6e3c80a`.

**Conforms to:** [GOALS.md](GOALS.md); ADR [0009](docs/adr/0009-ui-stack.md),
[0010](docs/adr/0010-one-registry.md), [0011](docs/adr/0011-openapi-schemas.md),
[0012](docs/adr/0012-same-origin-ui.md); [DESIGN.md](DESIGN.md);
[docs/13-ui-wrapper-contract.md](docs/13-ui-wrapper-contract.md); TODO.md T-35 and Stage 2.

---

## 1. The one sentence

The only open issue in the repository and the reason there is still no usable UI are the
same thing, and this wave closes it: **the served OpenAPI document does not describe what
the engine accepts or returns, so the generated TypeScript client ADR 0009 binds the
interface to cannot exist, so every screen is hand-written `fetch` against a document that
cannot check it — and nobody can log in anyway.**

## 2. What is actually true today

Measured at `6e3c80a`, not claimed:

| Fact | Evidence |
|---|---|
| 65 operations are declared | `crates/wicket-server/src/capabilities.rs` (`KERNEL` + `MODULE`) |
| 19 of them carry typed schemas | `SCHEMA_CAPABILITIES`, `crates/wicket-server/src/openapi.rs:34-54` |
| The other 46 are a path index | issue #1: a client generated from it is `Promise<unknown>` |
| Issue #1 is ACCEPTED, not implemented | ADR 0011 accepted; `schemars` is already on the allow-list (`Cargo.toml:71`); GOV-7 says the allow-list edit lands **with** T-35 Wave 1 |
| Signature meaning is a literal constant on every edge | `crates/wicket-server/src/openapi.rs` ~465-470, named in TODO T-35 |
| ADR 0012 is ACCEPTED and unimplemented | no `ServeDir` / `rust-embed` anywhere in `crates/wicket-server` |
| The engine does not serve the SPA | `docs/13-ui-wrapper-contract.md` §1, `crates/wicket-server/src/http.rs` |
| There is no generated TS client | `apps/wicket-web/src/api/` is hand-written `fetch` + hand-written types |
| Three screens exist | Item Master, Genealogy, Shop Floor Terminal |
| **You cannot log in** | `src/router.tsx` has no auth route; no session guard on any route |
| **You cannot find anything** | `/office/items/$itemId` and `/floor/work-orders/$workOrderId` take a **UUID in the URL**. There is no list and no search. `OfficeHome` / `FloorHome` are the only landing pages |

The mockups are approved and the icon is done (`design/`), DESIGN.md §10 records the signed
visual gate. So the UI gate is **not** what is blocking. Plumbing is.

## 3. The chain, in dependency order

```
T-35 Wave 1: schemas on all 65 operations        (closes issue #1)
      |
      +--> generated TypeScript client + drift gate     (ADR 0009 satisfied)
      |
ADR 0012: engine serves the SPA same-origin       (one process, one port, one URL)
      |
      +--> login + session guard                  (the app becomes reachable by a human)
              |
              +--> find-things screens: item list/search, work-order list
                      |
                      +--> the three built screens stop being UUID-only dead ends
```

Nothing in this chain is a research question. Every step has an accepted ADR or a TODO row
behind it. That is why this wave can run wide.

## 4. Tracks and lanes

Partition is by **file ownership**, not by difficulty. The exec-master authors the binding
manifest; this is the shape it must honour.

### Track A — Typed schemas (closes issue #1, TODO T-35 Wave 1)

The hazard is that every batch wants to edit `openapi.rs`. The wave therefore takes the
structural fix FIRST and in its own lane, and only then fans out:

- **A0 — schema registration seam.** Replace the single flat `SCHEMA_CAPABILITIES` list with
  a per-area registration module so each later lane owns a whole file and never touches
  `openapi.rs`. A0 is a **gate lane**: nothing in A1..An dispatches until it lands. Owns
  `crates/wicket-server/src/openapi.rs` and the new `src/schemas/mod.rs`.
- **A1..An — one lane per handler area** (inventory, locations/lots, production/work orders,
  documents/print/customfields, kernel admin, genealogy/jobs). Each owns its own
  `src/schemas/<area>.rs` and the response types in its own `src/handlers/<area>.rs`.
  Batches come from `_team/reports/sweep-schemas.md`, which enumerates the 46 by file.
- **A-sig — per-edge signature meaning.** The second half of T-35: the document must state
  each edge's own meaning, not one constant stamped on all of them. Separate lane, separate
  code path, owns the signature block in the document builder.

Kernel wire primitives `AnyQuantity` and `MoneyWire` are **excluded** from the derive — ADR
0011 and the issue-1 comment both fix this, their encodings are hand-specified in `docs/10`
§3.1-§3.2. A lane that derives `JsonSchema` on either has failed its spec.

### Track B — The client and the origin (runs parallel to A; integrates after it)

- **B1 — generated TypeScript client + drift gate.** Generator chosen in
  `_team/reports/sweep-tsclient.md`. Committed output, a `just` recipe to regenerate, and a
  CI step that fails on divergence — modelled on `scripts/lint-openapi-fixture.sh`, which
  already gates the operation list the same way. Owns the generator config, the generated
  directory, the recipe, the script.
- **B2 — ADR 0012 implementation.** The engine serves the built UI from its own origin.
  Unknown `/api/v1/*` must still return the JSON 404 envelope; only non-API paths fall back
  to `index.html`. Owns `crates/wicket-server/src/http.rs` static mount + `config.rs` asset
  root + `docs/12-configuration.md` row.
- **B3 — `just demo` becomes one command.** Build the UI, boot the engine, seed, open one
  URL. Owns the `justfile` demo recipe and `README` §Running it.

### Track C — The UI a person can use (the point of the wave)

- **C1 — login, session, and the auth guard.** The documented handshake is in
  `docs/AGENT-UI-CONTEXT.md`: `POST /api/v1/identity/login`, cookies + `X-CSRF-Token` for
  same-origin, Bearer for native. Needs a login screen per DESIGN.md, a session store, the
  CSRF header on every mutating call, a 401 → login redirect, and logout. Owns
  `src/features/auth/*`, `src/api/http.ts`, and the auth routes in `src/router.tsx`.
- **C2 — item list and search (office).** Replace the UUID-only dead end. `listItems` and
  `resolveItemByNumber` already exist and are already typed. Owns
  `src/features/items/ItemList*.tsx` and the office index route.
- **C3 — work-order list (floor).** Same defect, floor side, floor ergonomics: targets
  ≥ 64×64 px, 16 px gap, no nav rail. Owns `src/features/floor/WorkOrderList*.tsx` and the
  floor index route.
- **C4 — app shell.** Mode switch, navigation from `getNavigation`, session identity in the
  chrome, and the token pass so all three modes share tokens and never share layouts.
  Owns `src/modes/*` and `src/styles/*`.

`router.tsx` is touched by C1, C2 and C3. **One writer per file per wave** (CONTRIBUTING §7,
AGENTS.md). C1 owns `router.tsx`; C2 and C3 export their routes and C1 mounts them, or the
exec-master sequences C2/C3 behind C1. The manifest must state which.

### Track D — Mounting what the screens need

From `_team/reports/sweep-mounts.md`: only the MUST rank runs in this wave. A capability
whose Rust side is `unimplemented!()` is not a mounting job and does not enter the manifest.

## 5. Design guidelines for every Track C lane

Binding. These restate DESIGN.md and the roster; a lane that breaks one has failed its spec.

- **Dark mode is the default.** Light mode does not get built in this wave.
- **No emojis. Anywhere.** Status is word + shape + colour, and colour is never the only
  signal.
- **No lorem ipsum and no invented data.** Real manufacturing content in every fixture. Never
  render a quantity, lot or status the engine did not return — `src/features/floor/unbacked.ts`
  is the existing pattern for this and new screens follow it.
- **Identifiers and quantities are monospace with tabular figures.**
- **Three modes, shared tokens, never shared layouts.** Office is dense. Floor is sparse with
  ≥ 64×64 px targets and no nav rail. Quality is structural.
- **A destructive action never sits where a confirming action sits on another screen.**
- **Animation only where it carries meaning** — state transitions and loading, never decoration.
- Every new screen reuses `src/styles/tokens.css`. A lane that introduces a hard-coded colour
  has failed its spec.

## 6. Acceptance for the wave

1. `GET /api/v1/openapi.json` carries `requestBody`, response `content` and `parameters` for
   **every** operation in the capability table, and the golden fixture gate still passes.
2. A TypeScript client generates from that document with no `unknown` in the public surface,
   and CI fails if the committed client drifts from the document.
3. One command builds and serves the whole product on one port, same-origin, CORS still absent.
4. A human opens that URL, logs in, lands somewhere real, **finds an item without knowing its
   UUID**, opens a work order on the floor, and runs a genealogy trace.
5. `just ci` and `just ci-db` green. The TypeScript app is gated too — if it is not in `just
   ci` today (see `_team/reports/sweep-gates.md`), adding it is part of B1.
6. Issue #1 closes with the implementing commit, per GOV-7.

## 7. What this wave deliberately does not do

Stage 5 migration. Tauri packaging. Light mode. Custom fields beyond items. The general
ledger. T-43's coverage gate. Naming them here is how they stay out of a lane charter.

---

## 8. Research amendments — BINDING, and they supersede §2 and §4 where they disagree

Six research lanes and one decision lane ran before this plan was final. What they found
changes the plan; this section is the authority.

### 8.1 The operation count in §2 was wrong

`_team/reports/sweep-schemas.md`. The capability table is **72 operations** (42 `KERNEL` +
30 `MODULE`, asserted by `table_has_the_mounted_count` and `slice.rs`), not 65. **19** carry
schemas, so **53 are untyped**, not 46. Issue #1 and `docs/13-ui-wrapper-contract.md` both
say 65; they are stale and the wave corrects them. Any lane charter quoting 65 is quoting a
stale document.

### 8.2 Track A's partition is five batches, and one of them is not lane-sized

Untyped operations group by handler file into: **I** identity writes (7,
`handlers/identity.rs`) · **C** custom fields (5, `handlers/customfields.rs`) · **D**
documents (5, `handlers/documents.rs`) · **P** print (3, `handlers/print.rs`) · **M**
everything else (**33**, `handlers/mod.rs` plus module `api.rs`/`domain.rs` files).

`handlers/mod.rs` is one file, so under one-writer-per-file M is one 33-operation lane.
That is the wave's width bottleneck and its longest pole. The wave therefore takes **two**
gate lanes before the fan-out, not one:

- **A0 — schema registration seam.** Owns `crates/wicket-server/src/openapi.rs` and creates
  `src/schemas/{mod,identity,customfields,documents,print,kernel}.rs`. Replaces the flat
  `SCHEMA_CAPABILITIES` const and the `schema_binding` match with `schemas::all()` folding
  per-area registrations. After A0, **no schema lane ever edits `openapi.rs`**. A0 must not
  claim `required_signature` — that belongs to A-sig.
- **A0b — split `handlers/mod.rs` into area files.** Pure mechanical move, no behaviour
  change, `just ci-db` green and the golden OpenAPI fixture byte-identical before and after.
  This is what turns Batch M from one lane into five. If A0b proves unsafe, M runs as a
  single long grok lane instead — the manifest must say which, and why.

`capabilities.rs` is **not** edited by any schema lane. Ids, methods, paths and permissions
are already there, and editing it "in lockstep" would create the second source of truth
ADR 0010 exists to prevent.

### 8.3 A-sig has a real, located defect

The per-edge signature meaning must come from `SignatureRequirement.meaning` on the
state-machine edge, reaching OpenAPI through `Engine::edges_for_manifest()` →
`Kernel::refresh_signature_edges` → `SignatureEdge::Required { meaning, .. }`. `openapi.rs`
must read that, never a string literal. Where the profile says `NotRequired`, the
`x-wicket-signature` extension is **absent**, not present-and-empty. Where one HTTP
operation can fire more than one edge, it is not advertised at all.
`docs/10-api-conventions.md` §5.1 still examples the old constant and is corrected in the
same change.

### 8.4 ADR 0012 does not trip the allow-list; a new crate would

`_team/reports/sweep-sameorigin.md`. `tower-http` is already allow-listed at `Cargo.toml:67`
with `features = ["trace"]`. Adding **`fs`** to that existing entry enables `ServeDir` and is
**not** a GOV-7 escalation — GOV-7 governs new crate *names*. `rust-embed` **would** be an
escalation and is not needed; embedding stays deferred. The `cors` feature stays off.

Configuration follows `WICKET_BIND` (optional, `Config`-owned), never `WICKET_BLOB_ROOT`
(required-at-boot, env-only): **`WICKET_UI_ROOT`** / `--ui-root` (no clap default, so env and
TOML can win) / TOML `ui_root`, defaulting to unset = serve no static files, API only, and
reverse-proxy deployments keep working. When it **is** set and names a path that is missing,
not a directory, or has no `index.html`, **boot fails** — silent API-only would let
`just demo` look like it served a UI that was never built. `docs/12-configuration.md` §3 and
§5 gain the row in the same change, `.env.example` gains it commented, and
`WICKET_BLOB_ROOT` stays the only hard production boot failure.

`docs/13-ui-wrapper-contract.md` §1 and `docs/AGENT-UI-CONTEXT.md` currently say the engine
does **not** serve the SPA. That is true today. Per GOV-12 they flip to "**may** serve the
SPA from `WICKET_UI_ROOT`; CORS remains absent" **in the same change that ships the mount**,
and not one commit earlier.

### 8.5 The npm governance question is ruled, and it is not an RFC

`_team/reports/DECISION-npm-allowlist.md`. GOV-7's text covers npm; its mechanism does not
exist for npm, which by GOV-12 is a defect in GOV-7 rather than permission to skip it. The
ruling is **amend GOV-7 in this wave and land the generator under it, in one commit**:
`GOALS.md` GOV-7 row replaced (two closed allow-lists; `dependencies` escalates like a crate,
`devDependencies` is a plain pinned commit), `CONTRIBUTING.md` §3 trigger list extended, and
`TODO.md` gains **T-105** for the absent lint. `react` / `react-dom` / TanStack are
grandfathered as the list's opening contents. **No RFC issue is opened.** `openapi-typescript`
is a pinned devDependency and must never be imported by shipped code.

### 8.6 The generator is `openapi-typescript`, and the client needs a document fixture first

`_team/reports/sweep-tsclient.md`. Types-only, zero runtime bundle cost, tolerates the
document's partial typing, single committed output at
`apps/wicket-web/src/api/generated/openapi.ts`. `http.ts`, `client.ts`, `view-models.ts`,
`format-currency*` and `map-*.ts` all stay; `src/api/types/*.ts` are deleted incrementally as
their wire types move to `components['schemas'][...]`.

The drift gate needs an input the repo does not have: a **committed canonical
`crates/wicket-server/tests/fixtures/openapi-document.json`**, produced from the same code
path as `GET /api/v1/openapi.json`, so the client lint runs offline. That is its own lane
(**B1a**) and it is ADR 0011's own outstanding consequence, not a new idea. `just
openapi-document` writes it, `scripts/lint-openapi-client.sh` and the existing
`scripts/lint-openapi-fixture.sh` style gate it, and regeneration is never an implicit CI
dependency.

**Ordering correction to §4:** ADR 0009 says the client "cannot meaningfully exist" until the
schemas land. B1's *infrastructure* — devDependency, recipe, script, CI wiring — is
independent and runs in parallel with Track A. B1's *committed output* must be regenerated
after Track A integrates, or it ships a client full of `unknown`. The manifest must place
that regeneration after A, not beside it.

### 8.7 Track C: the app cannot be logged into, and `router.tsx` is contended

`_team/reports/sweep-uiauth.md`. Confirmed: there is no login route, no `postJson`, no CSRF
header, no session store, no 401 redirect, no logout. Every read the UI makes requires a
session and the UI cannot mint one. The shortest path to "a person opens a browser, logs in,
finds an item, sees a work order, runs a trace" is four lanes, and only the first is a gate:

- **C1 `ui-auth-handshake`** — gate. Login route and screen, `postJson`, `X-CSRF-Token` on
  mutations, session holder (memory + csrf; cookies ride `credentials: include`), 401 →
  login with cache clear, logout that drops both cookies because the server does not.
- **C2 `ui-item-finder`** — `listItems` with `number_prefix` search, compact office table,
  row → `/office/items/$itemId`. Do not take BOM or Save.
- **C3 `ui-floor-wo-finder`** — `listWorkOrders`, floor-ergonomic pick list, tap →
  `/floor/work-orders/$workOrderId`, scanned UUID navigates. Clock-on and quantity stay
  disabled; they are engine-unmounted and faking them breaks the never-render-unbacked-data
  rule.
- **C4 `ui-genealogy-usable`** — poll `GET {result_url}` for async traces with
  `progress_pct` / `progress_note`, render on `Succeeded`. Optional `listLots` picker.

**The `router.tsx` collision is resolved by seam, not by sequencing.** C1 is the sole writer
of `src/router.tsx`. It writes the final route tree in one pass — `/login`, the `beforeLoad`
auth gate on all three modes, and the index routes for items, work orders and lots —
importing `ItemListScreen`, `WorkOrderPick` and the genealogy additions from paths it creates
as minimal honest stubs. C2, C3 and C4 then own those feature files exclusively and never
touch the router. This is the same move as A0: one writer creates the seam, the fan-out fills
it.

Do **not** put these on the shortest path: per-mode accent tokens, sticky item header, table
virtualization, BOM rows, CLOCK ON / REPORT QTY / REPORT SCRAP, traveler and serial scanning,
the e-sign dialog, the left-to-right genealogy graph, the recall rail, Export DHR. Every one
of them is either purely visual or backed by an operation the engine has not mounted.
`view-models.ts` already marks those `UNBACKED_*` and that marking is the contract.

### 8.8 Acceptance commands, per lane class

`_team/reports/sweep-gates.md`. A charter names exactly one of these and a lane does not
report DONE without its output in the lane report:

- Rust lane: `just ci-db` (it already runs `just ci`; `slice.rs` is excluded by `--lib`, so
  `just ci` alone is not acceptance). Prefix `just db-up && just db-reset` on a fresh
  worktree. A lane adding a capability row runs `just openapi-fixture` first — the lint
  compares, it does not write.
- TypeScript lane: `just ui-install && just ui-lint && just ui-build && just ui-test`. Never
  `just ci`.
- Both: the TypeScript sequence, then `just ci-db`.
- Every commit: `git commit -s` (DCO), and no `todo!()` / `unimplemented!()`.

### 8.9 Track D is cut from this wave

`_team/reports/sweep-mounts.md`, verdict: **none of T-31, T-32, T-33 or T-34 is a MUST for
the three approved screens.** The findings that produce that verdict are worth keeping:

- Item-master SAVE is already mounted (`updateItem`). Genealogy is already reachable
  (`traceGenealogy` + `getGenealogyJob` + `getImpact`) — the gap is that
  `GenealogyScreen.tsx` renders "job results are not polled" and stops, which is C4's job,
  not a mount.
- T-33's TODO sentence ("returns a job id into a route that does not exist") is **stale**;
  T-30 mounted `getGenealogyJob`. Same for parts of T-34. The backlog rows are corrected in
  this wave's docs pass, not by writing code against them.
- The shop-floor kiosk genuinely is a toy without writes — but the writes it wants
  (`CLOCK ON`, `REPORT QTY`, `REPORT SCRAP`) **do not exist as mountable Rust functions at
  all**. They are not in T-31..T-34 and mounting nothing would be inventing a write. Work-order
  `cancel` is the same: a machine edge with no store function and zero tests. Implement first,
  mount second, in a later wave.

So Track D is deferred whole. Two further reasons, which are mine rather than the sweep's:
a new capability row changes the golden OpenAPI fixture that every Track A lane is being
graded against, and every mount lane would contend for `capabilities.rs` and `http.rs` —
importing exactly the collision class this wave is structured to avoid. Mounting resumes in
wave ui2, against a document that by then describes itself.

One thing does get carried into this wave, because it is a defect rather than a feature: the
module `operation_id` collision the sweep found — `modules/inventory` declares `getDocument`,
which kernel `GET /api/v1/documents/{id}` already uses. It is unmounted today so nothing
breaks, but it is a trap set for the first person who mounts it. Recorded in TODO as part of
the docs pass; renamed to `getInventoryDocument` when it mounts.

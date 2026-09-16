# 0011. Schemas in the served document are derived from Rust types, not hand-written

Audience: contributor. Status: absent.

Status:   **Accepted** (2026-09-15)
Date:     2026-09-15
Decider:  project owner

## Context

ADR 0009 binds the interface to **one generated TypeScript client**, and ADR 0010 makes the
capability table the single source that generates the router and the document. The served
document does not currently carry enough to generate anything.

`crates/wicket-server/src/openapi.rs` emits, per operation, an `operationId`, an
`x-wicket-permission`, and a `responses` block of bare descriptions. There is no `requestBody`,
no response `content`, and no `parameters` — `/api/v1/items/{id}` does not declare that `{id}`
exists. `components.schemas` holds exactly one entry, `ErrorEnvelope`.

A client generated from that document is `getPrincipal(): Promise<unknown>` — named functions
over untyped `fetch`. T-44's fixture gate makes the **path set** trustworthy; it cannot detect
drift in schemas that do not exist. So the wrapper contract ADR 0009 requires is unreachable,
and mounting further operations lengthens the index without typing the client.

The project's measured law applies:

> Every rule enforced by a tool held. Every rule enforced by prose failed.

Hand-written JSON Schema sitting beside a handler is a rule enforced by prose. It is correct on
the day it is written and silently wrong afterwards.

## Decision

**Schemas are derived from the Rust types the handlers already serialize.**

1. The `json!` blobs handlers emit are promoted to **named `serde` types**.
2. JSON Schema is derived from those types with **`schemars`**, added to the workspace
   allow-list under GOV-7.
3. `openapi.rs` attaches `$ref`s **by capability id**, in a `match` that is a twin of the
   `http.rs` bind, and a **table-walk test** asserts every in-scope capability has a binding.

   **Corrected 2026-09-16.** This clause first said "a capability with no schema binding fails to
   compile". That is not achievable and the claim is withdrawn: `Capability.id` is
   `&'static str`, a `match` on `&str` is never exhaustive, and a catch-all arm is mandatory —
   so a deleted arm compiles and falls through. `http.rs` already lives with this, pairing its
   `match` with the runtime test `every_capability_has_a_handler`. The guarantee is therefore a
   **failing test**, not a failing build, and the schema binding gets the same treatment as the
   handler binding rather than a stronger one it cannot have.
4. **`schemars` does not own routing.** `utoipa` was rejected for exactly this: it wants route
   registration, which is ADR 0010's job. The capability table remains the only route source.
5. **`JsonSchema` is not derived on kernel wire primitives** (`AnyQuantity`, `MoneyWire`). Those
   have hand-specified string encodings in `docs/10` §3.1-§3.2; a derived schema would either
   contradict the document or leak `rust_decimal` internals into the public contract.

## Consequences

**Cost.** One third-party crate on a deliberately closed allow-list, plus the derive burden on
every response type. The allow-list exists so this is an escalation rather than a commit, and
this ADR is that escalation.

**What it buys.** A schema cannot silently diverge from the handler that produces it, because
both are the same Rust type. That is the property hand-written schemas cannot have at any price.

**What still needs a gate.** The table-walk test proves a capability *has* a schema; it does not
prove the public contract did not change. A committed full-document fixture, generated from the
same derive path and diffed in `just ci`, is the T-44-shaped review gate on that. It is a review
signal, not a second source of truth.

**The orphan rule decides where the derive lives — amended 2026-09-16.** `JsonSchema` can only
be derived where the type is defined. The response types of the operations that matter —
`ItemBody`, `LotBody`, `TraceBody`, `Tree`, `Impact`, `JobStatus` — live in `modules/items`,
`modules/lots`, `modules/genealogy` and `wicket-jobs`, so `wicket-server` cannot give them
schemas. Restricting the first wave to `wicket-server`-owned types would schema the session shell
and `getOnHand` and leave every operation the approved mockups press untyped.

**Therefore `schemars` is added to the crates that own the types**, and the derive sits on the
type itself. The two alternatives were rejected: duplicating each DTO inside `handlers/` and
deriving on the copy contradicts this ADR's own "both are the same Rust type" and reintroduces
the drift it exists to prevent; a separate crate of schema wrappers hits the `AGENTS.md` ban on
new crates while Goal 2 is false.

The cost is stated plainly: `schemars` reaches four more crates than the original decision
implied. That is the price of the derive-don't-duplicate rule, and it is cheaper than a second
hand-maintained catalogue of shapes. Kernel wire primitives (`AnyQuantity`, `MoneyWire`) and the
typed ids in `wicket-core` remain **excluded** — their encodings are hand-specified in `docs/10`.

**Reversal cost.** Moderate and bounded: the derives come off, the `$ref` match becomes literal
JSON, and the document shape is unchanged. This is recorded as an ADR because the decision
determines how schemas are produced for the rest of the project, not because it is hard to undo.

## Alternatives

**Hand-written JSON Schema in the capability table.** No dependency. Rejected: 65 operations by
request and response, maintained by hand, with nothing able to detect divergence from the
handlers. This is the failure mode the project's law names.

**`utoipa`.** More batteries included, and rejected because it wants to own route registration,
which fights ADR 0010's single registry.

**Extractor-driven generators.** Blocked — handlers take `Bytes` in places, so the extractor
type does not describe the body.

**Do nothing and hand-write the TypeScript client.** Rejected: it works for one screen, and then
the first-party UI stops being the paying customer that keeps the document honest, which is the
mechanism ADR 0009 relies on.

## Sequencing

Path parameters, query parameters, required headers and per-edge signature meaning need **no**
dependency and land first (T-35 Wave 0). Typed bodies follow under this ADR, smallest-useful
first: the operations the approved genealogy and item-master mockups press.

Evidence: `_team/reports/spike-t35-schemas.md`, `_team/reports/spike-t35-signature.md`.

## Amendment 3 — typed ids are included, not excluded (2026-09-16)

Amendment 2 granted the module crates so the orphan rule could be satisfied. It left a
statement in Consequences that `wicket-core` typed ids remain excluded from the derive. That
statement is **withdrawn**: it cannot be honoured.

`LotBody`, `SerialBody`, `TreeNode`, `Impact` and `JobId` all embed `wicket-core` typed ids.
`JsonSchema` is not derivable for a struct whose fields do not implement it, so excluding the
ids makes every one of those derives fail to compile. The exclusion described an arrangement
that does not exist.

Typed ids are UUID or string newtypes. A derive on them emits
`{"type": "string", "format": "uuid"}` — verified by compiling it, not by reading it — which is
exactly true and carries no maintenance burden. They are therefore **included**.

What the original exclusion was actually protecting is narrower and still stands:
**`AnyQuantity` and `MoneyWire` get no derive.** Their wire encodings are hand-specified in
`docs/10-api-conventions.md` §3.1-§3.2, and a derive would either contradict that document or
leak `rust_decimal` internals into the public schema. They get hand-written schemas quoting
`docs/10`.

The same care applies to any field carrying `#[serde(with = ...)]`: `TreeNode.amount` is a
`Decimal` that serialises as a string, so its schema must say `String`. A derived schema that
disagrees with the wire is worse than no schema, because a generated client will trust it.

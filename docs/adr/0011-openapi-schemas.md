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
3. `openapi.rs` attaches `$ref`s **by capability id**, in a compile-time `match` that is a twin
   of the `http.rs` bind. A capability with no schema binding fails to compile.
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

**What still needs a gate.** Compile-time binding proves a capability *has* a schema; it does not
prove the public contract did not change. A committed full-document fixture, generated from the
same derive path and diffed in `just ci`, is the T-44-shaped review gate on that. It is a review
signal, not a second source of truth.

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

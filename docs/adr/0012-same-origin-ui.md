# 0012. The engine serves the UI from its own origin

- Status: **Accepted**
- Date: 2026-09-16
- Supersedes: nothing. Complements [0009](0009-ui-stack.md).

## Context

The stated architecture is one engine on one node, with UIs on many devices: browsers in the
office, kiosk browsers on the shop floor, and later native wrappers on macOS, Linux, Windows,
Android and iOS.

Today the engine mounts the capability table and nothing else (`crates/wicket-server/src/http.rs:94-116`).
It does not serve the web UI and it sets no CORS headers. This is invisible in development
because the Vite dev server proxies `/api` to `127.0.0.1:8080`, making the UI same-origin by
accident. The first time a UI is opened from a second device, it stops working.

So the question had to be answered before any wrapper packaging: does the engine serve the UI,
or does it grow a cross-origin policy?

## Decision

**The engine serves the built UI from its own origin.** Browser clients — office and floor
alike — are same-origin with the API. **CORS stays deliberately absent.** Native wrappers
authenticate with a Bearer token rather than cookies and are unaffected.

## Why — the evidence, not the taste

The deciding fact is about cookies on a shop floor, and it is not a matter of preference.

Login issues `wicket_session` and `wicket_csrf` with `Path=/`, no `Domain`, **`SameSite=Lax`**,
and `Secure` only when `X-Forwarded-Proto: https` is present
(`crates/wicket-server/src/handlers/mod.rs:238-264`). For a cross-origin `fetch` to carry those
cookies at all, they would have to become `SameSite=None`, and **every browser requires
`Secure` alongside `SameSite=None`, which requires HTTPS**.

A shop-floor LAN does not have a certificate. `docs/10-api-conventions.md` §7 does not assume
TLS either. So cross-origin cookie authentication is not merely more work on this deployment —
**it cannot function on the exact network Wicket is built for**. That is what settles it.

Two supporting facts:

- **No new dependency.** `axum` 0.8.9 and `tower-http` 0.6.11 are already in `Cargo.lock`.
  Serving static files needs `tower-http`'s existing `fs` feature. `AGENTS.md` bans new crates
  and new modules; it does not ban enabling a feature on a crate already present.
- **CSRF is not the obstacle.** `check_csrf` compares `X-CSRF-Token` against the session's
  stored `csrf` (`session.rs:189-201`) and would survive cross-origin. The blocker is purely
  that the session cookie would never arrive.

## Consequences

- The engine gains one static-file route serving the built UI, mounted so it cannot shadow
  `/api/v1/**`. It is the only route outside the capability table, and that exception should be
  stated explicitly wherever the table is described as exhaustive.
- A release must now produce the UI bundle as well as the binary. `apps/wicket-web` remains a
  non-workspace TypeScript app; the build order becomes UI first, then package.
- Development is unchanged: the Vite dev server keeps proxying `/api`, which is already
  same-origin behaviour.
- Serving the UI is **not** required. An operator may put a reverse proxy in front instead and
  get the same same-origin property. The engine serving it is the default that works with no
  extra moving parts.
- The absence of CORS becomes a deliberate, documented property rather than an oversight. Anyone
  adding a CORS layer later is changing this decision, not filling a gap.
- `just demo` should serve the UI too, so that one command yields a system a person can actually
  look at. Until it does, the demo is an API with no face.

## Alternatives considered

**Credentialed CORS with the UI hosted separately.** Rejected. It requires `SameSite=None` and
therefore TLS on a shop LAN that has no certificate. It would also widen the auth surface —
`Access-Control-Allow-Credentials` with a cookie session is exactly the configuration that
rewards a single mistake in origin matching with a cross-site session leak. Rejecting it costs
us nothing that the same-origin path does not already provide.

**A reverse proxy as the only supported answer.** Rejected as the *default* but retained as a
deployment option. It solves the same problem and is what many sites will do anyway, but making
it mandatory means the product does not run without an additional component — which contradicts
"one engine on one node".

**Embedding the UI assets in the binary.** Deferred, not rejected. A single self-contained
binary is attractive for installation, but it couples UI and engine release cadence and makes
the binary large. Serving from a directory first keeps that door open.

## Revisit if

- TLS becomes standard on the shop-floor LAN, at which point cross-origin becomes *possible* —
  though still not obviously better.
- A third-party or customer-built UI must run on an origin we do not control. That is a real
  reason for CORS and should be weighed on its own, not folded into this decision.
- The engine acquires a second non-capability route, which would mean the "one exception" framing
  above needs revisiting rather than extending.

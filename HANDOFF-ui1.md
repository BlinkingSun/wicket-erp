# Handoff — Wicket wave `ui1`, written 2026-09-19 at round close

Audience: the next session (human or agent). Status: current as of this commit.

## 1. Where the tree is

**`main` is at the fully integrated face wave.** It was fast-forwarded from
`integrate/ui1-face` after `just ci-db` returned **EXIT=0 (144 test binaries, 0 failures)**
and the UI gate returned **57 tests green**. Nothing has been pushed to any remote; `origin`
and `dev` are untouched.

**What that means in practice:** Wicket can now be opened and used by a person.
`just demo` builds the SPA and serves engine + UI on one origin. Sign in with **`demo` /
`demo-login`**, and you get an office item list with prefix search, a floor work-order pick
list, an item master, and a genealogy trace that polls a job to completion.

## 2. What is unfinished, and exactly where it lives

The **types wave** — the work that closes GitHub issue **#1** — was stopped mid-flight for
travel. Its state is preserved on branches, not in worktrees (the worktrees were pruned).

| Branch | What it is | Trust level |
|---|---|---|
| `wip/a-locations` | **Complete.** Committed, `just ci-db` exit 0, ownership respected | ready to integrate |
| `wip/a-identity` | **Committed** by the lane before it was stopped | verify its gate, then integrate |
| `wip/a-customfields`, `wip/a-documents`, `wip/a-esign`, `wip/a-inventory`, `wip/a-items`, `wip/a-kernel`, `wip/a-lots`, `wip/a-print`, `wip/a-production` | **UNVERIFIED.** The orchestrator captured each lane's uncommitted diff as a `WIP(...)` commit so nothing was lost. None finished its own `just ci-db`; none was audited. | **do not merge** without re-running the lane |

All eleven are based on `a0-seam` (`3074055`), which is already in `main`, so they rebase cleanly.

Also preserved from earlier rounds, previously at risk of being orphaned:
`wip/backlog-r1`, `wip/genpolish-r1`, `wip/ledgerdup-r1`, `wip/mount1-r1`, `wip/onhand-r1`,
`wip/serial-r1`, `wip/ui3floor-rw1`.

## 3. How to resume the types wave

Everything you need is already written. The specs are in git history on each `wip/` branch at
`SPEC.md`, and the generator that produced them is `_team/state/gen-types-specs.py` — re-run it
with `_team/state/a0-handover.txt` to regenerate all eleven.

```bash
# recreate a lane worktree
git worktree add ERP-wt/wt-a-<area> wip/a-<area>
# the gate environment is NOT in the repo -- see section 5
just db-reset && just ci-db; echo "EXIT=$?"
```

The live manifest is `_team/state/wave-ui1types1.json`. Restart the supervisor with
`nohup ~/agent-team-v5/bin/team-supervisor --task erp --workdir "$(pwd)" &` after deleting
`<state-root>/erp/supervisor.stop`.

**Ownership law for these lanes:** each owns only `crates/wicket-server/src/schemas/<area>.rs`
plus its own handler/type files. **`a-esign` is the only lane permitted to touch
`handlers/mod.rs`** — the five esign handlers stayed there because `tests/esign.rs`
`include_str!`-pins their bodies.

## 4. The order of the remaining work

1. Verify and integrate the eleven schema lanes.
2. `b1-tooling`, then `b1-client` (specs recoverable from the wave manifests). `b1-client` must
   not commit a generated client until every schema lane is in, or eleven lanes regenerate one file.
3. **Phase-end whole-program test** — charter at `_team/state/charter-phase-end.txt`. Its first
   job is PUNCHLIST **P11**, which blocks the gate.
4. `docs-integrate` — corrects 65→72 and the now-false present-tense claims in
   `docs/13-ui-wrapper-contract.md` and `docs/AGENT-UI-CONTEXT.md`.
5. `team-gate`, then close issue #1 with the **measured** count of typed operations.

## 5. Two traps that cost this round real time

**The acceptance gate cannot run from a clean checkout.** `just ci-db` panics
`WICKET_MIGRATE_DATABASE_URL is unset`. There is no `.env` and no `dotenv-load`; `.env.example`
says outright that the process does not load it. Export these, and give concurrent lanes
**distinct database names** or they fight over one cluster:

```bash
export WICKET_TEST_TEMPLATE=wicket_tpl_<lane> WICKET_TEST_DB=wicket_<lane> \
  WICKET_DATABASE_URL="postgres://wicket_app:wicket@127.0.0.1:5432/wicket_<lane>?sslmode=disable" \
  WICKET_MIGRATE_DATABASE_URL="postgres://wicket_migrate:wicket@127.0.0.1:5432/wicket_<lane>?sslmode=disable" \
  WICKET_BOOTSTRAP_URL="postgres://127.0.0.1:5432/postgres?sslmode=disable" \
  WICKET_BLOB_ROOT=/tmp/wicket-blobs-<lane> WICKET_PROFILE=plain-shop WICKET_REQUIRE_PG=1
```

**Never read a gate's result through a pipe.** `just ci-db 2>&1 | tail -20` reports *tail's*
exit code. A `fmt-check` failure was read as a pass that way during this round. Capture `$?`
per recipe.

## 6. The open risks

`_team/reports/PUNCHLIST-ui1.md` has twelve items with owners. The one that matters:

**P11 blocks the wave gate.** `a0-seam` moved 5800 lines and claims the served OpenAPI document
is byte-identical before and after. Its deep audit passed that, but noted the lane's four dumps
looked like "four copies of one document" — which would mean the empty diffs compared a document
to itself. `scripts/lint-openapi-fixture.sh` cannot settle it: it parses `capabilities.rs`
textually and proves only the operation **set**, not the schemas. Until the phase-end test builds
the document at `6e3c80a` and at head on both profiles and diffs them, treat byte-identical as
**audited but not independently reproduced**.

## 7. Reading order for context

`_team/reports/ROUND-ui1-face.md` (what happened and what I got wrong) ·
`PLAN-ui1.md` §8 (the research amendments that corrected the plan) ·
`_team/reports/plan-audit-ui1.md` (the adversarial audit that shaped the wave) ·
`_team/reports/PUNCHLIST-ui1.md` · `_team/reports/RESUME-ui1.md` ·
the six `_team/reports/sweep-*.md` research reports.

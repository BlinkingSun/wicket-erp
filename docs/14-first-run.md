# First run (local demo)

From a clone with PostgreSQL on `127.0.0.1:5432` (Homebrew `postgresql@17` or Docker
`dev/compose.yml`), Rust 1.98+, and Python 3 with the `argon2-cffi` package (`pip install argon2-cffi`).

The demo uses database **`wicket_demo`** (`dev/demo.env`). That is separate from
**`wicket_test`**, which `just ci-db` and `.env.example` use. Demo seed data must not land in the
test database.

## One command

First time on an empty machine, create roles and databases:

```sh
export WICKET_BOOTSTRAP_URL='postgres://127.0.0.1:5432/postgres?sslmode=disable'
just db-up
just demo-db-reset
just demo
```

(`just db-reset` still provisions `wicket_test` for the test harness; use `just demo-db-reset` for
the demo database only.)

Every later start (same data, idempotent):

```sh
just demo
```

## What you should see

The recipe prints the engine URL, health check path, and demo credentials:

- **Engine:** `http://127.0.0.1:8080`
- **Login:** username `demo`, password `demo-login`

After seeding, the API holds items such as `MDS-450-M4x12`, lots linked by heat number, inventory,
and a completed work order so genealogy trace returns a tree.

`just demo` writes `dev/demo-seed-item-id`, `dev/demo-seed-lot-id`, and
`dev/demo-seed-work-order-id` with UUIDs for the screw item, the finished lot `LOT-WO-1847` (backward
genealogy shows titanium consumption), and the completed work order. Paste those into the Item Master,
genealogy, and shop-floor URL paths when the UI asks for an id.

## UI on another machine or port

The Vite dev server proxies `/api` to the engine. Point it at a running server:

```sh
export WICKET_API_ORIGIN=http://127.0.0.1:8080
npm run dev --prefix apps/wicket-web
```

Open the URL Vite prints (default `http://127.0.0.1:5173`).

## Reset

```sh
export WICKET_BOOTSTRAP_URL='postgres://127.0.0.1:5432/postgres?sslmode=disable'
just demo-db-reset
just demo
```

Blob files live under `dev/demo-blobs/` (created automatically).

## Troubleshooting

| Symptom | Fix |
|---|---|
| `postgres is not accepting connections on 127.0.0.1:5432` | Start Postgres (`brew services start postgresql@17`) or `docker compose -f dev/compose.yml up -d`. |
| `WICKET_BOOTSTRAP_URL is required` on `db-reset` / `demo-db-reset` | Export `WICKET_BOOTSTRAP_URL` as in `.env.example` (see table above). |
| `seed-demo-api: login failed` | Run `just demo` again after migrate; ensure `demo` user exists (`dev/demo/bootstrap-identity.sql` runs automatically). |
| `ModuleNotFoundError: argon2` | `pip install argon2-cffi` (bootstrap hashes the demo password). |
| Port 8080 already in use | Stop the old engine (`lsof -ti:8080 \| xargs kill`) or set `WICKET_BIND` in `dev/demo.env`. |
| `demo bootstrap: admin role missing` | Run `wicket migrate` first (`just demo` does this). |

## How the first login exists (demo only)

The HTTP API cannot bootstrap itself today:

- `POST /api/v1/identity/principals` (`createPrincipal`) requires an existing session with
  `identity.manage`.
- Profile TOML seeds **roles** (`seed_profile` / `seed_bundles`) but not human users.
- Migrations insert only `system` and `migration` service principals (`identity` migration); there is
  no `assign_role` HTTP operation (see slice test comment on W3a).

`just demo` therefore applies `dev/demo/bootstrap-identity.sql` **after** `wicket migrate`: a dev-only
SQL transaction (with audit GUCs) creates user `demo`, stores an Argon2id login hash, assigns the
profile `admin` role, and adds extra permission keys the profile admin bundle omits but API seeding
needs. All **business** data is still created through `/api/v1` in `scripts/seed-demo-api.sh`.

## NOT FOR PRODUCTION — identity bootstrap gap

**The SQL in `dev/demo/bootstrap-identity.sql` is a development shim, not a product feature.**

Every real installation still needs a first human (or break-glass) principal. Today the engine offers
no supported path for that: not through the `wicket` CLI, not through HTTP without an existing
`identity.manage` session. The demo recipe papers over the gap by running raw SQL as
`wicket_migrate`.

That is acceptable for a local demo. It is **not** acceptable to copy this SQL into a deployment
script, a Helm chart, or an operator runbook. Doing so bypasses whatever bootstrap, audit, and
account-provisioning controls production must have, and it will quietly become “how we always did it”
if nobody documents the gap.

**Production needs a real bootstrap path** — for example a one-shot `wicket identity bootstrap-admin`
subcommand (or equivalent) that creates the initial principal through the same domain rules as the
API, emits an auditable event, and is explicitly scoped to first-run. Until that exists, treat
first-account creation as an open product requirement, not something `dev/demo/bootstrap-identity.sql`
solves.

# First run (local demo)

From a clone with PostgreSQL on `127.0.0.1:5432` (Homebrew `postgresql@17` or Docker
`dev/compose.yml`), Rust 1.98+, and Python 3 with the `argon2-cffi` package (`pip install argon2-cffi`).

## One command

First time on an empty machine, create roles and the app database:

```sh
export WICKET_BOOTSTRAP_URL='postgres://127.0.0.1:5432/postgres?sslmode=disable'
just db-up
just db-reset
just demo
```

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
just db-reset
just demo
```

Blob files live under `dev/demo-blobs/` (created automatically).

## Troubleshooting

| Symptom | Fix |
|---|---|
| `postgres is not accepting connections on 127.0.0.1:5432` | Start Postgres (`brew services start postgresql@17`) or `docker compose -f dev/compose.yml up -d`. |
| `WICKET_BOOTSTRAP_URL is required` on `db-reset` | Export `WICKET_BOOTSTRAP_URL` as in `.env.example` (see table above). |
| `seed-demo-api: login failed` | Run `just demo` again after migrate; ensure `demo` user exists (`dev/demo/bootstrap-identity.sql` runs automatically). |
| `ModuleNotFoundError: argon2` | `pip install argon2-cffi` (bootstrap hashes the demo password). |
| Port 8080 already in use | Stop the old engine (`lsof -ti:8080 \| xargs kill`) or set `WICKET_BIND` in `dev/demo.env`. |
| `demo bootstrap: admin role missing` | Run `wicket migrate` first (`just demo` does this). |

## How the first login exists

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

#!/usr/bin/env bash
# ADR 0009: committed TypeScript client must match the plain-shop OpenAPI document fixture.
# Offline; needs Node on PATH. POSIX bash 3.2.
# Generator is the pinned devDependency (openapi-typescript@7.13.0), invoked with
# --no-install so this script does not fetch. Regeneration is `just openapi-client`
# (this script --write). Never a dependency of ci / ci-db. `just ui-lint` runs compare mode.
set -eu

ROOT="${REPO_ROOT:-$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)}"
DOC="$ROOT/crates/wicket-server/tests/fixtures/openapi-document.json"
OUT="$ROOT/apps/wicket-web/src/api/generated/openapi.ts"
WEB="$ROOT/apps/wicket-web"

WRITE=0
if [ "${1:-}" = "--write" ]; then
  WRITE=1
elif [ "${1:-}" != "" ]; then
  echo "lint-openapi-client: unknown argument: $1 (expected --write or none)" >&2
  exit 1
fi

if [ ! -f "$DOC" ]; then
  echo "lint-openapi-client: missing document fixture: $DOC" >&2
  echo "lint-openapi-client: regenerate with: just openapi-document" >&2
  exit 1
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/lint-openapi-client.XXXXXX")"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT
trap 'cleanup; exit 130' INT TERM

npx --prefix "$WEB" --no-install openapi-typescript "$DOC" -o "$tmp/openapi.ts"

if [ "$WRITE" -eq 1 ]; then
  mkdir -p "$(dirname "$OUT")"
  cp "$tmp/openapi.ts" "$OUT"
  echo "lint-openapi-client: wrote $OUT"
  exit 0
fi

if [ ! -f "$OUT" ]; then
  echo "lint-openapi-client: missing committed client: $OUT" >&2
  echo "lint-openapi-client: regenerate with: just openapi-client" >&2
  exit 1
fi

if ! diff -q "$tmp/openapi.ts" "$OUT" >/dev/null; then
  echo "lint-openapi-client: generated client differs from committed output" >&2
  echo "lint-openapi-client: regenerate with: just openapi-client" >&2
  diff -u "$OUT" "$tmp/openapi.ts" | sed 's/^/  /' >&2 || true
  exit 1
fi

echo "lint-openapi-client: committed client matches OpenAPI document fixture"

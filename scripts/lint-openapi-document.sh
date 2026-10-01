#!/usr/bin/env bash
# ADR 0011: both committed OpenAPI document fixtures name 3.0.3, and each
# file's operationId set equals the capability-table ids (count 72).
# Offline; no Postgres, no Node. POSIX bash 3.2; portable awk.
# Does not compare document bodies. Full regeneration is `just openapi-document`
# (explicit; not a dependency of ci / ci-db). A schema change that does not
# add or rename an operation id stays green.
set -eu

ROOT="${REPO_ROOT:-$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)}"
CAPS="$ROOT/crates/wicket-server/src/capabilities.rs"
PLAIN="$ROOT/crates/wicket-server/tests/fixtures/openapi-document.json"
REGULATED="$ROOT/crates/wicket-server/tests/fixtures/openapi-document-regulated.json"
EXPECT=72

if [ "${1:-}" != "" ]; then
  echo "lint-openapi-document: unknown argument: $1 (this script has no --write)" >&2
  echo "lint-openapi-document: regenerate with: just openapi-document" >&2
  exit 1
fi

if [ ! -f "$CAPS" ]; then
  echo "lint-openapi-document: missing capability table: $CAPS" >&2
  exit 1
fi

# Same kernel(/module( scan as lint-openapi-fixture.sh. First quoted string is the id.
extract_ids() {
  awk '
    /kernel\(|module\(/ {
      in_call = 1
      nstr = 0
    }
    in_call {
      s = $0
      while (match(s, /"[^"]*"/)) {
        nstr++
        strs[nstr] = substr(s, RSTART + 1, RLENGTH - 2)
        s = substr(s, RSTART + RLENGTH)
      }
    }
    in_call && /\)/ {
      if (nstr >= 3 && strs[1] != "") {
        print strs[1]
        found++
      }
      in_call = 0
      nstr = 0
    }
    END {
      if (found + 0 == 0) {
        print "lint-openapi-document: extracted 0 capability ids from capabilities.rs" > "/dev/stderr"
        exit 1
      }
    }
  ' "$CAPS"
}

extract_operation_ids() {
  awk '
    {
      s = $0
      while (match(s, /"operationId": "[^"]*"/)) {
        hit = substr(s, RSTART, RLENGTH)
        split(hit, parts, "\"")
        if (parts[4] != "") {
          print parts[4]
        }
        s = substr(s, RSTART + RLENGTH)
      }
    }
  ' "$1"
}

check_file() {
  label="$1"
  path="$2"
  if [ ! -f "$path" ]; then
    echo "lint-openapi-document: missing fixture ($label): $path" >&2
    echo "lint-openapi-document: regenerate with: just openapi-document" >&2
    exit 1
  fi
  if ! grep -F -q '"openapi": "3.0.3"' "$path"; then
    echo "lint-openapi-document: $label does not contain \"openapi\": \"3.0.3\"" >&2
    exit 1
  fi
  extract_operation_ids "$path" | LC_ALL=C sort > "$tmp/doc"
  doc_n="$(grep -c . "$tmp/doc" || true)"
  doc_u="$(LC_ALL=C sort -u "$tmp/doc" | grep -c . || true)"
  if [ "$doc_n" -ne "$doc_u" ]; then
    echo "lint-openapi-document: duplicate operationId in $label" >&2
    LC_ALL=C sort "$tmp/doc" | uniq -d | sed 's/^/  /' >&2
    exit 1
  fi
  if [ "$doc_n" -ne "$EXPECT" ]; then
    echo "lint-openapi-document: $label has $doc_n operationIds, expected $EXPECT" >&2
    exit 1
  fi
  LC_ALL=C sort -u "$tmp/doc" > "$tmp/doc_u"
  extra="$(comm -23 "$tmp/doc_u" "$tmp/table" || true)"
  missing="$(comm -13 "$tmp/doc_u" "$tmp/table" || true)"
  fail=0
  if [ -n "$extra" ]; then
    echo "lint-openapi-document: extra operationId in $label (not in capability table):" >&2
    printf '%s\n' "$extra" | sed 's/^/  /' >&2
    fail=1
  fi
  if [ -n "$missing" ]; then
    echo "lint-openapi-document: missing operationId in $label (in capability table):" >&2
    printf '%s\n' "$missing" | sed 's/^/  /' >&2
    fail=1
  fi
  if [ "$fail" -ne 0 ]; then
    echo "lint-openapi-document: regenerate with: just openapi-document" >&2
    exit 1
  fi
}

tmp="$(mktemp -d "${TMPDIR:-/tmp}/lint-openapi-document.XXXXXX")"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT
trap 'cleanup; exit 130' INT TERM

extract_ids | LC_ALL=C sort > "$tmp/table"
table_n="$(grep -c . "$tmp/table" || true)"
table_u="$(LC_ALL=C sort -u "$tmp/table" | grep -c . || true)"
if [ "$table_n" -ne "$table_u" ]; then
  echo "lint-openapi-document: duplicate capability id in capabilities.rs" >&2
  LC_ALL=C sort "$tmp/table" | uniq -d | sed 's/^/  /' >&2
  exit 1
fi
if [ "$table_n" -ne "$EXPECT" ]; then
  echo "lint-openapi-document: capability table has $table_n ids, expected $EXPECT" >&2
  exit 1
fi

check_file "plain-shop" "$PLAIN"
check_file "regulated-device" "$REGULATED"

echo "lint-openapi-document: both fixtures name OpenAPI 3.0.3 and $EXPECT operation ids match the capability table"

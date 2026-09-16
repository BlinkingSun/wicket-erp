# mount1-r1

**VERDICT:** pass

Seven operations mounted over engine functions that already existed. No
`modules/**` or `crates/wicket-identity/**` edits. Operation count **65 → 72**.

## The seven

| Operation | Method / path | Permission | Confirmed in `KNOWN_PERMISSIONS` |
|---|---|---|---|
| `listPrincipals` | `GET /api/v1/identity/principals` | `identity.manage` | yes (`session.rs`) |
| `getPrincipalByUsername` | `GET /api/v1/identity/principals/by-username?username=` | `identity.manage` | yes |
| `listRoles` | `GET /api/v1/identity/roles` | `identity.manage` | yes |
| `getRoleByName` | `GET /api/v1/identity/roles/by-name?name=` | `identity.manage` | yes |
| `listRolesForPrincipal` | `GET /api/v1/identity/principals/{id}/roles` | `identity.manage` | yes |
| `resolveItemByNumber` | `GET /api/v1/items/resolve?number=` | `items.view` | yes (`session.rs`) |
| `resolveWorkOrderByNumber` | `GET /api/v1/work-orders/resolve?number=` | `production.view` | yes (`session.rs`) |

No permission string was invented. `items.view` and `production.view` are
already in `KNOWN_PERMISSIONS` and already used by `getItem` / `getWorkOrder`.

By-number lookup is a required query parameter on its own operation, not a
path segment and not a `listItems` filter. Resolvers return `{ "id": "<uuid>" }`
— the engine functions return an id, not a full body.

Pagination on `listPrincipals` / `listRoles` follows `list_lots`: `limit`
default 50, max 200, `cursor`, `has_more`. `listRolesForPrincipal` is
unpaginated in the engine; the HTTP envelope still uses `ListBody` with
`has_more: false`.

## error.rs arms

**Found (the W8 trap):** `error.rs` mapped `Items::NotFound` to HTTP 404.
`items::resolve` returns **`Items::UnknownNumber`**, which had **no arm** and
fell through to the catch-all `_` → HTTP **500**.

**Changed:** `Items::UnknownNumber(_)` is now in the same 404 `NOT_FOUND` arm
as `Items::NotFound`.

**production_min `NotFound`:** already mapped in that same arm
(`Self::Production(wicket_mod_production_min::Error::NotFound)`). No gap.
`production_min::resolve` returns `Error::NotFound` on a miss, so the existing
arm is the correct 404.

**InvalidLimit:** `Identity::InvalidLimit` had no arm (would 500). Added
`Identity` / `Items` / `Lots` / `Production` `InvalidLimit` → HTTP **400**
`VALIDATION` on field `limit`. Handler `parse_limit` also refuses `limit=201`
before the engine.

Asserted:

- unit: `error::tests::unknown_item_number_is_http_404`
- unit: `error::tests::production_not_found_is_http_404`
- unit: `error::tests::identity_invalid_limit_is_http_400`
- unit: `error::tests::items_invalid_limit_is_http_400`
- HTTP: `resolve_item_by_number_happy_and_unknown_is_404` (unknown **and**
  lowercased number → 404, not 500)
- HTTP: `resolve_work_order_by_number_happy_and_unknown_is_404` (unknown → 404)
- HTTP: `list_principals_happy_and_invalid_limit` (`limit=201` → 400)
- HTTP: `list_roles_happy_and_invalid_limit` (`limit=0` → 400)

A happy-path-only test does not close this. The unknown-number tests assert
status 404 and code `NOT_FOUND`.

## Schemas (T-35 pattern)

`SCHEMA_CAPABILITIES` 12 → 19. Same `schema_binding` / `merge_type` /
table-walk / served-document assertions as T-35. Query parameters declared
in `query_parameters` the same way `listLots` / `getOnHand` are.

## Gate output

```
just lint-mounts
lint-mounts: 72 capability ids bound; no string-literal mounts
exit 0

just lint-openapi-fixture
lint-openapi-fixture: 72 operations match the capability table
exit 0

table_has_the_mounted_count  →  assert_eq!(KERNEL.len() + MODULE.len(), 72)  ok
```

`just ci` (env: `set -a; source .env.example; set +a; export WICKET_TEST_TEMPLATE=wicket_test_template WICKET_TEST_DB=wicket_test`):

```
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

(last crate: `wicket_uom` `--lib`) **exit 0**

`just ci-db` (same env):

```
     Running tests/mount1.rs
running 8 tests
test resolve_item_by_number_happy_and_unknown_is_404 ... ok
test resolve_work_order_by_number_happy_and_unknown_is_404 ... ok
test get_role_by_name_happy ... ok
test get_principal_by_username_happy ... ok
test list_roles_for_principal_happy ... ok
test list_roles_happy_and_invalid_limit ... ok
test list_principals_happy_and_invalid_limit ... ok
test seven_operations_are_closed_world_on_permission ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.14s

   Doc-tests wicket_uom
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**exit 0**

Closed-world: noperm (`identity.session` only) is 403 on all seven; operator
(`items.view` / `production.view`, no `identity.manage`) is 403 on the five
identity reads and may call the resolvers; identity-admin (`identity.manage`,
no item/production view) is 403 on both resolvers.

## DIVERGENCE

- **Resolver rows are `kernel()`, not `module()`.** `items/module.toml` and
  `production_min/module.toml` do not declare `/api/v1/items/resolve` or
  `/api/v1/work-orders/resolve`. This lane must not touch `modules/**`.
  `module_rows_exist_in_toml` would fail if they were `MODULE` rows. Paths
  and permissions still match the items / production-min conventions.
- Resolvers stay **case-sensitive**. A lowercased barcode is 404. Not
  "fixed".
- Docs that still say "65 rows" (`docs/13-ui-wrapper-contract.md`,
  `docs/AGENT-UI-CONTEXT.md`) were left alone; this lane does not own them.

Lane: mount1-r1

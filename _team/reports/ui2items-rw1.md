# ui2items-rw1 lane report

**VERDICT:** pass

## Findings F3–F9

| ID | Status |
|---|---|
| **F3** | **fixed** — Removed invented `listItemInventory`. Inventory tab calls mounted `getOnHand` with `item_id` and shows on-hand / available totals plus a note that per-location breakdown is not on the operation. |
| **F4** | **fixed** — Revision strip renders pipe-separated layout with current `Rev {revision}` emphasised; prior history explained inline (needs `listItemRevisions`). |
| **F5** | **fixed** — Description is read-only copy; Save stays present, disabled, with `title` explaining no editable fields yet. |
| **F6** | **fixed** — Active office nav link uses `var(--accent)` for label text and left bar (`items.css`). |
| **F7** | **fixed** — Item master uses shared `status-pill` from `global.css`; removed private filled `.item-master__status-pill`. |
| **F8** | **fixed** — Dropped `text-transform: uppercase` on shared `.status-pill` so labels render as `Released`. |
| **F9** | **fixed** — Tests cover BOM empty state, inventory `getOnHand`, and each unbacked tab in `UNBACKED_TAB_CAPABILITIES` (routing, documents, where-used). |

## getOnHand item scoping

`GET /api/v1/inventory/on-hand` requires `item_id` (OpenAPI / `OnHandQ` in `handlers/mod.rs`). Optional `location_id` and `lot_id` narrow the slice. With only `item_id`, `wicket_mod_inventory::on_hand` sums ledger balance across non-boundary locations for that item. **The operation is scoped to one item** and is wired on the Inventory tab.

## Gate output (§2.1)

```
npm run build  →  ✓ built in 114ms  →  exit 0
npm run lint   →  (eslint src, no errors)  →  exit 0
npm test       →  Test Files 3 passed (3), Tests 11 passed (11)  →  exit 0
```

(Run from `apps/wicket-web` after `npm ci`.)

## Mockup walk F4–F8 (`design/mockup-item-master.png`)

| Finding | Matches mockup? | Notes |
|---|---|---|
| **F4** | **Partial** | Strip shape, separator, and emphasised current revision align. Mockup shows three dated revision rows; only current revision is API-backed, so earlier rows are replaced by an explicit unavailability line. |
| **F5** | **Yes** | Description is static text, not an editor. |
| **F6** | **Yes** | Active Items nav label and left accent bar use amber/accent. |
| **F7** | **No (intentional)** | Mockup uses a filled orange pill; UI uses the shared outlined `status-pill` per house-style / SPEC §1 F7. |
| **F8** | **Yes** | Status displays as `Released`, not `RELEASED`. |

## DIVERGENCE

- **Status pill (F7):** Mockup filled treatment not adopted; shared outlined `status-pill` is the single app-wide style. Changing to filled would be a global `status-pill` change, not a screen-local override.
- **Revision strip (F4):** No fabricated Rev A/B rows or dates; honesty over visual parity with mockup history.

## Files touched

`apps/wicket-web/src/api/{client.ts,map-on-hand.ts,types/on-hand.ts,view-models.ts}`, `apps/wicket-web/src/features/items/*`, `apps/wicket-web/src/styles/global.css`, `apps/wicket-web/src/modes/office-layout.tsx` (unchanged; nav styles in `items.css`).

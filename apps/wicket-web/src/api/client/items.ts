// Lane c2-items: listItems, resolveItemByNumber land here.

import { getJson, patchJson } from "../http";
import { parseItemBody } from "../map-item";
import type { ItemPatchWire } from "../types/items";
import type { ItemMasterView } from "../view-models";

export async function getItem(itemId: string): Promise<ItemMasterView> {
  const payload = await getJson(`/api/v1/items/${encodeURIComponent(itemId)}`);
  return parseItemBody(payload);
}

export async function updateItem(args: {
  itemId: string;
  patch: ItemPatchWire;
  version: number;
  idempotencyKey: string;
}): Promise<ItemMasterView> {
  const payload = await patchJson(
    `/api/v1/items/${encodeURIComponent(args.itemId)}`,
    args.patch,
    {
      "If-Match": String(args.version),
      "Idempotency-Key": args.idempotencyKey,
    },
  );
  return parseItemBody(payload);
}

// Lane c2-items: listItems, resolveItemByNumber land here.

import { getJson, patchJson } from "../http";
import { parseItemBody } from "../map-item";
import {
  parseItemListBody,
  parseResolveItemBody,
  type ItemListPageView,
  type ResolvedItemIdView,
} from "../map-item-list";
import type { ItemPatchWire } from "../types/items";
import type { ItemMasterView } from "../view-models";

export type { ItemListPageView, ResolvedItemIdView };

export type ListItemsQuery = {
  limit?: number;
  cursor?: string;
  numberPrefix?: string;
  kind?: string;
  status?: string;
};

export async function listItems(
  query: ListItemsQuery = {},
): Promise<ItemListPageView> {
  const params = new URLSearchParams();
  if (query.limit !== undefined) {
    params.set("limit", String(query.limit));
  }
  if (query.cursor) {
    params.set("cursor", query.cursor);
  }
  if (query.numberPrefix) {
    params.set("number_prefix", query.numberPrefix);
  }
  if (query.kind) {
    params.set("kind", query.kind);
  }
  if (query.status) {
    params.set("status", query.status);
  }
  const qs = params.toString();
  const payload = await getJson(qs ? `/api/v1/items?${qs}` : "/api/v1/items");
  return parseItemListBody(payload);
}

export async function resolveItemByNumber(
  number: string,
): Promise<ResolvedItemIdView> {
  const params = new URLSearchParams({ number });
  const payload = await getJson(`/api/v1/items/resolve?${params.toString()}`);
  return parseResolveItemBody(payload);
}

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

import { getJson, patchJson } from "./http";
import { parseItemBody } from "./map-item";
import { mapTraceResponse } from "./map-trace";
import type { ItemPatchWire } from "./types/items";
import type {
  GenealogyResultView,
  ItemMasterView,
  TraceDirection,
} from "./view-models";

export type { GenealogyResultView, ItemMasterView, TraceDirection } from "./view-models";

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

export async function traceGenealogy(args: {
  fromLotId: string;
  direction: TraceDirection;
}): Promise<GenealogyResultView> {
  const params = new URLSearchParams({
    from_lot_id: args.fromLotId,
    direction: args.direction,
  });
  const payload = await getJson(`/api/v1/genealogy/trace?${params.toString()}`);
  return mapTraceResponse(payload);
}

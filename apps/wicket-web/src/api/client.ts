import { getJson, patchJson } from "./http";
import { parseOwnProfileBody } from "./map-identity";
import { parseItemBody } from "./map-item";
import { type OnHandView, parseOnHandBody } from "./map-on-hand";
import { mapTraceResponse } from "./map-trace";
import { parseWorkOrderBody } from "./map-work-order";
import type { ItemPatchWire } from "./types/items";
import type {
  GenealogyResultView,
  ItemMasterView,
  OwnProfileView,
  TraceQueryDirection,
  WorkOrderView,
} from "./view-models";

export type { OnHandView } from "./map-on-hand";
export type {
  GenealogyResultView,
  ItemMasterView,
  OwnProfileView,
  TraceDirection,
  TraceQueryDirection,
  WorkOrderView,
} from "./view-models";

export async function getItem(itemId: string): Promise<ItemMasterView> {
  const payload = await getJson(`/api/v1/items/${encodeURIComponent(itemId)}`);
  return parseItemBody(payload);
}

export async function getWorkOrder(workOrderId: string): Promise<WorkOrderView> {
  const payload = await getJson(
    `/api/v1/work-orders/${encodeURIComponent(workOrderId)}`,
  );
  return parseWorkOrderBody(payload);
}

export async function getOwnProfile(): Promise<OwnProfileView> {
  const payload = await getJson("/api/v1/identity/me");
  return parseOwnProfileBody(payload);
}

export async function getOnHand(args: {
  itemId: string;
  locationId?: string;
  lotId?: string;
}): Promise<OnHandView> {
  const params = new URLSearchParams({ item_id: args.itemId });
  if (args.locationId) {
    params.set("location_id", args.locationId);
  }
  if (args.lotId) {
    params.set("lot_id", args.lotId);
  }
  const payload = await getJson(`/api/v1/inventory/on-hand?${params.toString()}`);
  return parseOnHandBody(payload);
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
  direction: TraceQueryDirection;
}): Promise<GenealogyResultView> {
  const params = new URLSearchParams({
    from_lot_id: args.fromLotId,
    direction: args.direction,
  });
  const payload = await getJson(`/api/v1/genealogy/trace?${params.toString()}`);
  return mapTraceResponse(payload);
}

// On-hand inventory reads; extend here for additional on-hand shaped calls.

import { getJson } from "../http";
import { type OnHandView, parseOnHandBody } from "../map-on-hand";

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

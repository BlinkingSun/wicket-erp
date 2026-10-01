// Lane c3-floor: listWorkOrders, resolveWorkOrderByNumber land here.

import { getJson } from "../http";
import { jsonObject } from "./object";
import { parseWorkOrderBody } from "../map-work-order";
import type { components } from "../generated/openapi";
import type { WorkOrderView } from "../view-models";

type WorkOrderListWire = components["schemas"]["ListBody_for_WorkOrderJson"];
type ResolveIdWire = components["schemas"]["ResolveIdBody"];

export type WorkOrderListPage = {
  data: WorkOrderView[];
  nextCursor: string | null;
  hasMore: boolean;
};

export type ListWorkOrdersArgs = {
  limit?: number;
  cursor?: string;
  status?: string;
};

export async function getWorkOrder(workOrderId: string): Promise<WorkOrderView> {
  const payload = jsonObject(
    await getJson(`/api/v1/work-orders/${encodeURIComponent(workOrderId)}`),
    "Work order response was not an object.",
  );
  return parseWorkOrderBody(payload);
}

export async function listWorkOrders(
  args: ListWorkOrdersArgs = {},
): Promise<WorkOrderListPage> {
  const params = new URLSearchParams();
  if (args.limit != null) {
    params.set("limit", String(args.limit));
  }
  if (args.cursor) {
    params.set("cursor", args.cursor);
  }
  if (args.status) {
    params.set("status", args.status);
  }
  const query = params.toString();
  const path = query ? `/api/v1/work-orders?${query}` : "/api/v1/work-orders";
  return parseWorkOrderList(
    jsonObject(await getJson(path), "Work order list response was not an object."),
  );
}

export async function resolveWorkOrderByNumber(
  number: string,
): Promise<{ id: string }> {
  const params = new URLSearchParams({ number });
  const payload = jsonObject(
    await getJson(`/api/v1/work-orders/resolve?${params.toString()}`),
    "Work order resolve response was not an object.",
  );
  const id = (payload as ResolveIdWire).id;
  if (typeof id !== "string" || id.length === 0) {
    throw new Error("Work order resolve response missing id.");
  }
  return { id };
}

function parseWorkOrderList(payload: object): WorkOrderListPage {
  const body = payload as WorkOrderListWire;
  if (!Array.isArray(body.data)) {
    throw new Error("Work order list response missing data.");
  }
  const nextCursor =
    body.next_cursor === null ||
    body.next_cursor === undefined ||
    typeof body.next_cursor === "string"
      ? (body.next_cursor ?? null)
      : null;
  return {
    data: body.data.map(parseWorkOrderBody),
    nextCursor,
    hasMore: body.has_more === true,
  };
}

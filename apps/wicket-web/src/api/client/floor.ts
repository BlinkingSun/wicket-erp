// Lane c3-floor: listWorkOrders, resolveWorkOrderByNumber land here.

import { getJson } from "../http";
import { parseWorkOrderBody } from "../map-work-order";
import type { WorkOrderView } from "../view-models";

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
  const payload = await getJson(
    `/api/v1/work-orders/${encodeURIComponent(workOrderId)}`,
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
  return parseWorkOrderList(await getJson(path));
}

export async function resolveWorkOrderByNumber(
  number: string,
): Promise<{ id: string }> {
  const params = new URLSearchParams({ number });
  const payload = await getJson(
    `/api/v1/work-orders/resolve?${params.toString()}`,
  );
  if (payload === null || typeof payload !== "object") {
    throw new Error("Work order resolve response was not an object.");
  }
  const id = (payload as { id?: unknown }).id;
  if (typeof id !== "string" || id.length === 0) {
    throw new Error("Work order resolve response missing id.");
  }
  return { id };
}

function parseWorkOrderList(payload: unknown): WorkOrderListPage {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Work order list response was not an object.");
  }
  const body = payload as {
    data?: unknown;
    next_cursor?: unknown;
    has_more?: unknown;
  };
  if (!Array.isArray(body.data)) {
    throw new Error("Work order list response missing data.");
  }
  const nextCursor =
    body.next_cursor === null || typeof body.next_cursor === "string"
      ? body.next_cursor
      : null;
  return {
    data: body.data.map(parseWorkOrderBody),
    nextCursor,
    hasMore: body.has_more === true,
  };
}

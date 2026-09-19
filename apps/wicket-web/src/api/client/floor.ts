// Lane c3-floor: listWorkOrders, resolveWorkOrderByNumber land here.

import { getJson } from "../http";
import { parseWorkOrderBody } from "../map-work-order";
import type { WorkOrderView } from "../view-models";

export async function getWorkOrder(workOrderId: string): Promise<WorkOrderView> {
  const payload = await getJson(
    `/api/v1/work-orders/${encodeURIComponent(workOrderId)}`,
  );
  return parseWorkOrderBody(payload);
}

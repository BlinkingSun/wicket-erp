import type { QuantityBodyWire, WorkOrderBodyWire } from "./types/work-orders";
import type { WorkOrderView } from "./view-models";

function formatAmount(amount: string): string {
  return amount.replace(/(\.\d*?)0+$/, "$1").replace(/\.$/, "");
}

function titleCaseStatus(status: string): string {
  if (status.length === 0) {
    return status;
  }
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}

function isQuantityBody(value: unknown): value is QuantityBodyWire {
  if (value === null || typeof value !== "object") {
    return false;
  }
  const quantity = value as QuantityBodyWire;
  return (
    typeof quantity.amount === "string" &&
    typeof quantity.unit === "number" &&
    typeof quantity.dimension === "string"
  );
}

export function mapWorkOrderBody(body: WorkOrderBodyWire): WorkOrderView {
  return {
    id: body.id,
    number: body.number,
    itemId: body.item_id,
    quantityAmount: formatAmount(body.quantity.amount),
    quantityDimension: body.quantity.dimension,
    status: body.status,
    statusLabel: titleCaseStatus(body.status),
    revision: body.revision,
    wipLocationId: body.wip_location_id,
    version: body.version,
  };
}

export function parseWorkOrderBody(payload: unknown): WorkOrderView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Work order response was not an object.");
  }
  const body = payload as WorkOrderBodyWire;
  if (typeof body.id !== "string" || typeof body.item_id !== "string") {
    throw new Error("Work order response missing id or item_id.");
  }
  if (body.number !== null && typeof body.number !== "string") {
    throw new Error("Work order response number must be a string or null.");
  }
  if (!isQuantityBody(body.quantity)) {
    throw new Error("Work order response missing quantity amount, unit, or dimension.");
  }
  if (typeof body.status !== "string" || typeof body.revision !== "string") {
    throw new Error("Work order response missing status or revision.");
  }
  if (typeof body.version !== "number") {
    throw new Error("Work order response missing version.");
  }
  if (body.wip_location_id !== null && typeof body.wip_location_id !== "string") {
    throw new Error("Work order response wip_location_id must be a string or null.");
  }
  return mapWorkOrderBody(body);
}

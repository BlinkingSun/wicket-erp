import type { components } from "./generated/openapi";
import type { ItemMasterView } from "./view-models";

export type ItemBodyWire = components["schemas"]["ItemBody"];
export type ItemPatchWire = components["schemas"]["ItemPatch"];

const STOCK_UOM_LABELS: Record<number, string> = {
  1: "EA",
  2: "MM",
  3: "IN",
  4: "FT",
};

function titleCaseKind(kind: string): string {
  if (kind.length === 0) {
    return kind;
  }
  return kind.charAt(0).toUpperCase() + kind.slice(1).toLowerCase();
}

function titleCaseStatus(status: string): string {
  if (status.length === 0) {
    return status;
  }
  return status.charAt(0).toUpperCase() + status.slice(1).toLowerCase();
}

function stockUomLabel(stockUom: number): string {
  return STOCK_UOM_LABELS[stockUom] ?? `#${stockUom}`;
}

export function mapItemBody(body: ItemBodyWire): ItemMasterView {
  return {
    id: body.id,
    number: body.number,
    revision: body.revision,
    description: body.description,
    kindLabel: titleCaseKind(body.kind),
    stockUomLabel: stockUomLabel(body.stock_uom),
    statusLabel: titleCaseStatus(body.status),
    version: body.version,
  };
}

export function parseItemBody(payload: object): ItemMasterView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Item response was not an object.");
  }
  const body = payload as ItemBodyWire;
  if (typeof body.id !== "string" || typeof body.number !== "string") {
    throw new Error("Item response missing id or number.");
  }
  return mapItemBody(body);
}

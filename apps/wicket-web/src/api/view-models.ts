export type TraceDirection = "forward" | "backward";

export type TraceQuantityView = {
  amount: string;
  unit: number;
  dimension: string;
};

export type TraceTreeNodeView = {
  lotId: string;
  itemId: string | null;
  serial: string | null;
  locationId: string | null;
  posting: number;
  occurredAt: string | null;
  quantity: TraceQuantityView;
  edgeQuantity: TraceQuantityView;
  amount: string;
  amountCurrency: number;
  children: TraceTreeNodeView[];
};

export type GenealogyDirectionTraceView = {
  direction: TraceDirection;
  nodes: TraceTreeNodeView[];
};

export type GenealogyTraceView = {
  kind: "inline";
  direction: TraceDirection;
  nodes: TraceTreeNodeView[];
};

export type GenealogyBothTraceView = {
  kind: "both";
  backward: GenealogyDirectionTraceView;
  forward: GenealogyDirectionTraceView;
};

export type GenealogyJobView = {
  kind: "job";
  jobId: string;
  resultUrl: string;
};

export type GenealogyResultView =
  | GenealogyTraceView
  | GenealogyBothTraceView
  | GenealogyJobView;

export type ItemMasterView = {
  id: string;
  number: string;
  revision: string;
  description: string;
  kindLabel: string;
  stockUomLabel: string;
  statusLabel: string;
  version: number;
};

export type WorkOrderView = {
  id: string;
  number: string | null;
  itemId: string;
  quantityAmount: string;
  quantityDimension: string;
  status: string;
  statusLabel: string;
  revision: string;
  wipLocationId: string | null;
  version: number;
};

export type OwnProfileView = {
  id: string;
  username: string;
  displayName: string;
};

export const ITEM_MASTER_TABS = [
  "bom",
  "routing",
  "inventory",
  "documents",
  "where-used",
] as const;

export type ItemMasterTabId = (typeof ITEM_MASTER_TABS)[number];

export type UnbackedCapability = {
  operationId: string;
  label: string;
};

export const UNBACKED_TAB_CAPABILITIES: Record<
  Exclude<ItemMasterTabId, "bom" | "inventory">,
  UnbackedCapability
> = {
  routing: { operationId: "getItemRouting", label: "Routing" },
  documents: { operationId: "listItemDocuments", label: "Documents" },
  "where-used": { operationId: "listItemWhereUsed", label: "Where Used" },
};

export const UNBACKED_BOM: UnbackedCapability = {
  operationId: "getItemBom",
  label: "Bill of Material",
};

export const UNBACKED_REVISION_HISTORY: UnbackedCapability = {
  operationId: "listItemRevisions",
  label: "Revision history",
};

export const UNBACKED_FLOOR = {
  identifierLookup: {
    operationId: "resolveIdentifier",
    label: "Identifier lookup",
  },
  clockOn: { operationId: "clockOn", label: "Clock on" },
  reportQuantity: {
    operationId: "reportWorkOrderQuantity",
    label: "Report quantity",
  },
  reportScrap: { operationId: "reportScrap", label: "Report scrap" },
  certification: {
    operationId: "getOperatorCertification",
    label: "Operator certification",
  },
  gage: {
    operationId: "getGageCalibrationStatus",
    label: "Gage calibration",
  },
  drawing: { operationId: "getItemDrawing", label: "Part drawing" },
  operation: { operationId: "getWorkOrderOperation", label: "Operation" },
  workCentre: { operationId: "getWorkOrderWorkCentre", label: "Work centre" },
  remaining: {
    operationId: "getWorkOrderProgress",
    label: "Remaining quantity",
  },
} as const satisfies Record<string, UnbackedCapability>;

export type TraceDirection = "forward" | "backward";

export type TraceNodeView = {
  lotId: string;
  identifier: string;
  itemId?: string;
  quantityLabel?: string;
  serials: string[];
};

export type TraceEdgeView = {
  fromLotId: string;
  toLotId: string;
  kind: string;
  workOrder?: string;
};

export type TraceRootView = {
  lotId: string;
  identifier: string;
  itemId?: string;
};

export type GenealogyTraceView = {
  kind: "inline";
  root: TraceRootView;
  nodes: TraceNodeView[];
  edges: TraceEdgeView[];
};

export type GenealogyJobView = {
  kind: "job";
  jobId: string;
  resultUrl: string;
};

export type GenealogyResultView = GenealogyTraceView | GenealogyJobView;

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

export const UNBACKED_INVENTORY_TAB: UnbackedCapability = {
  operationId: "listItemInventory",
  label: "Inventory",
};

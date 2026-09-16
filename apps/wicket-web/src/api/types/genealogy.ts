/**
 * Throwaway adapters for the genealogy screen only.
 * Replaced by src/api/generated/ when T-35 Wave 1 lands.
 * Do not import this module from components.
 */

export type AnyQuantity = {
  amount: string;
  unit: number;
  dimension: string;
};

export type TraceRoot = {
  lot_id: string;
  identifier: string;
  item_id?: string;
};

export type TraceNode = {
  lot_id: string;
  identifier: string;
  item_id?: string;
  quantity?: AnyQuantity;
  serials?: string[];
};

export type TraceEdge = {
  from_lot_id: string;
  to_lot_id: string;
  kind: string;
  work_order?: string;
  quantity_in?: AnyQuantity;
  quantity_out?: AnyQuantity;
};

export type TraceInline = {
  root: TraceRoot;
  nodes: TraceNode[];
  edges: TraceEdge[];
};

export type TraceJob = {
  job_id: string;
  result_url: string;
};

export type TraceResponse = TraceInline | TraceJob;

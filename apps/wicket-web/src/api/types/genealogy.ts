/**
 * Throwaway adapters for the genealogy screen only.
 * Replaced by src/api/generated/ when T-35 Wave 1 lands.
 * Do not import this module from components.
 */

export type EngineTraceQuantity = {
  amount: string;
  unit: number;
  dimension: string;
};

export type EngineTraceNode = {
  lot: string;
  item: string | null;
  serial: string | null;
  location: string | null;
  posting: number;
  occurred_at: string | null;
  quantity: EngineTraceQuantity;
  edge_quantity: EngineTraceQuantity;
  amount: string;
  amount_currency: number;
  children: EngineTraceNode[];
};

export type EngineDirectionTrace = {
  direction: "forward" | "backward";
  nodes: EngineTraceNode[];
};

export type EngineBothTrace = {
  backward: EngineDirectionTrace;
  forward: EngineDirectionTrace;
};

export type TraceJob = {
  job_id: string;
  result_url: string;
};

export type TraceResponse = EngineDirectionTrace | EngineBothTrace | TraceJob;

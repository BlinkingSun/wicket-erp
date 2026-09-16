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

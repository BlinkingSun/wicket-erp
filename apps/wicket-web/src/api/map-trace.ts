import type { AnyQuantity } from "./types/genealogy";
import type {
  GenealogyResultView,
  TraceEdgeView,
  TraceNodeView,
} from "./view-models";

function formatQuantity(quantity: AnyQuantity): string {
  const trimmed = quantity.amount
    .replace(/(\.\d*?)0+$/, "$1")
    .replace(/\.$/, "");
  return `${trimmed} ${quantity.dimension}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function mapTraceResponse(data: unknown): GenealogyResultView {
  if (!isRecord(data)) {
    throw new Error("Genealogy trace response was not an object.");
  }

  if ("job_id" in data && !("root" in data)) {
    const jobId = data.job_id;
    const resultUrl = data.result_url;
    if (typeof jobId !== "string" || typeof resultUrl !== "string") {
      throw new Error("Genealogy job response was missing job_id or result_url.");
    }
    return { kind: "job", jobId, resultUrl };
  }

  const rootRaw = data.root;
  const nodesRaw = data.nodes;
  const edgesRaw = data.edges;
  if (!isRecord(rootRaw) || !Array.isArray(nodesRaw) || !Array.isArray(edgesRaw)) {
    throw new Error("Genealogy trace response was missing root, nodes, or edges.");
  }

  const rootLotId = rootRaw.lot_id;
  const rootIdentifier = rootRaw.identifier;
  if (typeof rootLotId !== "string" || typeof rootIdentifier !== "string") {
    throw new Error("Genealogy trace root was missing lot_id or identifier.");
  }

  const nodes: TraceNodeView[] = nodesRaw.map((node) => {
    if (!isRecord(node) || typeof node.lot_id !== "string" || typeof node.identifier !== "string") {
      throw new Error("Genealogy trace node was missing lot_id or identifier.");
    }
    const quantity =
      isRecord(node.quantity) &&
      typeof node.quantity.amount === "string" &&
      typeof node.quantity.dimension === "string"
        ? formatQuantity(node.quantity as AnyQuantity)
        : undefined;
    const serials = Array.isArray(node.serials)
      ? node.serials.filter((serial): serial is string => typeof serial === "string")
      : [];
    return {
      lotId: node.lot_id,
      identifier: node.identifier,
      itemId: typeof node.item_id === "string" ? node.item_id : undefined,
      quantityLabel: quantity,
      serials,
    };
  });

  const edges: TraceEdgeView[] = edgesRaw.map((edge) => {
    if (
      !isRecord(edge) ||
      typeof edge.from_lot_id !== "string" ||
      typeof edge.to_lot_id !== "string" ||
      typeof edge.kind !== "string"
    ) {
      throw new Error("Genealogy trace edge was missing from_lot_id, to_lot_id, or kind.");
    }
    return {
      fromLotId: edge.from_lot_id,
      toLotId: edge.to_lot_id,
      kind: edge.kind,
      workOrder: typeof edge.work_order === "string" ? edge.work_order : undefined,
    };
  });

  return {
    kind: "inline",
    root: {
      lotId: rootLotId,
      identifier: rootIdentifier,
      itemId: typeof rootRaw.item_id === "string" ? rootRaw.item_id : undefined,
    },
    nodes,
    edges,
  };
}

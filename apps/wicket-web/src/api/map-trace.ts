import type {
  GenealogyBothTraceView,
  GenealogyDirectionTraceView,
  GenealogyResultView,
  TraceQuantityView,
  TraceTreeNodeView,
} from "./view-models";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function mapQuantity(raw: unknown, field: string): TraceQuantityView {
  if (!isRecord(raw)) {
    throw new Error(`Genealogy trace node was missing ${field}.`);
  }
  const amount = raw.amount;
  const dimension = raw.dimension;
  const unit = raw.unit;
  if (typeof amount !== "string" || typeof dimension !== "string" || typeof unit !== "number") {
    throw new Error(`Genealogy trace ${field} was malformed.`);
  }
  return { amount, dimension, unit };
}

function nullableString(raw: unknown, field: string): string | null {
  if (raw === null) {
    return null;
  }
  if (typeof raw === "string") {
    return raw;
  }
  throw new Error(`Genealogy trace node had invalid ${field}.`);
}

function mapTreeNode(raw: unknown): TraceTreeNodeView {
  if (!isRecord(raw) || typeof raw.lot !== "string") {
    throw new Error("Genealogy trace node was missing lot.");
  }
  const amount = raw.amount;
  if (typeof amount !== "string") {
    throw new Error("Genealogy trace node was missing amount.");
  }
  const amountCurrency = raw.amount_currency;
  if (typeof amountCurrency !== "number") {
    throw new Error("Genealogy trace node was missing amount_currency.");
  }
  const posting = raw.posting;
  if (typeof posting !== "number") {
    throw new Error("Genealogy trace node was missing posting.");
  }

  const childrenRaw = raw.children;
  if (!Array.isArray(childrenRaw)) {
    throw new Error("Genealogy trace node was missing children.");
  }
  const children = childrenRaw.map(mapTreeNode);

  return {
    lotId: raw.lot,
    itemId: nullableString(raw.item, "item"),
    serial: nullableString(raw.serial, "serial"),
    locationId: nullableString(raw.location, "location"),
    posting,
    occurredAt: nullableString(raw.occurred_at, "occurred_at"),
    quantity: mapQuantity(raw.quantity, "quantity"),
    edgeQuantity: mapQuantity(raw.edge_quantity, "edge_quantity"),
    amount,
    amountCurrency,
    children,
  };
}

function mapDirectionBranch(raw: unknown): GenealogyDirectionTraceView {
  if (!isRecord(raw)) {
    throw new Error("Genealogy trace branch was not an object.");
  }
  const direction = raw.direction;
  if (direction !== "forward" && direction !== "backward") {
    throw new Error("Genealogy trace branch was missing direction.");
  }
  const nodesRaw = raw.nodes;
  if (!Array.isArray(nodesRaw)) {
    throw new Error("Genealogy trace branch was missing nodes.");
  }
  return {
    direction,
    nodes: nodesRaw.map(mapTreeNode),
  };
}

export function mapTraceResponse(data: unknown): GenealogyResultView {
  if (!isRecord(data)) {
    throw new Error("Genealogy trace response was not an object.");
  }

  if ("job_id" in data) {
    const jobId = data.job_id;
    const resultUrl = data.result_url;
    if (typeof jobId !== "string" || typeof resultUrl !== "string") {
      throw new Error("Genealogy job response was missing job_id or result_url.");
    }
    return { kind: "job", jobId, resultUrl };
  }

  if ("backward" in data && "forward" in data) {
    const both: GenealogyBothTraceView = {
      kind: "both",
      backward: mapDirectionBranch(data.backward),
      forward: mapDirectionBranch(data.forward),
    };
    return both;
  }

  if ("direction" in data && "nodes" in data) {
    const branch = mapDirectionBranch(data);
    return {
      kind: "inline",
      direction: branch.direction,
      nodes: branch.nodes,
    };
  }

  throw new Error(
    "Genealogy trace response was not a job, direction trace, or both-direction trace.",
  );
}

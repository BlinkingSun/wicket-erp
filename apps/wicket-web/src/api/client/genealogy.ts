// Lane c-gen: getGenealogyJob, listLots land here.

import { getJson } from "../http";
import { mapTraceResponse } from "../map-trace";
import type { GenealogyResultView, TraceQueryDirection } from "../view-models";

export async function traceGenealogy(args: {
  fromLotId: string;
  direction: TraceQueryDirection;
}): Promise<GenealogyResultView> {
  const params = new URLSearchParams({
    from_lot_id: args.fromLotId,
    direction: args.direction,
  });
  const payload = await getJson(`/api/v1/genealogy/trace?${params.toString()}`);
  return mapTraceResponse(payload);
}

const JOB_STATES = [
  "Queued",
  "Running",
  "Succeeded",
  "Failed",
  "Cancelled",
] as const;

export type GenealogyJobState = (typeof JOB_STATES)[number];

export type GenealogyJobStatusView = {
  id: string;
  kind: string;
  state: GenealogyJobState;
  progressPct: number;
  progressNote: string | null;
  result: GenealogyResultView | null;
  lastError: string | null;
};

export type LotListItemView = {
  id: string;
  identifier: string;
  itemId: string;
  status: string;
  supplierLot: string | null;
};

export type LotListPageView = {
  lots: LotListItemView[];
  nextCursor: string | null;
  hasMore: boolean;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isJobState(value: unknown): value is GenealogyJobState {
  return (
    typeof value === "string" && (JOB_STATES as readonly string[]).includes(value)
  );
}

function nullableString(raw: unknown, field: string): string | null {
  if (raw === null || raw === undefined) {
    return null;
  }
  if (typeof raw === "string") {
    return raw;
  }
  throw new Error(`Genealogy job had invalid ${field}.`);
}

export function mapGenealogyJobStatus(data: unknown): GenealogyJobStatusView {
  if (!isRecord(data)) {
    throw new Error("Genealogy job response was not an object.");
  }
  if (typeof data.id !== "string") {
    throw new Error("Genealogy job response was missing id.");
  }
  if (typeof data.kind !== "string") {
    throw new Error("Genealogy job response was missing kind.");
  }
  if (!isJobState(data.state)) {
    throw new Error("Genealogy job response was missing state.");
  }
  if (typeof data.progress_pct !== "number") {
    throw new Error("Genealogy job response was missing progress_pct.");
  }
  let result: GenealogyResultView | null = null;
  if (data.result !== null && data.result !== undefined) {
    result = mapTraceResponse(data.result);
  }
  return {
    id: data.id,
    kind: data.kind,
    state: data.state,
    progressPct: data.progress_pct,
    progressNote: nullableString(data.progress_note, "progress_note"),
    result,
    lastError: nullableString(data.last_error, "last_error"),
  };
}

export async function getGenealogyJob(
  resultUrl: string,
): Promise<GenealogyJobStatusView> {
  const payload = await getJson(resultUrl);
  return mapGenealogyJobStatus(payload);
}

function mapLotListItem(raw: unknown): LotListItemView {
  if (!isRecord(raw)) {
    throw new Error("Lot list item was not an object.");
  }
  if (typeof raw.id !== "string") {
    throw new Error("Lot list item was missing id.");
  }
  if (typeof raw.identifier !== "string") {
    throw new Error("Lot list item was missing identifier.");
  }
  if (typeof raw.item_id !== "string") {
    throw new Error("Lot list item was missing item_id.");
  }
  if (typeof raw.status !== "string") {
    throw new Error("Lot list item was missing status.");
  }
  return {
    id: raw.id,
    identifier: raw.identifier,
    itemId: raw.item_id,
    status: raw.status,
    supplierLot: nullableString(raw.supplier_lot, "supplier_lot"),
  };
}

export function mapLotListPage(data: unknown): LotListPageView {
  if (!isRecord(data)) {
    throw new Error("Lot list response was not an object.");
  }
  if (!Array.isArray(data.data)) {
    throw new Error("Lot list response was missing data.");
  }
  if (typeof data.has_more !== "boolean") {
    throw new Error("Lot list response was missing has_more.");
  }
  return {
    lots: data.data.map(mapLotListItem),
    nextCursor: nullableString(data.next_cursor, "next_cursor"),
    hasMore: data.has_more,
  };
}

export async function listLots(args: {
  limit: number;
  cursor?: string;
}): Promise<LotListPageView> {
  const params = new URLSearchParams({ limit: String(args.limit) });
  if (args.cursor) {
    params.set("cursor", args.cursor);
  }
  const payload = await getJson(`/api/v1/lots?${params.toString()}`);
  return mapLotListPage(payload);
}

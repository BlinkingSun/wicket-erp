// Lane c-gen: getGenealogyJob, listLots land here.

import { getJson } from "../http";
import { jsonObject } from "./object";
import { mapTraceResponse } from "../map-trace";
import type { components } from "../generated/openapi";
import type { GenealogyResultView, TraceQueryDirection } from "../view-models";

type JobStatusWire = components["schemas"]["JobStatus"];
type LotListWire = components["schemas"]["ListBody_for_LotBody"];
type LotBodyWire = components["schemas"]["LotBody"];

export async function traceGenealogy(args: {
  fromLotId: string;
  direction: TraceQueryDirection;
}): Promise<GenealogyResultView> {
  const params = new URLSearchParams({
    from_lot_id: args.fromLotId,
    direction: args.direction,
  });
  const payload = jsonObject(
    await getJson(`/api/v1/genealogy/trace?${params.toString()}`),
    "Genealogy trace response was not an object.",
  );
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

function isRecord(value: object | null): value is JobStatusWire | LotListWire | LotBodyWire {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isJobState(value: string): value is GenealogyJobState {
  return (JOB_STATES as readonly string[]).includes(value);
}

function nullableString(raw: string | null | undefined, field: string): string | null {
  if (raw === null || raw === undefined) {
    return null;
  }
  if (typeof raw === "string") {
    return raw;
  }
  throw new Error(`Genealogy job had invalid ${field}.`);
}

export function mapGenealogyJobStatus(data: object): GenealogyJobStatusView {
  if (!isRecord(data)) {
    throw new Error("Genealogy job response was not an object.");
  }
  const body = data as JobStatusWire;
  if (typeof body.id !== "string") {
    throw new Error("Genealogy job response was missing id.");
  }
  if (typeof body.kind !== "string") {
    throw new Error("Genealogy job response was missing kind.");
  }
  if (typeof body.state !== "string" || !isJobState(body.state)) {
    throw new Error("Genealogy job response was missing state.");
  }
  if (typeof body.progress_pct !== "number") {
    throw new Error("Genealogy job response was missing progress_pct.");
  }
  let result: GenealogyResultView | null = null;
  if (body.result !== null && body.result !== undefined) {
    if (typeof body.result !== "object") {
      throw new Error("Genealogy job result was not an object.");
    }
    result = mapTraceResponse(body.result);
  }
  return {
    id: body.id,
    kind: body.kind,
    state: body.state,
    progressPct: body.progress_pct,
    progressNote: nullableString(body.progress_note, "progress_note"),
    result,
    lastError: nullableString(body.last_error, "last_error"),
  };
}

export async function getGenealogyJob(
  resultUrl: string,
): Promise<GenealogyJobStatusView> {
  const payload = jsonObject(await getJson(resultUrl), "Genealogy job response was not an object.");
  return mapGenealogyJobStatus(payload);
}

function mapLotListItem(raw: object): LotListItemView {
  const body = raw as LotBodyWire;
  if (typeof body.id !== "string") {
    throw new Error("Lot list item was missing id.");
  }
  if (typeof body.identifier !== "string") {
    throw new Error("Lot list item was missing identifier.");
  }
  if (typeof body.item_id !== "string") {
    throw new Error("Lot list item was missing item_id.");
  }
  if (typeof body.status !== "string") {
    throw new Error("Lot list item was missing status.");
  }
  return {
    id: body.id,
    identifier: body.identifier,
    itemId: body.item_id,
    status: body.status,
    supplierLot: nullableString(body.supplier_lot, "supplier_lot"),
  };
}

export function mapLotListPage(data: object): LotListPageView {
  const body = data as LotListWire;
  if (!Array.isArray(body.data)) {
    throw new Error("Lot list response was missing data.");
  }
  if (typeof body.has_more !== "boolean") {
    throw new Error("Lot list response was missing has_more.");
  }
  return {
    lots: body.data.map((entry) => {
      if (entry === null || typeof entry !== "object") {
        throw new Error("Lot list item was not an object.");
      }
      return mapLotListItem(entry);
    }),
    nextCursor: nullableString(body.next_cursor, "next_cursor"),
    hasMore: body.has_more,
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
  const payload = jsonObject(
    await getJson(`/api/v1/lots?${params.toString()}`),
    "Lot list response was not an object.",
  );
  return mapLotListPage(payload);
}

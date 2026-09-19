import { parseItemBody } from "./map-item";

export type ItemListRowView = {
  id: string;
  number: string;
  description: string;
  kindLabel: string;
  statusLabel: string;
  revision: string;
};

export type ItemListPageView = {
  rows: ItemListRowView[];
  nextCursor: string | null;
  hasMore: boolean;
};

export type ResolvedItemIdView = {
  id: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object";
}

export function parseItemListBody(payload: unknown): ItemListPageView {
  if (!isRecord(payload) || !Array.isArray(payload.data)) {
    throw new Error("Item list response was not a list envelope.");
  }
  const rows: ItemListRowView[] = [];
  for (const entry of payload.data) {
    const item = parseItemBody(entry);
    rows.push({
      id: item.id,
      number: item.number,
      description: item.description,
      kindLabel: item.kindLabel,
      statusLabel: item.statusLabel,
      revision: item.revision,
    });
  }
  const nextCursor =
    typeof payload.next_cursor === "string" && payload.next_cursor.length > 0
      ? payload.next_cursor
      : null;
  return {
    rows,
    nextCursor,
    hasMore: payload.has_more === true,
  };
}

export function parseResolveItemBody(payload: unknown): ResolvedItemIdView {
  if (!isRecord(payload) || typeof payload.id !== "string" || payload.id.length === 0) {
    throw new Error("Item resolve response missing id.");
  }
  return { id: payload.id };
}

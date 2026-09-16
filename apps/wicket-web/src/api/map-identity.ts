import type { OwnProfileBodyWire } from "./types/identity";
import type { OwnProfileView } from "./view-models";

export function mapOwnProfileBody(body: OwnProfileBodyWire): OwnProfileView {
  return {
    id: body.id,
    username: body.username,
    displayName: body.display_name,
  };
}

export function parseOwnProfileBody(payload: unknown): OwnProfileView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Profile response was not an object.");
  }
  const body = payload as OwnProfileBodyWire;
  if (
    typeof body.id !== "string" ||
    typeof body.username !== "string" ||
    typeof body.display_name !== "string"
  ) {
    throw new Error("Profile response missing id, username, or display_name.");
  }
  return mapOwnProfileBody(body);
}

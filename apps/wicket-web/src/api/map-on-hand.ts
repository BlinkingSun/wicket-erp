import type { components } from "./generated/openapi";

export type OnHandBodyWire = components["schemas"]["OnHandBody"];

export type OnHandView = {
  onHand: string;
  available: string;
};

export function mapOnHandBody(body: OnHandBodyWire): OnHandView {
  return {
    onHand: body.on_hand,
    available: body.available,
  };
}

export function parseOnHandBody(payload: object): OnHandView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("On-hand response was not an object.");
  }
  const body = payload as OnHandBodyWire;
  if (typeof body.on_hand !== "string" || typeof body.available !== "string") {
    throw new Error("On-hand response missing on_hand or available.");
  }
  return mapOnHandBody(body);
}

import type { components } from "./generated/openapi";
import type { OwnProfileView } from "./view-models";

export type OwnProfileBodyWire = components["schemas"]["PrincipalBody"];
export type LoginBodyWire = components["schemas"]["LoginResponse"];
export type NavigationBodyWire = components["schemas"]["NavigationBody"];

export type LoginView = {
  sessionId: string;
  principalId: string;
  displayName: string;
  csrf: string;
};

export type NavigationView = {
  visible: string[];
  hidden: string[];
};

export function mapOwnProfileBody(body: OwnProfileBodyWire): OwnProfileView {
  return {
    id: body.id,
    username: body.username,
    displayName: body.display_name,
  };
}

export function parseOwnProfileBody(payload: object): OwnProfileView {
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

export function mapLoginBody(body: LoginBodyWire): LoginView {
  return {
    sessionId: body.session_id,
    principalId: body.principal_id,
    displayName: body.display_name,
    csrf: body.csrf,
  };
}

export function parseLoginBody(payload: object): LoginView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Login response was not an object.");
  }
  const body = payload as LoginBodyWire;
  if (
    typeof body.session_id !== "string" ||
    typeof body.principal_id !== "string" ||
    typeof body.display_name !== "string" ||
    typeof body.csrf !== "string"
  ) {
    throw new Error(
      "Login response missing session_id, principal_id, display_name, or csrf.",
    );
  }
  return mapLoginBody(body);
}

export function mapNavigationBody(body: NavigationBodyWire): NavigationView {
  return {
    visible: body.visible,
    hidden: body.hidden,
  };
}

export function parseNavigationBody(payload: object): NavigationView {
  if (payload === null || typeof payload !== "object") {
    throw new Error("Navigation response was not an object.");
  }
  const body = payload as NavigationBodyWire;
  if (!Array.isArray(body.visible) || !Array.isArray(body.hidden)) {
    throw new Error("Navigation response missing visible or hidden.");
  }
  if (
    body.visible.some((entry) => typeof entry !== "string") ||
    body.hidden.some((entry) => typeof entry !== "string")
  ) {
    throw new Error("Navigation visible and hidden must be string lists.");
  }
  return mapNavigationBody(body);
}

// Lane c1-auth: login, logout, getNavigation land here.

import { getJson, postJson } from "../http";
import { jsonObject } from "./object";
import {
  parseLoginBody,
  parseNavigationBody,
  parseOwnProfileBody,
} from "../map-identity";
import type { LoginView, NavigationView } from "../map-identity";
import type { OwnProfileView } from "../view-models";

export type { LoginView, NavigationView };

export async function getOwnProfile(): Promise<OwnProfileView> {
  const payload = jsonObject(
    await getJson("/api/v1/identity/me"),
    "Profile response was not an object.",
  );
  return parseOwnProfileBody(payload);
}

export async function login(args: {
  username: string;
  password: string;
}): Promise<LoginView> {
  const payload = jsonObject(
    await postJson(
      "/api/v1/identity/login",
      { username: args.username, password: args.password },
      { skipUnauthorized: true },
    ),
    "Login response was not an object.",
  );
  return parseLoginBody(payload);
}

export async function logout(): Promise<void> {
  await postJson("/api/v1/identity/logout", {}, { skipUnauthorized: true });
}

export async function getNavigation(): Promise<NavigationView> {
  const payload = jsonObject(
    await getJson("/api/v1/navigation"),
    "Navigation response was not an object.",
  );
  return parseNavigationBody(payload);
}

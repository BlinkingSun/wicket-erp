// Lane c1-auth: login, logout, getNavigation land here.

import { getJson, postJson } from "../http";
import {
  parseLoginBody,
  parseNavigationBody,
  parseOwnProfileBody,
} from "../map-identity";
import type { LoginView, NavigationView } from "../map-identity";
import type { OwnProfileView } from "../view-models";

export type { LoginView, NavigationView };

export async function getOwnProfile(): Promise<OwnProfileView> {
  const payload = await getJson("/api/v1/identity/me");
  return parseOwnProfileBody(payload);
}

export async function login(args: {
  username: string;
  password: string;
}): Promise<LoginView> {
  const payload = await postJson(
    "/api/v1/identity/login",
    { username: args.username, password: args.password },
    { skipUnauthorized: true },
  );
  return parseLoginBody(payload);
}

export async function logout(): Promise<void> {
  await postJson("/api/v1/identity/logout", {}, { skipUnauthorized: true });
}

export async function getNavigation(): Promise<NavigationView> {
  const payload = await getJson("/api/v1/navigation");
  return parseNavigationBody(payload);
}

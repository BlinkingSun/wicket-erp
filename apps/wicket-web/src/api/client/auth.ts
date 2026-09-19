// Lane c1-auth: login, logout, getNavigation land here.

import { getJson } from "../http";
import { parseOwnProfileBody } from "../map-identity";
import type { OwnProfileView } from "../view-models";

export async function getOwnProfile(): Promise<OwnProfileView> {
  const payload = await getJson("/api/v1/identity/me");
  return parseOwnProfileBody(payload);
}

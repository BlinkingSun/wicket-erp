import { logout } from "../../api/client";
import { expireUnauthorizedSession } from "./session";

export async function signOut(): Promise<void> {
  try {
    await logout();
  } catch {
    // The server row may already be gone. Drop local state either way.
  }
  expireUnauthorizedSession();
}

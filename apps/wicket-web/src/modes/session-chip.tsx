import { useSyncExternalStore } from "react";
import { getSession, subscribeSession } from "../features/auth/session";
import { signOut } from "../features/auth/sign-out";

type SessionChipProps = {
  size?: "rail" | "floor";
};

export function SessionChip({ size = "rail" }: SessionChipProps) {
  const session = useSyncExternalStore(
    subscribeSession,
    getSession,
    getSession,
  );
  const signedIn = session.csrf !== null;
  if (!signedIn) {
    return null;
  }
  const name =
    session.displayName !== null && session.displayName.trim().length > 0
      ? session.displayName
      : "Signed in";

  return (
    <div
      className={
        size === "floor" ? "session-chip session-chip--floor" : "session-chip"
      }
    >
      <p className="session-chip__name">{name}</p>
      <button
        className="session-chip__logout"
        type="button"
        onClick={() => {
          void signOut();
        }}
      >
        Sign out
      </button>
    </div>
  );
}

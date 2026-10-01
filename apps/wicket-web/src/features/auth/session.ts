import type { QueryClient } from "@tanstack/react-query";

export type SessionSnapshot = {
  csrf: string | null;
  displayName: string | null;
};

const CSRF_COOKIE = "wicket_csrf";
const SESSION_COOKIE = "wicket_session";

type Listener = () => void;

const listeners = new Set<Listener>();

let snapshot: SessionSnapshot = {
  csrf: null,
  displayName: null,
};

let queryClient: QueryClient | null = null;
let onUnauthorizedNavigate: (() => void) | null = null;

function emit(): void {
  for (const listener of listeners) {
    listener();
  }
}

function readCookie(name: string): string | null {
  if (typeof document === "undefined") {
    return null;
  }
  const parts = document.cookie.split(";");
  for (const part of parts) {
    const trimmed = part.trim();
    const eq = trimmed.indexOf("=");
    if (eq === -1) {
      continue;
    }
    if (trimmed.slice(0, eq) === name) {
      const value = trimmed.slice(eq + 1);
      return value.length > 0 ? value : null;
    }
  }
  return null;
}

function expireCookie(name: string): void {
  if (typeof document === "undefined") {
    return;
  }
  document.cookie = `${name}=; Path=/; Max-Age=0; SameSite=Lax`;
}

export function dropAuthCookies(): void {
  expireCookie(SESSION_COOKIE);
  expireCookie(CSRF_COOKIE);
}

export function getSession(): SessionSnapshot {
  return snapshot;
}

export function subscribeSession(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getCsrfToken(): string | null {
  return snapshot.csrf ?? readCookie(CSRF_COOKIE);
}

export function hasSession(): boolean {
  return getCsrfToken() !== null;
}

export function setSession(next: {
  csrf: string;
  displayName: string;
}): void {
  snapshot = { csrf: next.csrf, displayName: next.displayName };
  emit();
}

export function clearSession(): void {
  snapshot = { csrf: null, displayName: null };
  emit();
}

export function bindQueryClient(client: QueryClient | null): void {
  queryClient = client;
}

export function setUnauthorizedNavigate(handler: (() => void) | null): void {
  onUnauthorizedNavigate = handler;
}

export function expireUnauthorizedSession(): void {
  clearSession();
  dropAuthCookies();
  queryClient?.clear();
  onUnauthorizedNavigate?.();
}

export function hydrateSessionFromCookie(): void {
  if (snapshot.csrf !== null) {
    return;
  }
  const csrf = readCookie(CSRF_COOKIE);
  if (csrf === null) {
    return;
  }
  snapshot = { csrf, displayName: snapshot.displayName };
  emit();
}

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createMemoryHistory,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { getJson, patchJson, postJson, ApiError } from "../../api/http";
import { routeTree } from "../../router";
import { signOut } from "./sign-out";
import {
  bindQueryClient,
  clearSession,
  dropAuthCookies,
  getCsrfToken,
  getSession,
  setSession,
  setUnauthorizedNavigate,
} from "./session";

const fetchMock = vi.fn();

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function requestHeaders(call = 0): Record<string, string> {
  const init = fetchMock.mock.calls[call]?.[1] as RequestInit | undefined;
  return (init?.headers ?? {}) as Record<string, string>;
}

function renderApp(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const history = createMemoryHistory({ initialEntries: [path] });
  const testRouter = createRouter({ routeTree, history });
  bindQueryClient(queryClient);
  return {
    queryClient,
    history,
    testRouter,
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={testRouter} />
      </QueryClientProvider>,
    ),
  };
}

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
  clearSession();
  dropAuthCookies();
  bindQueryClient(null);
  setUnauthorizedNavigate(null);
});

afterEach(() => {
  cleanup();
  clearSession();
  dropAuthCookies();
  bindQueryClient(null);
  setUnauthorizedNavigate(null);
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe("login screen", () => {
  it("stores the session and redirects to the intended route", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(
      jsonResponse(200, {
        session_id: "01932c5a-8b10-7001-8000-0000000000aa",
        principal_id: "01932c5a-8b10-7001-8000-0000000000bb",
        display_name: "Demo",
        csrf: "csrf-login",
      }),
    );
    fetchMock.mockResolvedValue(
      jsonResponse(200, { data: [], next_cursor: null, has_more: false }),
    );

    renderApp("/login?next=/office/items");

    await user.type(await screen.findByLabelText("Username"), "demo");
    await user.type(screen.getByLabelText("Password"), "demo-login");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    await waitFor(() => {
      expect(
        screen.getByRole("heading", { name: "Items" }),
      ).toBeInTheDocument();
    });
    expect(getSession()).toEqual({
      csrf: "csrf-login",
      displayName: "Demo",
    });
    expect(String(fetchMock.mock.calls[0]?.[0])).toContain(
      "/api/v1/identity/login",
    );
    expect(requestHeaders()["X-CSRF-Token"]).toBeUndefined();
    expect(requestHeaders()["Idempotency-Key"]).toEqual(
      expect.stringMatching(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
      ),
    );
  });

  it("renders the engine message and stays on /login when credentials are refused", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(
      jsonResponse(401, {
        error: {
          code: "UNAUTHENTICATED",
          message: "invalid credentials",
          field: null,
          request_id: "01932c5a-8b10-7001-8000-0000000000cc",
        },
      }),
    );

    renderApp("/login?next=/quality/genealogy");

    await user.type(await screen.findByLabelText("Username"), "demo");
    await user.type(screen.getByLabelText("Password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Sign in" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "invalid credentials",
    );
    expect(screen.getByRole("heading", { name: "Sign in" })).toBeInTheDocument();
    expect(getSession()).toEqual({ csrf: null, displayName: null });
    expect(screen.queryByText(/Lot-and-Serial Genealogy/)).not.toBeInTheDocument();
  });
});

describe("session expiry and logout", () => {
  it("redirects to /login when a mid-session call returns 401", async () => {
    setSession({ csrf: "csrf-live", displayName: "Demo" });
    fetchMock.mockResolvedValue(
      jsonResponse(401, {
        error: {
          code: "UNAUTHENTICATED",
          message: "session missing or expired",
          field: null,
          request_id: "01932c5a-8b10-7001-8000-0000000000dd",
        },
      }),
    );

    renderApp("/office/items/01932c5a-8b10-7001-8000-000000000001");

    expect(
      await screen.findByRole("heading", { name: "Sign in" }),
    ).toBeInTheDocument();
    expect(getSession()).toEqual({ csrf: null, displayName: null });
  });

  it("clears the store, cookies, and query cache on logout", async () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["items", "getItem"], { id: "kept" });
    bindQueryClient(queryClient);
    setSession({ csrf: "csrf-out", displayName: "Demo" });
    document.cookie = "wicket_csrf=csrf-out; Path=/";
    document.cookie = "wicket_session=session-out; Path=/";
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));

    await signOut();

    expect(getSession()).toEqual({ csrf: null, displayName: null });
    expect(document.cookie).not.toContain("wicket_csrf");
    expect(document.cookie).not.toContain("wicket_session");
    expect(queryClient.getQueryData(["items", "getItem"])).toBeUndefined();
    expect(String(fetchMock.mock.calls[0]?.[0])).toContain(
      "/api/v1/identity/logout",
    );
  });
});

describe("http helpers", () => {
  it("sends X-CSRF-Token on a mutating call and not on a GET", async () => {
    setSession({ csrf: "csrf-header", displayName: "Demo" });
    fetchMock.mockResolvedValue(jsonResponse(200, { id: "ok" }));

    await getJson("/api/v1/identity/me");
    expect(requestHeaders(0)["X-CSRF-Token"]).toBeUndefined();
    expect(requestHeaders(0)["Idempotency-Key"]).toBeUndefined();

    fetchMock.mockClear();
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    await postJson("/api/v1/identity/logout", {});
    expect(requestHeaders(0)["X-CSRF-Token"]).toBe("csrf-header");
    expect(requestHeaders(0)["Idempotency-Key"]).toEqual(
      expect.stringMatching(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
      ),
    );

    fetchMock.mockClear();
    fetchMock.mockResolvedValue(jsonResponse(200, { id: "ok" }));
    await patchJson("/api/v1/items/1", { description: "x" }, { "If-Match": "1" });
    expect(requestHeaders(0)["X-CSRF-Token"]).toBe("csrf-header");
  });

  it("keeps the engine error code on ApiError", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(409, {
        error: {
          code: "REFUSED",
          message: "lot is held",
          field: "lot_id",
          request_id: "01932c5a-8b10-7001-8000-0000000000ee",
        },
      }),
    );

    const error = await getJson("/api/v1/items").catch((err: unknown) => err);
    expect(error).toBeInstanceOf(ApiError);
    const apiError = error as ApiError;
    expect(apiError.status).toBe(409);
    expect(apiError.code).toBe("REFUSED");
    expect(apiError.message).toBe("lot is held");
    expect(apiError.field).toBe("lot_id");
  });

  it("reads csrf from the store for mutating headers", () => {
    setSession({ csrf: "csrf-store", displayName: "Demo" });
    expect(getCsrfToken()).toBe("csrf-store");
    clearSession();
    expect(getCsrfToken()).toBeNull();
  });
});

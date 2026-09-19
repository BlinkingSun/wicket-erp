import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createMemoryHistory, createRouter } from "@tanstack/react-router";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { routeTree } from "../router";
import {
  clearSession,
  dropAuthCookies,
  getSession,
  setSession,
} from "../features/auth/session";

afterEach(() => {
  cleanup();
  dropAuthCookies();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

function renderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  return {
    router,
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
  };
}

describe("mode route trees", () => {
  it("reaches the office shell with module navigation", async () => {
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    renderAt("/office");
    expect(await screen.findByRole("navigation", { name: "Office modules" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Items" })).toBeInTheDocument();
    expect(screen.getByText(/Open an item from the API/)).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Quality modules" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Mode" })).not.toBeInTheDocument();
  });

  it("reaches the shop-floor terminal", async () => {
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    renderAt("/floor");
    expect(await screen.findByLabelText("Scan traveler or badge")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "TAP OR SCAN" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "NO WORK ORDER" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Interaction modes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Mode" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Office modules" })).not.toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Quality modules" })).not.toBeInTheDocument();
  });

  it("reaches the quality genealogy screen", async () => {
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    renderAt("/quality/genealogy");
    expect(await screen.findByText("Lot-and-Serial Genealogy Trace | Recall")).toBeInTheDocument();
    expect(await screen.findByRole("search")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Quality modules" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Office modules" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Mode" })).not.toBeInTheDocument();
  });
});

describe("mode chrome", () => {
  it("renders the signed-in name from the session store", async () => {
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    renderAt("/office");
    expect(await screen.findByText("Demo Operator")).toBeInTheDocument();
  });

  it("shows a truthful label when the display name is unknown", async () => {
    clearSession();
    dropAuthCookies();
    document.cookie = "wicket_csrf=csrf-refresh; Path=/";
    renderAt("/office");
    expect(await screen.findByText("Signed in")).toBeInTheDocument();
    expect(screen.queryByText("Demo Operator")).not.toBeInTheDocument();
    expect(screen.queryByText("Test Operator")).not.toBeInTheDocument();
  });

  it("navigates from the floor mode switch to office", async () => {
    const user = userEvent.setup();
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    renderAt("/floor");
    const toggle = await screen.findByRole("button", { name: "Mode" });
    expect(toggle).toHaveStyle({ minWidth: "64px", minHeight: "64px" });
    await user.click(toggle);
    await user.click(screen.getByRole("link", { name: "Office" }));
    expect(
      await screen.findByRole("navigation", { name: "Office modules" }),
    ).toBeInTheDocument();
  });

  it("clears the session and lands on login from chrome logout", async () => {
    const user = userEvent.setup();
    setSession({ csrf: "csrf-modes", displayName: "Demo Operator" });
    const fetchMock = vi.fn().mockResolvedValue(new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);
    renderAt("/office");
    expect(await screen.findByText("Demo Operator")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Sign out" }));
    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeInTheDocument();
    await waitFor(() => {
      expect(getSession()).toEqual({ csrf: null, displayName: null });
    });
  });

  it("sends an unsigned visitor to login instead of the office shell", async () => {
    clearSession();
    dropAuthCookies();
    renderAt("/office");
    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Office modules" })).not.toBeInTheDocument();
  });
});

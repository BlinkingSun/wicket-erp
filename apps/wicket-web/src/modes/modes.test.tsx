import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createMemoryHistory, createRouter } from "@tanstack/react-router";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { routeTree } from "../router";

afterEach(() => {
  cleanup();
});

function renderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}

describe("mode route trees", () => {
  it("reaches the office shell with module navigation", async () => {
    renderAt("/office");
    expect(await screen.findByRole("navigation", { name: "Office modules" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Items" })).toBeInTheDocument();
    expect(screen.getByText(/Open an item from the API/)).toBeInTheDocument();
  });

  it("reaches the shop-floor terminal", async () => {
    renderAt("/floor");
    expect(await screen.findByLabelText("Scan traveler or badge")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "TAP OR SCAN" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "NO WORK ORDER" })).toBeInTheDocument();
  });

  it("reaches the quality genealogy screen", async () => {
    renderAt("/quality/genealogy");
    expect(await screen.findByText("Lot-and-Serial Genealogy Trace | Recall")).toBeInTheDocument();
    expect(await screen.findByRole("search")).toBeInTheDocument();
  });
});

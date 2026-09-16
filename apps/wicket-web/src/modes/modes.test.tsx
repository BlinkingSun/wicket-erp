import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createMemoryHistory, createRouter } from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { routeTree } from "../router";

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
  it("reaches the office empty shell", async () => {
    renderAt("/office");
    expect(await screen.findByText(/Office mode has no screens in this wave/)).toBeInTheDocument();
  });

  it("reaches the shop-floor empty shell", async () => {
    renderAt("/floor");
    expect(await screen.findByText(/Shop floor mode has no screens in this wave/)).toBeInTheDocument();
  });

  it("reaches the quality genealogy screen", async () => {
    renderAt("/quality/genealogy");
    expect(await screen.findByText("Lot-and-Serial Genealogy Trace | Recall")).toBeInTheDocument();
    expect(await screen.findByRole("search")).toBeInTheDocument();
  });
});

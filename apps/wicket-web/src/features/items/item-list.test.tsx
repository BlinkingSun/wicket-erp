import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ItemListPageView } from "../../api/client";

const { mockNavigate } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
}));

vi.mock("../../api/client", () => ({
  listItems: vi.fn(),
  resolveItemByNumber: vi.fn(),
}));

vi.mock("@tanstack/react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-router")>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

import { listItems } from "../../api/client";
import { ItemList } from "./ItemList";

const screw: ItemListPageView["rows"][number] = {
  id: "01932c5a-8b10-7001-8000-000000000001",
  number: "MDS-450-M4x12",
  description:
    "Cortical Bone Screw, Ti-6Al-4V ELI, M4 x 12 mm, hex drive, ASTM F136.",
  kindLabel: "Make",
  statusLabel: "Released",
  revision: "C",
};

const bar: ItemListPageView["rows"][number] = {
  id: "01932c5a-8b10-7001-8000-000000000002",
  number: "RM-TI-BAR-12",
  description: "Titanium bar, stocked in mm of length.",
  kindLabel: "Buy",
  statusLabel: "Released",
  revision: "A",
};

function page(
  rows: ItemListPageView["rows"],
  extras: Partial<ItemListPageView> = {},
): ItemListPageView {
  return {
    rows,
    nextCursor: null,
    hasMore: false,
    ...extras,
  };
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function renderList() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ItemList />
    </QueryClientProvider>,
  );
}

describe("item list", () => {
  it("renders rows from a mocked listItems page", async () => {
    vi.mocked(listItems).mockResolvedValueOnce(page([screw, bar]));

    renderList();

    expect(await screen.findByText("MDS-450-M4x12")).toBeInTheDocument();
    expect(screen.getByText("MDS-450-M4x12")).toHaveClass("mono");
    expect(
      screen.getByText(
        "Cortical Bone Screw, Ti-6Al-4V ELI, M4 x 12 mm, hex drive, ASTM F136.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("Make")).toBeInTheDocument();
    expect(screen.getByText("Buy")).toBeInTheDocument();
    expect(screen.getByText("RM-TI-BAR-12")).toHaveClass("mono");
    const status = screen.getAllByText("Released");
    expect(status[0]).toHaveClass("status-pill");
    const screwRow = screen.getByText("MDS-450-M4x12").closest("tr");
    expect(screwRow).not.toBeNull();
    expect(within(screwRow as HTMLTableRowElement).getByText("C")).toHaveClass(
      "mono",
    );
    expect(listItems).toHaveBeenCalledWith({ limit: 50 });
    expect(screen.queryByRole("button", { name: "Load more" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    expect(screen.queryByText(/getItemBom/)).not.toBeInTheDocument();
  });

  it("refetches with number_prefix when a prefix is typed", async () => {
    vi.mocked(listItems).mockResolvedValue(page([screw]));
    const user = userEvent.setup();

    renderList();
    await screen.findByText("MDS-450-M4x12");

    await user.type(screen.getByLabelText("Item number prefix"), "MDS");

    await waitFor(() => {
      expect(listItems).toHaveBeenCalledWith({
        limit: 50,
        numberPrefix: "MDS",
      });
    });
  });

  it("navigates to item master when a row is clicked", async () => {
    vi.mocked(listItems).mockResolvedValueOnce(page([screw, bar]));
    const user = userEvent.setup();

    renderList();
    await screen.findByText("MDS-450-M4x12");
    await user.click(screen.getByText("MDS-450-M4x12"));

    expect(mockNavigate).toHaveBeenCalledWith({
      to: "/office/items/$itemId",
      params: { itemId: screw.id },
    });
  });

  it("shows a prefix-specific empty state when nothing matches", async () => {
    vi.mocked(listItems).mockResolvedValue(page([]));
    const user = userEvent.setup();

    renderList();
    await user.type(screen.getByLabelText("Item number prefix"), "WO-NO-SUCH");

    const status = await screen.findByRole("status");
    expect(status).toHaveTextContent("No item matched the prefix WO-NO-SUCH.");
    expect(status).not.toHaveTextContent("no items");
    expect(screen.queryByText("MDS-450-M4x12")).not.toBeInTheDocument();
  });

  it("renders the engine message when listItems fails", async () => {
    vi.mocked(listItems).mockRejectedValueOnce(
      new Error("item store unavailable"),
    );

    renderList();

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "item store unavailable",
    );
    expect(screen.queryByText("MDS-450-M4x12")).not.toBeInTheDocument();
  });

  it("loads the next page with the engine cursor, not a client slice", async () => {
    const cursor = "01932c5a-8b10-7001-8000-000000000002";
    vi.mocked(listItems)
      .mockResolvedValueOnce(
        page([screw], { nextCursor: cursor, hasMore: true }),
      )
      .mockResolvedValueOnce(page([bar]));
    const user = userEvent.setup();

    renderList();
    await screen.findByText("MDS-450-M4x12");
    expect(screen.queryByText("RM-TI-BAR-12")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Load more" }));

    expect(await screen.findByText("RM-TI-BAR-12")).toBeInTheDocument();
    expect(screen.getByText("MDS-450-M4x12")).toBeInTheDocument();
    expect(listItems).toHaveBeenNthCalledWith(2, {
      limit: 50,
      cursor,
    });
    expect(screen.queryByRole("button", { name: "Load more" })).not.toBeInTheDocument();
  });

  it("does not invent a cursor when has_more is true and next_cursor is absent", async () => {
    vi.mocked(listItems).mockResolvedValueOnce(
      page([screw], { nextCursor: null, hasMore: true }),
    );

    renderList();
    await screen.findByText("MDS-450-M4x12");

    expect(screen.queryByRole("button", { name: "Load more" })).not.toBeInTheDocument();
    expect(listItems).toHaveBeenCalledTimes(1);
  });
});

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRouter,
} from "@tanstack/react-router";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { routeTree } from "../../router";
import { TRAVELER_SCAN_MESSAGE } from "./ScanField";
import { SCAN_LOOKUP_UNAVAILABLE } from "./unbacked";

const sampleWorkOrder: WorkOrderView = {
  id: "01932c5a-8b10-7001-8000-000000000042",
  number: "WO-0001",
  itemId: "01932c5a-8b10-7001-8000-000000000001",
  quantityAmount: "5",
  quantityDimension: "Count",
  status: "completed",
  statusLabel: "Completed",
  revision: "C",
  wipLocationId: "01932c5a-8b10-7001-8000-000000000010",
  version: 4,
};

const secondWorkOrder: WorkOrderView = {
  id: "01932c5a-8b10-7001-8000-000000000043",
  number: "WO-0002",
  itemId: "01932c5a-8b10-7001-8000-000000000002",
  quantityAmount: "25",
  quantityDimension: "Count",
  status: "in_process",
  statusLabel: "In Process",
  revision: "A",
  wipLocationId: "01932c5a-8b10-7001-8000-000000000011",
  version: 2,
};

const sampleItem: ItemMasterView = {
  id: sampleWorkOrder.itemId,
  number: "MDS-450-M4x12",
  revision: "C",
  description: "Cortical bone screw, Ti-6Al-4V ELI, M4 x 12.",
  kindLabel: "Make",
  stockUomLabel: "EA",
  statusLabel: "Released",
  version: 2,
};

const barItem: ItemMasterView = {
  id: secondWorkOrder.itemId,
  number: "RM-TI-BAR-12",
  revision: "A",
  description: "Titanium bar, stocked in mm of length.",
  kindLabel: "Buy",
  stockUomLabel: "MM",
  statusLabel: "Released",
  version: 1,
};

vi.mock("../../api/client", () => ({
  getWorkOrder: vi.fn(),
  getItem: vi.fn(),
  getOwnProfile: vi.fn(),
  listWorkOrders: vi.fn(),
}));

import {
  getItem,
  getOwnProfile,
  getWorkOrder,
  listWorkOrders,
} from "../../api/client";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function renderAt(path: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const history = createMemoryHistory({ initialEntries: [path] });
  const router = createRouter({
    routeTree,
    history,
  });
  return {
    history,
    router,
    ...render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    ),
  };
}

function mockListPage(
  data: WorkOrderView[],
  nextCursor: string | null = null,
  hasMore = false,
) {
  vi.mocked(listWorkOrders).mockResolvedValueOnce({
    data,
    nextCursor,
    hasMore,
  });
}

describe("floor work-order pick list", () => {
  it("renders from a mocked listWorkOrders", async () => {
    mockListPage([sampleWorkOrder]);
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);

    renderAt("/floor/work-orders");

    const row = await screen.findByRole("link", { name: /WO-0001/ });
    expect(within(row).getByText("Completed")).toBeInTheDocument();
    expect(await within(row).findByText("MDS-450-M4x12")).toBeInTheDocument();
    expect(within(row).getByText("5")).toHaveClass("mono");
    expect(listWorkOrders).toHaveBeenCalledWith({
      limit: 20,
      cursor: undefined,
      status: undefined,
    });
  });

  it("taps a row to the terminal and Back returns to the list", async () => {
    const user = userEvent.setup();
    mockListPage([sampleWorkOrder]);
    vi.mocked(getItem).mockResolvedValue(sampleItem);
    vi.mocked(getWorkOrder).mockResolvedValue(sampleWorkOrder);
    vi.mocked(getOwnProfile).mockResolvedValue({
      id: "01932c5a-8b10-7001-8000-000000000099",
      username: "atester",
      displayName: "A. Tester",
    });

    const { history } = renderAt("/floor/work-orders");

    await user.click(await screen.findByRole("link", { name: /WO-0001/ }));

    expect(
      await screen.findByRole("heading", { name: "WO-0001" }),
    ).toBeInTheDocument();
    expect(history.location.pathname).toBe(
      `/floor/work-orders/${sampleWorkOrder.id}`,
    );
    expect(getWorkOrder).toHaveBeenCalledWith(sampleWorkOrder.id);

    await user.click(
      screen.getByRole("link", { name: "Back to work orders" }),
    );

    expect(
      await screen.findByRole("link", { name: /WO-0001/ }),
    ).toBeInTheDocument();
    expect(history.location.pathname).toBe("/floor/work-orders");
  });

  it("navigates when a UUID is scanned and does not request a traveler", async () => {
    const user = userEvent.setup();
    mockListPage([sampleWorkOrder]);
    vi.mocked(getItem).mockResolvedValue(sampleItem);
    vi.mocked(getWorkOrder).mockResolvedValue(sampleWorkOrder);
    vi.mocked(getOwnProfile).mockResolvedValue({
      id: "01932c5a-8b10-7001-8000-000000000099",
      username: "atester",
      displayName: "A. Tester",
    });

    const { history } = renderAt("/floor/work-orders");
    await screen.findByRole("link", { name: /WO-0001/ });

    const scan = screen.getByLabelText("Scan traveler or badge");
    await user.clear(scan);
    await user.type(scan, sampleWorkOrder.id);
    await user.click(screen.getByRole("button", { name: "TAP OR SCAN" }));

    await waitFor(() => {
      expect(history.location.pathname).toBe(
        `/floor/work-orders/${sampleWorkOrder.id}`,
      );
    });
    expect(
      await screen.findByRole("heading", { name: "WO-0001" }),
    ).toBeInTheDocument();
    expect(getWorkOrder).toHaveBeenCalledWith(sampleWorkOrder.id);
  });

  it("shows the honest lookup message for a scanned work-order number", async () => {
    const user = userEvent.setup();
    mockListPage([sampleWorkOrder]);
    vi.mocked(getItem).mockResolvedValue(sampleItem);

    renderAt("/floor/work-orders");
    await screen.findByRole("link", { name: /WO-0001/ });

    const scan = screen.getByLabelText("Scan traveler or badge");
    await user.type(scan, "WO-0001");
    await user.click(screen.getByRole("button", { name: "TAP OR SCAN" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      TRAVELER_SCAN_MESSAGE,
    );
    expect(screen.getByRole("status", { hidden: true })).toHaveTextContent(
      SCAN_LOOKUP_UNAVAILABLE,
    );
    expect(getWorkOrder).not.toHaveBeenCalled();
    expect(listWorkOrders).toHaveBeenCalledTimes(1);
  });

  it("honours the list cursor when MORE is pressed", async () => {
    const user = userEvent.setup();
    const cursor = "01932c5a-8b10-7001-8000-000000000042";
    vi.mocked(listWorkOrders)
      .mockResolvedValueOnce({
        data: [sampleWorkOrder],
        nextCursor: cursor,
        hasMore: true,
      })
      .mockResolvedValueOnce({
        data: [secondWorkOrder],
        nextCursor: null,
        hasMore: false,
      });
    vi.mocked(getItem).mockImplementation(async (itemId: string) => {
      if (itemId === sampleItem.id) {
        return sampleItem;
      }
      if (itemId === barItem.id) {
        return barItem;
      }
      throw new Error(`unexpected item ${itemId}`);
    });

    renderAt("/floor/work-orders");
    expect(await screen.findByText("WO-0001")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "MORE" }));

    expect(await screen.findByText("WO-0002")).toBeInTheDocument();
    expect(await screen.findByText("RM-TI-BAR-12")).toBeInTheDocument();
    expect(listWorkOrders).toHaveBeenNthCalledWith(2, {
      limit: 20,
      cursor,
      status: undefined,
    });
  });

  it("sends the status filter on listWorkOrders", async () => {
    const user = userEvent.setup();
    mockListPage([sampleWorkOrder]);
    mockListPage([sampleWorkOrder]);
    vi.mocked(getItem).mockResolvedValue(sampleItem);

    renderAt("/floor/work-orders");
    await screen.findByRole("link", { name: /WO-0001/ });

    await user.click(screen.getByRole("button", { name: "Completed" }));

    await waitFor(() => {
      expect(listWorkOrders).toHaveBeenLastCalledWith({
        limit: 20,
        cursor: undefined,
        status: "completed",
      });
    });
  });
});

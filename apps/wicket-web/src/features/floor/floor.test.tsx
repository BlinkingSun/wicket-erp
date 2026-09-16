import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { parseWorkOrderBody } from "../../api/map-work-order";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { UNBACKED_FLOOR } from "../../api/view-models";
import { ShopFloorTerminal } from "./ShopFloorTerminal";
import { SCAN_LOOKUP_UNAVAILABLE } from "./ScanField";

const sampleWorkOrder: WorkOrderView = {
  id: "01932c5a-8b10-7001-8000-000000000042",
  number: "WO-1042",
  itemId: "01932c5a-8b10-7001-8000-000000000001",
  quantityAmount: "25",
  quantityDimension: "Count",
  status: "in_process",
  statusLabel: "In Process",
  revision: "B",
  wipLocationId: "01932c5a-8b10-7001-8000-000000000010",
  version: 4,
};

const sampleItem: ItemMasterView = {
  id: sampleWorkOrder.itemId,
  number: "PN-880",
  revision: "B",
  description: "Fixture plate, 4140, 12 mm.",
  kindLabel: "Make",
  stockUomLabel: "EA",
  statusLabel: "Released",
  version: 2,
};

vi.mock("../../api/client", () => ({
  getWorkOrder: vi.fn(),
  getItem: vi.fn(),
  getOwnProfile: vi.fn(),
}));

import { getItem, getOwnProfile, getWorkOrder } from "../../api/client";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function renderTerminal(workOrderId?: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ShopFloorTerminal workOrderId={workOrderId} />
    </QueryClientProvider>,
  );
}

describe("shop-floor terminal", () => {
  it("parses the getWorkOrder wo_json shape, including a null number", () => {
    const view = parseWorkOrderBody({
      id: sampleWorkOrder.id,
      number: null,
      item_id: sampleWorkOrder.itemId,
      quantity: { amount: "25.00000000", unit: 1, dimension: "Count" },
      status: "draft",
      revision: "B",
      wip_location_id: null,
      version: 1,
      application_version: "0.0.0",
      configuration_version: "0.0.0",
      released_at: null,
      completed_at: null,
    });
    expect(view.number).toBeNull();
    expect(view.quantityAmount).toBe("25");
    expect(view.quantityDimension).toBe("Count");
    expect(view.statusLabel).toBe("Draft");
    expect(view.itemId).toBe(sampleWorkOrder.itemId);
  });

  it("renders the header from a mocked getWorkOrder", async () => {
    vi.mocked(getWorkOrder).mockResolvedValueOnce(sampleWorkOrder);
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);
    vi.mocked(getOwnProfile).mockResolvedValueOnce({
      id: "01932c5a-8b10-7001-8000-000000000099",
      username: "atester",
      displayName: "A. Tester",
    });

    renderTerminal(sampleWorkOrder.id);

    expect(await screen.findByRole("heading", { name: "WO-1042" })).toBeInTheDocument();
    expect(screen.getByText("In Process")).toHaveClass("status-pill");
    expect(
      await screen.findByText("PN-880 Rev B | Fixture plate, 4140, 12 mm."),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Ordered quantity")).toHaveTextContent("25");
    expect(screen.getByLabelText("Ordered quantity")).toHaveTextContent("ORDERED");
    expect(getWorkOrder).toHaveBeenCalledWith(sampleWorkOrder.id);
    expect(getItem).toHaveBeenCalledWith(sampleWorkOrder.itemId);
    expect(await screen.findByText("A. Tester")).toBeInTheDocument();
  });

  it("shows the identifier-lookup message on scan submit and echoes the input", async () => {
    vi.mocked(getOwnProfile).mockRejectedValueOnce(new Error("unauthenticated"));
    const user = userEvent.setup();

    renderTerminal();

    const scan = screen.getByLabelText("Scan traveler or badge");
    await user.type(scan, "WO-1042");
    await user.click(screen.getByRole("button", { name: "TAP OR SCAN" }));

    const note = await screen.findByText(
      `Received "WO-1042". ${SCAN_LOOKUP_UNAVAILABLE}`,
    );
    expect(note).toBeInTheDocument();
    expect(note).toHaveAttribute("role", "status");
    expect(note.textContent).toContain(SCAN_LOOKUP_UNAVAILABLE);
  });

  it("disables each unbacked control and labels what it needs", async () => {
    vi.mocked(getWorkOrder).mockResolvedValueOnce(sampleWorkOrder);
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);
    vi.mocked(getOwnProfile).mockRejectedValueOnce(new Error("unauthenticated"));

    renderTerminal(sampleWorkOrder.id);
    await screen.findByRole("heading", { name: "WO-1042" });

    const clockOn = screen.getByRole("button", { name: /CLOCK ON/ });
    const reportQty = screen.getByRole("button", { name: /REPORT QTY/ });
    const reportScrap = screen.getByRole("button", { name: /REPORT SCRAP/ });

    expect(clockOn).toBeDisabled();
    expect(reportQty).toBeDisabled();
    expect(reportScrap).toBeDisabled();
    expect(clockOn).toHaveTextContent(UNBACKED_FLOOR.clockOn.operationId);
    expect(reportQty).toHaveTextContent(UNBACKED_FLOOR.reportQuantity.operationId);
    expect(reportQty).toHaveTextContent("completeWorkOrder");
    expect(reportScrap).toHaveTextContent(UNBACKED_FLOOR.reportScrap.operationId);

    expect(screen.getByText(UNBACKED_FLOOR.clockOn.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.reportQuantity.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.reportScrap.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.certification.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.gage.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.operation.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.workCentre.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.remaining.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.drawing.operationId)).toBeInTheDocument();
    expect(screen.getByText(UNBACKED_FLOOR.identifierLookup.operationId)).toBeInTheDocument();
    expect(screen.getByText(/approveCalibration/)).toBeInTheDocument();
    expect(reportQty.textContent).toContain("quantity");
    expect(reportQty.textContent).toContain("If-Match");
  });
});

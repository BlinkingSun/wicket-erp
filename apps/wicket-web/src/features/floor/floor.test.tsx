import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRouter,
} from "@tanstack/react-router";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { parseWorkOrderBody } from "../../api/map-work-order";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { UNBACKED_FLOOR } from "../../api/view-models";
import { routeTree } from "../../router";
import { TRAVELER_SCAN_MESSAGE } from "./ScanField";
import {
  GAGE_REASON,
  REPORT_QTY_REASON,
  REPORT_SCRAP_REASON,
  SCAN_LOOKUP_UNAVAILABLE,
  unbackedReason,
} from "./unbacked";

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
  listWorkOrders: vi.fn(),
}));

import { getItem, getOwnProfile, getWorkOrder } from "../../api/client";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

async function renderTerminal(workOrderId?: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const path = workOrderId
    ? `/floor/work-orders/${workOrderId}`
    : "/floor";
  const router = createRouter({
    routeTree,
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
  await screen.findByLabelText("Scan traveler or badge");
  return view;
}

function visibleCopy(container: HTMLElement): string {
  const clone = container.cloneNode(true) as HTMLElement;
  clone.querySelectorAll(".floor-sr-only").forEach((node) => node.remove());
  return (clone.textContent ?? "").replace(/\s+/g, " ").trim();
}

const OPERATION_IDS = [
  ...Object.values(UNBACKED_FLOOR).map((capability) => capability.operationId),
  "completeWorkOrder",
  "approveCalibration",
] as const;

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
    vi.mocked(getWorkOrder).mockResolvedValue(sampleWorkOrder);
    vi.mocked(getItem).mockResolvedValue(sampleItem);
    vi.mocked(getOwnProfile).mockResolvedValue({
      id: "01932c5a-8b10-7001-8000-000000000099",
      username: "atester",
      displayName: "A. Tester",
    });

    await renderTerminal(sampleWorkOrder.id);

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
    expect(
      screen.getAllByRole("link", { name: "Back to work orders" }),
    ).toHaveLength(1);
  });

  it("shows the identifier-lookup message on scan submit and echoes the input", async () => {
    vi.mocked(getOwnProfile).mockRejectedValue(new Error("unauthenticated"));
    const user = userEvent.setup();

    await renderTerminal();

    const scan = screen.getByLabelText("Scan traveler or badge");
    expect(scan).toHaveAttribute("title", SCAN_LOOKUP_UNAVAILABLE);
    expect(scan).toHaveAttribute("aria-describedby", "floor-scan-note");
    await user.type(scan, "WO-1042");
    await user.click(screen.getByRole("button", { name: "TAP OR SCAN" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      TRAVELER_SCAN_MESSAGE,
    );
    const note = await screen.findByRole("status");
    expect(note).toHaveAttribute("id", "floor-scan-note");
    expect(note).toHaveClass("floor-sr-only");
    expect(note.textContent).toContain('Received "WO-1042"');
    expect(note.textContent).toContain(SCAN_LOOKUP_UNAVAILABLE);
    expect(getWorkOrder).not.toHaveBeenCalled();
  });

  it("disables each unbacked control and labels what it needs", async () => {
    vi.mocked(getWorkOrder).mockResolvedValue(sampleWorkOrder);
    vi.mocked(getItem).mockResolvedValue(sampleItem);
    vi.mocked(getOwnProfile).mockRejectedValue(new Error("unauthenticated"));

    await renderTerminal(sampleWorkOrder.id);
    await screen.findByRole("heading", { name: "WO-1042" });

    const clockOn = screen.getByRole("button", { name: "CLOCK ON" });
    const reportQty = screen.getByRole("button", { name: "REPORT QTY" });
    const reportScrap = screen.getByRole("button", { name: "REPORT SCRAP" });

    expect(clockOn).toBeDisabled();
    expect(reportQty).toBeDisabled();
    expect(reportScrap).toBeDisabled();
    expect(clockOn).toHaveTextContent(/^CLOCK ON$/);
    expect(reportQty).toHaveTextContent(/^REPORT QTY$/);
    expect(reportScrap).toHaveTextContent(/^REPORT SCRAP$/);

    expect(clockOn).toHaveAttribute(
      "title",
      unbackedReason(UNBACKED_FLOOR.clockOn),
    );
    expect(clockOn).toHaveAttribute("aria-describedby", "clock-on-need");
    expect(reportQty).toHaveAttribute("title", REPORT_QTY_REASON);
    expect(reportQty).toHaveAttribute("aria-describedby", "report-qty-need");
    expect(reportScrap).toHaveAttribute("title", REPORT_SCRAP_REASON);
    expect(reportScrap).toHaveAttribute("aria-describedby", "report-scrap-need");

    expect(document.getElementById("clock-on-need")).toHaveTextContent(
      UNBACKED_FLOOR.clockOn.operationId,
    );
    expect(document.getElementById("report-qty-need")).toHaveTextContent(
      UNBACKED_FLOOR.reportQuantity.operationId,
    );
    expect(document.getElementById("report-scrap-need")).toHaveTextContent(
      UNBACKED_FLOOR.reportScrap.operationId,
    );
    expect(document.getElementById("cert-need")).toHaveTextContent(
      UNBACKED_FLOOR.certification.operationId,
    );
    expect(document.getElementById("gage-need")).toHaveTextContent(
      UNBACKED_FLOOR.gage.operationId,
    );
    expect(document.getElementById("drawing-need")).toHaveTextContent(
      UNBACKED_FLOOR.drawing.operationId,
    );
    expect(document.getElementById("qty-need")).toHaveTextContent(
      UNBACKED_FLOOR.remaining.operationId,
    );
    expect(document.getElementById("floor-scan-note")).toHaveTextContent(
      UNBACKED_FLOOR.identifierLookup.operationId,
    );
    expect(document.getElementById("report-qty-need")?.textContent).toContain(
      "completeWorkOrder",
    );
    expect(document.getElementById("gage-need")?.textContent).toContain(
      "approveCalibration",
    );

    const cert = screen.getByText("CERT");
    const gage = screen.getByText("GAGE");
    expect(cert).toHaveClass("status-pill");
    expect(gage).toHaveClass("status-pill");
    expect(cert).toHaveAttribute("aria-disabled", "true");
    expect(gage).toHaveAttribute("aria-disabled", "true");
    expect(cert).toHaveAttribute(
      "title",
      unbackedReason(UNBACKED_FLOOR.certification),
    );
    expect(gage).toHaveAttribute("title", GAGE_REASON);
    expect(screen.getByLabelText("Part drawing")).toHaveAttribute(
      "title",
      unbackedReason(UNBACKED_FLOOR.drawing),
    );
  });

  it("exposes no operation identifier in visible copy", async () => {
    vi.mocked(getWorkOrder).mockResolvedValue(sampleWorkOrder);
    vi.mocked(getItem).mockResolvedValue(sampleItem);
    vi.mocked(getOwnProfile).mockResolvedValue({
      id: "01932c5a-8b10-7001-8000-000000000099",
      username: "atester",
      displayName: "A. Tester",
    });

    const { container } = await renderTerminal(sampleWorkOrder.id);
    await screen.findByRole("heading", { name: "WO-1042" });
    await screen.findByText("A. Tester");

    const visible = visibleCopy(container);
    for (const id of OPERATION_IDS) {
      expect(visible).not.toContain(id);
    }
    expect(visible).not.toMatch(/\bunavailable\b/i);
    expect(visible).not.toContain("NOT YET AVAILABLE");
    expect(visible).not.toContain("OP ");
    expect(visible).not.toContain("WC ");
  });
});

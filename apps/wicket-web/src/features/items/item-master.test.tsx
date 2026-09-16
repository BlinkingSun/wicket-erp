import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ItemMasterView } from "../../api/view-models";
import { UNBACKED_TAB_CAPABILITIES } from "../../api/view-models";
import { BOM_COLUMN_HEADERS } from "./bom-columns";
import { ItemMasterScreen } from "./ItemMasterScreen";

const sampleItem: ItemMasterView = {
  id: "01932c5a-8b10-7001-8000-000000000001",
  number: "MDS-450-M4x12",
  revision: "C",
  description:
    "Cortical Bone Screw, Ti-6Al-4V ELI, M4 x 12 mm, hex drive, ASTM F136.",
  kindLabel: "Make",
  stockUomLabel: "EA",
  statusLabel: "Released",
  version: 3,
};

vi.mock("../../api/client", () => ({
  getItem: vi.fn(),
  getOnHand: vi.fn(),
  updateItem: vi.fn(),
}));

import { getItem, getOnHand } from "../../api/client";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function renderScreen(itemId: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <ItemMasterScreen itemId={itemId} />
    </QueryClientProvider>,
  );
}

describe("item master screen", () => {
  it("renders header, tabs, and BOM column headers from mocked getItem", async () => {
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);

    renderScreen(sampleItem.id);

    const heading = await screen.findByRole("heading", { name: sampleItem.number });
    expect(heading).toBeInTheDocument();
    expect(
      within(heading.parentElement as HTMLElement).getByText(`Rev ${sampleItem.revision}`),
    ).toHaveClass("item-master__revision");
    expect(screen.getByText(sampleItem.statusLabel)).toHaveClass("status-pill");
    expect(screen.getByText("Make • EA")).toBeInTheDocument();
    expect(screen.getByText(sampleItem.description)).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    const save = screen.getByRole("button", { name: "Save" });
    expect(save).toBeDisabled();
    expect(save).toHaveAttribute("title", expect.stringContaining("No fields"));

    expect(screen.getByRole("tab", { name: "Bill of Material" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tab", { name: "Routing" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Inventory" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Documents" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Where Used" })).toBeInTheDocument();

    for (const label of BOM_COLUMN_HEADERS) {
      expect(screen.getByRole("columnheader", { name: label })).toBeInTheDocument();
    }

    expect(screen.getByText(/getItemBom/)).toBeInTheDocument();

    const strip = screen.getByLabelText("Revision history");
    expect(within(strip).getByText(/Rev C/)).toHaveClass("item-revision-strip__entry--current");
    expect(within(strip).getByText(/listItemRevisions/)).toBeInTheDocument();
  });

  it("shows getOnHand totals on the inventory tab", async () => {
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);
    vi.mocked(getOnHand).mockResolvedValueOnce({ onHand: "42", available: "40" });
    const user = userEvent.setup();

    renderScreen(sampleItem.id);
    await screen.findByRole("heading", { name: sampleItem.number });

    await user.click(screen.getByRole("tab", { name: "Inventory" }));

    const panel = screen.getByRole("tabpanel");
    expect(within(panel).getByText("42")).toBeInTheDocument();
    expect(within(panel).getByText("40")).toBeInTheDocument();
    expect(within(panel).getByText(/getOnHand/)).toBeInTheDocument();
    expect(getOnHand).toHaveBeenCalledWith({ itemId: sampleItem.id });
  });

  it.each(
    Object.entries(UNBACKED_TAB_CAPABILITIES) as [
      keyof typeof UNBACKED_TAB_CAPABILITIES,
      (typeof UNBACKED_TAB_CAPABILITIES)[keyof typeof UNBACKED_TAB_CAPABILITIES],
    ][],
  )("shows a Not yet available state on the $1 tab", async (tabId, capability) => {
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);
    const user = userEvent.setup();
    const tabLabel =
      tabId === "where-used"
        ? "Where Used"
        : capability.label;

    renderScreen(sampleItem.id);
    await screen.findByRole("heading", { name: sampleItem.number });

    await user.click(screen.getByRole("tab", { name: tabLabel }));

    const panel = screen.getByRole("tabpanel");
    expect(within(panel).getByText(new RegExp(capability.operationId))).toBeInTheDocument();
  });
});

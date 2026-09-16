import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ItemMasterView } from "../../api/view-models";
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
  updateItem: vi.fn(),
}));

import { getItem } from "../../api/client";

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

    expect(await screen.findByRole("heading", { name: sampleItem.number })).toBeInTheDocument();
    expect(screen.getByText(`Rev ${sampleItem.revision}`)).toBeInTheDocument();
    expect(screen.getByText(sampleItem.statusLabel)).toBeInTheDocument();
    expect(screen.getByText("Make • EA")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();

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
  });

  it("shows a Not yet available state on an unbacked tab", async () => {
    vi.mocked(getItem).mockResolvedValueOnce(sampleItem);
    const user = userEvent.setup();

    renderScreen(sampleItem.id);
    await screen.findByRole("heading", { name: sampleItem.number });

    await user.click(screen.getByRole("tab", { name: "Routing" }));

    const routingTabs = screen.getAllByRole("tab", { name: "Routing" });
    expect(routingTabs.some((tab) => tab.getAttribute("aria-selected") === "true")).toBe(
      true,
    );
    const routingPanel = screen.getByRole("tabpanel");
    expect(within(routingPanel).getByText(/getItemRouting/)).toBeInTheDocument();
  });
});

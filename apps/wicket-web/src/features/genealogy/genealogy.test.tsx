import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createMemoryHistory, createRouter } from "@tanstack/react-router";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { mapTraceResponse } from "../../api/map-trace";
import type { TraceQueryDirection, TraceTreeNodeView } from "../../api/view-models";
import { routeTree } from "../../router";
import backwardFixture from "./fixtures/trace-backward.json";
import bothFixture from "./fixtures/trace-both.json";
import forwardFixture from "./fixtures/trace-forward.json";
import { GenealogyQuery, HUMAN_ID_NOTE } from "./GenealogyQuery";
import { GenealogyScreen } from "./GenealogyScreen";
import { GenealogyTrace } from "./GenealogyTrace";
import { truncateUuid } from "./truncateUuid";

const { mockNavigate } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
}));

vi.mock("../../api/client", () => ({
  traceGenealogy: vi.fn(),
}));

vi.mock("@tanstack/react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tanstack/react-router")>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

import { traceGenealogy } from "../../api/client";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function findNodeByAmount(
  nodes: TraceTreeNodeView[],
  amount: string,
): TraceTreeNodeView | undefined {
  for (const node of nodes) {
    if (node.amount === amount) {
      return node;
    }
    const nested = findNodeByAmount(node.children, amount);
    if (nested) {
      return nested;
    }
  }
  return undefined;
}

function renderQuery(
  direction: TraceQueryDirection = "forward",
  onDirectionChange: (direction: TraceQueryDirection) => void = () => undefined,
) {
  return render(
    <GenealogyQuery
      initialLotId=""
      direction={direction}
      onTrace={() => undefined}
      onDirectionChange={onDirectionChange}
    />,
  );
}

function renderScreen(args: {
  fromLotId?: string;
  direction: TraceQueryDirection;
}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <GenealogyScreen fromLotId={args.fromLotId} direction={args.direction} />
    </QueryClientProvider>,
  );
}

describe("genealogy screen", () => {
  it("maps and paints a captured backward trace with nested children", () => {
    const mapped = mapTraceResponse(backwardFixture);
    expect(mapped.kind).toBe("inline");
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    expect(mapped.direction).toBe("backward");

    render(<GenealogyTrace trace={mapped} />);

    expect(screen.getByLabelText("Genealogy trace")).toBeInTheDocument();
    expect(screen.getByText(/Direction backward/)).toBeInTheDocument();
    expect(
      screen.getByTitle("01a0a8ac-ad51-713a-aaa8-491c2a86f625"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Edge 3000 Length/)).toBeInTheDocument();
    expect(screen.getByText("Amount 23.652000 USD")).toBeInTheDocument();
    expect(screen.queryByText(/currency 840/)).not.toBeInTheDocument();
  });

  it("maps and paints a captured forward trace", () => {
    const mapped = mapTraceResponse(forwardFixture);
    expect(mapped.kind).toBe("inline");
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    expect(mapped.direction).toBe("forward");

    render(<GenealogyTrace trace={mapped} />);

    expect(screen.getByLabelText("Genealogy trace")).toBeInTheDocument();
    expect(screen.getByText(/Direction forward/)).toBeInTheDocument();
    expect(
      screen.getAllByTitle("01a0a8ac-b877-7287-a490-976b988b00e4").length,
    ).toBeGreaterThan(0);
  });

  it("maps and paints a captured both-direction trace", () => {
    const mapped = mapTraceResponse(bothFixture);
    expect(mapped.kind).toBe("both");
    if (mapped.kind !== "both") {
      throw new Error("expected both trace");
    }

    render(<GenealogyTrace trace={mapped} />);

    expect(screen.getByLabelText("Genealogy trace backward")).toBeInTheDocument();
    expect(screen.getByLabelText("Genealogy trace forward")).toBeInTheDocument();
    expect(screen.getByText("Backward")).toBeInTheDocument();
    expect(screen.getByText("Forward")).toBeInTheDocument();
    expect(screen.getAllByText(/Edge 3000 Length/).length).toBeGreaterThan(0);
  });

  it("maps a job_id response as a job view, not an inline tree", () => {
    const mapped = mapTraceResponse({
      job_id: "01932c5a-8b10-7001-8000-000000000099",
      result_url: "/api/v1/genealogy/jobs/01932c5a-8b10-7001-8000-000000000099",
    });
    expect(mapped).toEqual({
      kind: "job",
      jobId: "01932c5a-8b10-7001-8000-000000000099",
      resultUrl: "/api/v1/genealogy/jobs/01932c5a-8b10-7001-8000-000000000099",
    });
  });

  it("keeps string amounts and quantities through mapping without coercing to number", () => {
    const mapped = mapTraceResponse(backwardFixture);
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    const nested = findNodeByAmount(mapped.nodes, "23.652000");
    expect(nested).toBeDefined();
    expect(typeof nested?.amount).toBe("string");
    expect(nested?.amount).toBe("23.652000");
    expect(typeof nested?.quantity.amount).toBe("string");
    expect(nested?.quantity.amount).toBe("3000.00000000");
    expect(typeof nested?.edgeQuantity.amount).toBe("string");
    expect(nested?.edgeQuantity.amount).toBe("3000.00000000");
    expect(truncateUuid("01a0a8ac-b877-7287-a490-976b988b00e4")).toBe(
      "01a0a8ac…00e4",
    );
  });

  it("renders the engine amount string verbatim next to the alphabetic currency", () => {
    const mapped = mapTraceResponse(backwardFixture);
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    render(<GenealogyTrace trace={mapped} />);
    expect(screen.getByText("Amount 23.652000 USD")).toBeInTheDocument();
    expect(screen.queryByText("23.65")).not.toBeInTheDocument();
    expect(screen.queryByText("23.652")).not.toBeInTheDocument();
  });

  it("falls back to the numeric currency code when the code is not in the map", () => {
    const mapped = mapTraceResponse({
      direction: "backward",
      nodes: [
        {
          amount: "23.652000",
          amount_currency: 999,
          children: [],
          edge_quantity: { amount: "3000.00000000", dimension: "Length", unit: 2 },
          item: null,
          location: null,
          lot: "01a0a8ac-ad51-713a-aaa8-491c2a86f625",
          occurred_at: null,
          posting: 14,
          quantity: { amount: "3000.00000000", dimension: "Length", unit: 2 },
          serial: null,
        },
      ],
    });
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    render(<GenealogyTrace trace={mapped} />);
    expect(screen.getByText("Amount 23.652000 999")).toBeInTheDocument();
    expect(screen.queryByText(/undefined/)).not.toBeInTheDocument();
  });

  it("renders the approved query chrome wired to a lot UUID, with a visible lookup note", () => {
    renderQuery();

    expect(screen.getByRole("search")).toBeInTheDocument();
    expect(screen.getByLabelText("Lot UUID query")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("01932c5a-8b10-7001-8000-000000000003")).toBeInTheDocument();
    expect(screen.getByText(HUMAN_ID_NOTE)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run genealogy trace" })).toBeInTheDocument();
  });

  it("exposes backward, forward, and both as a direction control", async () => {
    const user = userEvent.setup();
    const onDirectionChange = vi.fn();
    renderQuery("backward", onDirectionChange);

    expect(screen.getByRole("radiogroup", { name: "Direction" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "Backward" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("radio", { name: "Forward" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
    expect(screen.getByRole("radio", { name: "Both" })).toHaveAttribute(
      "aria-checked",
      "false",
    );

    await user.click(screen.getByRole("radio", { name: "Both" }));
    expect(onDirectionChange).toHaveBeenCalledWith("both");
    await user.click(screen.getByRole("radio", { name: "Forward" }));
    expect(onDirectionChange).toHaveBeenCalledWith("forward");
  });

  it("writes direction onto the genealogy search params, including both", async () => {
    const user = userEvent.setup();
    const lotId = "01932c5a-8b10-7001-8000-000000000003";
    vi.mocked(traceGenealogy).mockResolvedValue({
      kind: "inline",
      direction: "backward",
      nodes: [],
    });

    renderScreen({ fromLotId: lotId, direction: "backward" });

    expect(traceGenealogy).toHaveBeenCalledWith({
      fromLotId: lotId,
      direction: "backward",
    });

    await user.click(screen.getByRole("radio", { name: "Forward" }));
    expect(mockNavigate).toHaveBeenCalledWith({
      to: "/quality/genealogy",
      search: { from_lot_id: lotId, direction: "forward" },
    });

    await user.click(screen.getByRole("radio", { name: "Both" }));
    expect(mockNavigate).toHaveBeenCalledWith({
      to: "/quality/genealogy",
      search: { from_lot_id: lotId, direction: "both" },
    });
  });

  it("selects both from the route search param", async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const router = createRouter({
      routeTree,
      history: createMemoryHistory({
        initialEntries: ["/quality/genealogy?direction=both"],
      }),
    });
    render(
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("radio", { name: "Both" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });
});

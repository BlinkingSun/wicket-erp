import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { mapTraceResponse } from "../../api/map-trace";
import type { TraceTreeNodeView } from "../../api/view-models";
import backwardFixture from "./fixtures/trace-backward.json";
import bothFixture from "./fixtures/trace-both.json";
import forwardFixture from "./fixtures/trace-forward.json";
import { GenealogyQuery, HUMAN_ID_NOTE } from "./GenealogyQuery";
import { GenealogyTrace } from "./GenealogyTrace";
import { truncateUuid } from "./truncateUuid";

afterEach(() => {
  cleanup();
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
    expect(screen.getByText(/Amount 23\.652000 \(currency 840\)/)).toBeInTheDocument();
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

  it("keeps string amounts through mapping without coercing to number", () => {
    const mapped = mapTraceResponse(backwardFixture);
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }
    const nested = findNodeByAmount(mapped.nodes, "23.652000");
    expect(nested).toBeDefined();
    expect(typeof nested?.amount).toBe("string");
    expect(nested?.amount).toBe("23.652000");
    expect(truncateUuid("01a0a8ac-b877-7287-a490-976b988b00e4")).toBe(
      "01a0a8ac…00e4",
    );
  });

  it("renders the approved query chrome wired to a lot UUID, with a visible lookup note", () => {
    render(<GenealogyQuery initialLotId="" onTrace={() => undefined} />);

    expect(screen.getByRole("search")).toBeInTheDocument();
    expect(screen.getByLabelText("Lot UUID query")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("01932c5a-8b10-7001-8000-000000000003")).toBeInTheDocument();
    expect(screen.getByText(HUMAN_ID_NOTE)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run genealogy trace" })).toBeInTheDocument();
  });
});

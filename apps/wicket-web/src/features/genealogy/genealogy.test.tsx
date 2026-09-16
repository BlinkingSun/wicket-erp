import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { mapTraceResponse } from "../../api/map-trace";
import fixture from "./fixtures/trace.json";
import { GenealogyQuery, HUMAN_ID_NOTE } from "./GenealogyQuery";
import { GenealogyTrace } from "./GenealogyTrace";

describe("genealogy screen", () => {
  it("paints a trace from docs/10 section 9.6 fixture JSON (root, nodes, edges)", () => {
    const mapped = mapTraceResponse(fixture);
    expect(mapped.kind).toBe("inline");
    if (mapped.kind !== "inline") {
      throw new Error("expected inline trace");
    }

    render(<GenealogyTrace trace={mapped} />);

    expect(screen.getByText("HT-ATI-24-8831")).toBeInTheDocument();
    expect(screen.getByText("LOT-BAR-24-4412")).toBeInTheDocument();
    expect(screen.getByText("LOT-WO-1847")).toBeInTheDocument();
    expect(screen.getByText("split")).toBeInTheDocument();
    expect(screen.getByText("transformation")).toBeInTheDocument();
    expect(screen.getByText("WO-2026-1847")).toBeInTheDocument();
    expect(screen.getByText("SN-450-000134")).toBeInTheDocument();
    expect(screen.getByLabelText("Genealogy trace")).toBeInTheDocument();
    expect(screen.getByText(/Root HT-ATI-24-8831/)).toBeInTheDocument();
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

  it("renders the approved query chrome wired to a lot UUID, with a visible lookup note", () => {
    render(<GenealogyQuery initialLotId="" onTrace={() => undefined} />);

    expect(screen.getByRole("search")).toBeInTheDocument();
    expect(screen.getByLabelText("Lot UUID query")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("01932c5a-8b10-7001-8000-000000000003")).toBeInTheDocument();
    expect(screen.getByText(HUMAN_ID_NOTE)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Run genealogy trace" })).toBeInTheDocument();
  });
});

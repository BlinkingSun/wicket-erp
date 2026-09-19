import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  getGenealogyJob,
  traceGenealogy,
  type GenealogyJobState,
  type GenealogyJobStatusView,
} from "../../api/client";
import type { GenealogyResultView, TraceQueryDirection } from "../../api/view-models";
import { GenealogyQuery } from "./GenealogyQuery";
import { GenealogyTrace } from "./GenealogyTrace";
import "./genealogy.css";

const POLL_MS = 1000;
const ACTIVE_STATES: readonly GenealogyJobState[] = ["Queued", "Running"];

type GenealogyScreenProps = {
  fromLotId?: string;
  direction: TraceQueryDirection;
};

export function GenealogyScreen({ fromLotId, direction }: GenealogyScreenProps) {
  const navigate = useNavigate();
  const enabled = Boolean(fromLotId);

  const query = useQuery({
    queryKey: ["genealogy", "trace", fromLotId, direction],
    queryFn: () => {
      if (!fromLotId) {
        throw new Error("fromLotId is required");
      }
      return traceGenealogy({ fromLotId, direction });
    },
    enabled,
  });

  function setSearch(next: { fromLotId?: string; direction: TraceQueryDirection }) {
    void navigate({
      to: "/quality/genealogy",
      search: { from_lot_id: next.fromLotId, direction: next.direction },
    });
  }

  return (
    <div>
      <GenealogyQuery
        key={fromLotId ?? "empty"}
        initialLotId={fromLotId ?? ""}
        direction={direction}
        onTrace={(lotId) => {
          setSearch({ fromLotId: lotId, direction });
        }}
        onDirectionChange={(next) => {
          setSearch({ fromLotId, direction: next });
        }}
      />
      <TraceResult
        enabled={enabled}
        isPending={query.isPending}
        isError={query.isError}
        error={query.error}
        result={query.data}
      />
    </div>
  );
}

function TraceResult({
  enabled,
  isPending,
  isError,
  error,
  result,
}: {
  enabled: boolean;
  isPending: boolean;
  isError: boolean;
  error: Error | null;
  result: GenealogyResultView | undefined;
}) {
  if (!enabled) {
    return (
      <p className="trace-status">
        Enter a lot UUID to load a genealogy trace. This screen reads
        traceGenealogy only.
      </p>
    );
  }
  if (isPending) {
    return <p className="trace-status">Loading genealogy trace.</p>;
  }
  if (isError) {
    return (
      <p className="query-error">
        {error?.message ?? "Genealogy trace request failed."}
      </p>
    );
  }
  if (!result) {
    return null;
  }
  if (result.kind === "job") {
    return <JobPoller jobId={result.jobId} resultUrl={result.resultUrl} />;
  }
  if (result.kind === "both" || result.kind === "inline") {
    return <GenealogyTrace trace={result} />;
  }
  return null;
}

function JobPoller({ jobId, resultUrl }: { jobId: string; resultUrl: string }) {
  const [job, setJob] = useState<GenealogyJobStatusView | null>(null);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    async function poll() {
      try {
        const next = await getGenealogyJob(resultUrl);
        if (cancelled) {
          return;
        }
        setJob(next);
        setError(null);
        if (ACTIVE_STATES.includes(next.state)) {
          timer = setTimeout(() => {
            void poll();
          }, POLL_MS);
        }
      } catch (cause) {
        if (cancelled) {
          return;
        }
        setError(
          cause instanceof Error
            ? cause
            : new Error("Genealogy job request failed."),
        );
      }
    }

    void poll();
    return () => {
      cancelled = true;
      if (timer !== undefined) {
        clearTimeout(timer);
      }
    };
  }, [resultUrl]);

  if (error) {
    return (
      <p className="query-error">
        {error.message}
      </p>
    );
  }

  if (!job) {
    return (
      <p className="trace-status">
        Job <span className="mono">{jobId}</span> accepted. Waiting for the
        first status.
      </p>
    );
  }

  if (job.state === "Succeeded") {
    if (
      job.result &&
      (job.result.kind === "inline" || job.result.kind === "both")
    ) {
      return <GenealogyTrace trace={job.result} />;
    }
    return (
      <p className="trace-status" role="status">
        The engine returned Succeeded without a trace tree.
      </p>
    );
  }

  if (job.state === "Failed" || job.state === "Cancelled") {
    return (
      <div className="job-terminal" role="alert">
        <JobStateMark state={job.state} />
        <p className="query-error">
          {job.lastError ??
            "The engine returned no last_error for this job."}
        </p>
      </div>
    );
  }

  return <JobProgress job={job} />;
}

function JobProgress({ job }: { job: GenealogyJobStatusView }) {
  const barPct = Math.min(100, Math.max(0, job.progressPct));
  return (
    <section className="job-progress-block" aria-label="Genealogy job progress">
      <div className="job-progress-block__meta">
        <JobStateMark state={job.state} />
        <span className="job-progress-block__id mono" title={job.id}>
          {job.id}
        </span>
      </div>
      <div
        className="job-progress"
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={job.progressPct}
        aria-label="Genealogy job percent complete"
      >
        <div className="job-progress__fill" style={{ width: `${barPct}%` }} />
      </div>
      <p className="job-progress-block__pct mono">
        {job.progressPct} percent
      </p>
      {job.progressNote ? (
        <p className="job-progress-block__note">{job.progressNote}</p>
      ) : null}
    </section>
  );
}

function JobStateMark({ state }: { state: GenealogyJobState }) {
  return (
    <span className={`status-mark status-mark--${state.toLowerCase()}`}>
      <span className="status-mark__shape" aria-hidden="true" />
      <span className="status-mark__word">{state}</span>
    </span>
  );
}

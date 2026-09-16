import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { traceGenealogy } from "../../api/client";
import type { GenealogyResultView } from "../../api/view-models";
import { GenealogyQuery } from "./GenealogyQuery";
import { GenealogyTrace } from "./GenealogyTrace";

type GenealogyScreenProps = {
  fromLotId?: string;
  direction: "forward" | "backward";
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

  return (
    <div>
      <GenealogyQuery
        key={fromLotId ?? "empty"}
        initialLotId={fromLotId ?? ""}
        onTrace={(lotId) => {
          void navigate({
            to: "/quality/genealogy",
            search: { from_lot_id: lotId, direction },
          });
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
    return (
      <p className="trace-status">
        This trace was accepted as a background job ({result.jobId}). This
        screen renders inline-sized traces only; job results are not polled.
      </p>
    );
  }
  return <GenealogyTrace trace={result} />;
}

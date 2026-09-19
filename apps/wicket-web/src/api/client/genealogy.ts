// Lane c-gen: getGenealogyJob, getImpact land here.

import { getJson } from "../http";
import { mapTraceResponse } from "../map-trace";
import type { GenealogyResultView, TraceQueryDirection } from "../view-models";

export async function traceGenealogy(args: {
  fromLotId: string;
  direction: TraceQueryDirection;
}): Promise<GenealogyResultView> {
  const params = new URLSearchParams({
    from_lot_id: args.fromLotId,
    direction: args.direction,
  });
  const payload = await getJson(`/api/v1/genealogy/trace?${params.toString()}`);
  return mapTraceResponse(payload);
}

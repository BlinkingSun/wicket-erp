import { useQuery } from "@tanstack/react-query";
import { getOnHand } from "../../api/client";
import { UNBACKED_BOM, UNBACKED_REVISION_HISTORY } from "../../api/view-models";
import { BOM_COLUMN_HEADERS } from "./bom-columns";
import { NotYetAvailable } from "./NotYetAvailable";

type RevisionStripProps = {
  currentRevision: string;
};

export function RevisionStrip({ currentRevision }: RevisionStripProps) {
  return (
    <div className="item-revision-strip" aria-label="Revision history">
      <span className="item-revision-strip__prior">
        Earlier revisions are not available until{" "}
        <span className="mono">{UNBACKED_REVISION_HISTORY.operationId}</span> is
        mounted.
      </span>
      <span className="item-revision-strip__sep" aria-hidden="true">|</span>
      <span className="item-revision-strip__entry item-revision-strip__entry--current">
        Rev {currentRevision}
      </span>
    </div>
  );
}

export function BomTabPanel() {
  return (
    <section className="item-tab-panel" aria-label="Bill of Material">
      <table className="item-data-table">
        <thead>
          <tr>
            {BOM_COLUMN_HEADERS.map((label) => (
              <th key={label} scope="col">
                {label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          <tr>
            <td colSpan={BOM_COLUMN_HEADERS.length}>
              <NotYetAvailable capability={UNBACKED_BOM} />
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  );
}

type InventoryTabPanelProps = {
  itemId: string;
};

export function InventoryTabPanel({ itemId }: InventoryTabPanelProps) {
  const query = useQuery({
    queryKey: ["inventory", "getOnHand", itemId],
    queryFn: () => getOnHand({ itemId }),
  });

  if (query.isPending) {
    return (
      <section className="item-tab-panel" aria-label="Inventory">
        <p className="trace-status">Loading on-hand.</p>
      </section>
    );
  }
  if (query.isError) {
    return (
      <section className="item-tab-panel" aria-label="Inventory">
        <p className="query-error">
          {query.error instanceof Error
            ? query.error.message
            : "On-hand request failed."}
        </p>
      </section>
    );
  }

  const { onHand, available } = query.data;

  return (
    <section className="item-tab-panel" aria-label="Inventory">
      <table className="item-data-table item-inventory-table">
        <thead>
          <tr>
            <th scope="col">Measure</th>
            <th scope="col">Quantity</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>On hand (all locations)</td>
            <td className="mono">{onHand}</td>
          </tr>
          <tr>
            <td>Available</td>
            <td className="mono">{available}</td>
          </tr>
        </tbody>
      </table>
      <p className="item-tab-note">
        Totals from <span className="mono">getOnHand</span> for this item. Per-location
        and lot breakdown is not on this operation.
      </p>
    </section>
  );
}

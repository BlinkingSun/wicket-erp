import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { listLots, type LotListItemView } from "../../api/client";
import { truncateUuid } from "./truncateUuid";
import "./genealogy.css";

export const LOT_LOOKUP_NOTE =
  "Human lot-number lookup does not exist yet. This list is the first pages of listLots using limit and cursor only.";

const PAGE_LIMIT = 20;

export function LotPicker() {
  const navigate = useNavigate();
  const [cursor, setCursor] = useState<string | undefined>(undefined);

  const query = useQuery({
    queryKey: ["genealogy", "lots", cursor ?? "start"],
    queryFn: () => listLots({ limit: PAGE_LIMIT, cursor }),
  });

  if (query.isPending) {
    return <p className="trace-status">Loading lots.</p>;
  }
  if (query.isError) {
    return (
      <p className="query-error">
        {query.error.message ?? "Lot list request failed."}
      </p>
    );
  }
  const page = query.data;
  if (!page) {
    return null;
  }

  return (
    <div className="lot-picker">
      <p className="lot-picker__note">{LOT_LOOKUP_NOTE}</p>
      {page.lots.length === 0 ? (
        <p className="trace-status" role="status">
          listLots returned no lots.
        </p>
      ) : (
        <table className="lot-picker__table">
          <thead>
            <tr>
              <th>Identifier</th>
              <th>Status</th>
              <th>Lot id</th>
              <th>Item id</th>
              <th>Supplier lot</th>
              <th>Trace</th>
            </tr>
          </thead>
          <tbody>
            {page.lots.map((lot) => (
              <LotRow
                key={lot.id}
                lot={lot}
                onTrace={() => {
                  void navigate({
                    to: "/quality/genealogy",
                    search: { from_lot_id: lot.id, direction: "forward" },
                  });
                }}
              />
            ))}
          </tbody>
        </table>
      )}
      {page.hasMore && page.nextCursor ? (
        <button
          type="button"
          className="lot-picker__next"
          onClick={() => {
            setCursor(page.nextCursor ?? undefined);
          }}
        >
          Next page
        </button>
      ) : null}
    </div>
  );
}

function LotRow({
  lot,
  onTrace,
}: {
  lot: LotListItemView;
  onTrace: () => void;
}) {
  return (
    <tr>
      <td className="lot-picker__identifier">{lot.identifier}</td>
      <td>
        <span className={`status-mark status-mark--${lot.status}`}>
          <span className="status-mark__shape" aria-hidden="true" />
          <span className="status-mark__word">{lot.status}</span>
        </span>
      </td>
      <td>
        <abbr className="lot-picker__id" title={lot.id}>
          {truncateUuid(lot.id)}
        </abbr>
      </td>
      <td>
        <abbr className="lot-picker__item" title={lot.itemId}>
          {truncateUuid(lot.itemId)}
        </abbr>
      </td>
      <td className="lot-picker__identifier">{lot.supplierLot}</td>
      <td className="lot-picker__actions">
        <button
          type="button"
          className="lot-picker__trace"
          onClick={onTrace}
        >
          Trace
        </button>
      </td>
    </tr>
  );
}

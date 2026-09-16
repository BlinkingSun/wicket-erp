import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { getItem, getOwnProfile, getWorkOrder } from "../../api/client";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { UNBACKED_FLOOR } from "../../api/view-models";
import { ScanField } from "./ScanField";
import {
  GAGE_REASON,
  REPORT_QTY_REASON,
  REPORT_SCRAP_REASON,
  unbackedReason,
} from "./unbacked";
import "./floor.css";

const MONTHS = [
  "JAN",
  "FEB",
  "MAR",
  "APR",
  "MAY",
  "JUN",
  "JUL",
  "AUG",
  "SEP",
  "OCT",
  "NOV",
  "DEC",
] as const;

type ShopFloorTerminalProps = {
  workOrderId?: string;
};

export function ShopFloorTerminal({ workOrderId }: ShopFloorTerminalProps) {
  const woQuery = useQuery({
    queryKey: ["work-orders", "getWorkOrder", workOrderId],
    queryFn: () => {
      if (!workOrderId) {
        throw new Error("workOrderId is required");
      }
      return getWorkOrder(workOrderId);
    },
    enabled: Boolean(workOrderId),
  });

  const itemId = woQuery.data?.itemId;
  const itemQuery = useQuery({
    queryKey: ["items", "getItem", itemId],
    queryFn: () => {
      if (!itemId) {
        throw new Error("itemId is required");
      }
      return getItem(itemId);
    },
    enabled: Boolean(itemId),
  });

  const profileQuery = useQuery({
    queryKey: ["identity", "getOwnProfile"],
    queryFn: getOwnProfile,
  });

  return (
    <div className="floor-terminal">
      <FloorHeader
        workOrder={woQuery.data}
        item={itemQuery.data}
        woPending={woQuery.isPending && Boolean(workOrderId)}
        woError={woQuery.isError}
        woErrorMessage={
          woQuery.error instanceof Error
            ? woQuery.error.message
            : "Work order request failed."
        }
        itemPending={itemQuery.isPending && Boolean(itemId)}
      />
      <ScanField />
      <FloorActions />
      <FloorStatusBar operatorName={profileQuery.data?.displayName ?? null} />
    </div>
  );
}

function FloorHeader({
  workOrder,
  item,
  woPending,
  woError,
  woErrorMessage,
  itemPending,
}: {
  workOrder?: WorkOrderView;
  item?: ItemMasterView;
  woPending: boolean;
  woError: boolean;
  woErrorMessage: string;
  itemPending: boolean;
}) {
  const heading = woPending
    ? "LOADING"
    : woError
      ? "UNABLE TO LOAD"
      : workOrder
        ? (workOrder.number ?? "UNNUMBERED")
        : "NO WORK ORDER";

  const itemText = itemLine(workOrder, item, itemPending);

  return (
    <header className="floor-header">
      <div className="floor-header__identity">
        <h1 className="floor-header__number mono">{heading}</h1>
        {workOrder ? (
          <span className="status-pill">{workOrder.statusLabel}</span>
        ) : null}
        {itemText ? (
          <p className="floor-header__item">{itemText}</p>
        ) : null}
        {woError ? <p className="floor-header__error">{woErrorMessage}</p> : null}
      </div>
      <div
        className="floor-header__drawing"
        role="img"
        aria-label="Part drawing"
        title={unbackedReason(UNBACKED_FLOOR.drawing)}
        aria-describedby="drawing-need"
      />
      <div
        className="floor-qty"
        aria-label="Ordered quantity"
        title={unbackedReason(UNBACKED_FLOOR.remaining)}
        aria-describedby="qty-need"
      >
        <p className="floor-qty__figures mono">
          <span className="floor-qty__unknown">—</span>
          <span className="floor-qty__sep"> / </span>
          <span>{workOrder ? workOrder.quantityAmount : "—"}</span>
        </p>
        <div className="floor-qty__rule" />
        <p className="floor-qty__label">ORDERED</p>
      </div>
    </header>
  );
}

function itemLine(
  workOrder: WorkOrderView | undefined,
  item: ItemMasterView | undefined,
  itemPending: boolean,
): string | null {
  if (!workOrder) {
    return null;
  }
  if (item) {
    return `${item.number} Rev ${workOrder.revision} | ${item.description}`;
  }
  if (itemPending) {
    return null;
  }
  return workOrder.revision ? `Rev ${workOrder.revision}` : null;
}

function FloorActions() {
  const clockOnReason = unbackedReason(UNBACKED_FLOOR.clockOn);
  return (
    <div className="floor-actions">
      <span className="floor-action-hit" title={clockOnReason}>
        <button
          type="button"
          className="floor-action floor-action--primary"
          disabled
          title={clockOnReason}
          aria-describedby="clock-on-need"
        >
          <span className="floor-action__label">CLOCK ON</span>
        </button>
      </span>
      <span className="floor-action-hit" title={REPORT_QTY_REASON}>
        <button
          type="button"
          className="floor-action floor-action--secondary"
          disabled
          title={REPORT_QTY_REASON}
          aria-describedby="report-qty-need"
        >
          <span className="floor-action__label">REPORT QTY</span>
        </button>
      </span>
      <span className="floor-action-hit" title={REPORT_SCRAP_REASON}>
        <button
          type="button"
          className="floor-action floor-action--secondary"
          disabled
          title={REPORT_SCRAP_REASON}
          aria-describedby="report-scrap-need"
        >
          <span className="floor-action__label">REPORT SCRAP</span>
        </button>
      </span>
    </div>
  );
}

function UnbackedDescriptions() {
  return (
    <div className="floor-sr-only">
      <span id="clock-on-need">{unbackedReason(UNBACKED_FLOOR.clockOn)}</span>
      <span id="report-qty-need">{REPORT_QTY_REASON}</span>
      <span id="report-scrap-need">{REPORT_SCRAP_REASON}</span>
      <span id="drawing-need">{unbackedReason(UNBACKED_FLOOR.drawing)}</span>
      <span id="qty-need">{unbackedReason(UNBACKED_FLOOR.remaining)}</span>
      <span id="cert-need">{unbackedReason(UNBACKED_FLOOR.certification)}</span>
      <span id="gage-need">{GAGE_REASON}</span>
    </div>
  );
}

function FloorStatusBar({ operatorName }: { operatorName: string | null }) {
  return (
    <footer className="floor-status">
      <span className="floor-status__operator">
        {operatorName ?? "No session"}
      </span>
      <span
        className="status-pill"
        aria-disabled="true"
        title={unbackedReason(UNBACKED_FLOOR.certification)}
        aria-describedby="cert-need"
      >
        CERT
      </span>
      <span
        className="status-pill"
        aria-disabled="true"
        title={GAGE_REASON}
        aria-describedby="gage-need"
      >
        GAGE
      </span>
      <FloorClock />
      <UnbackedDescriptions />
    </footer>
  );
}

function FloorClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(id);
  }, []);
  return (
    <time className="floor-clock mono" dateTime={now.toISOString()}>
      {formatFloorClock(now)}
    </time>
  );
}

function formatFloorClock(date: Date): string {
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const month = MONTHS[date.getMonth()] ?? "JAN";
  const year = String(date.getFullYear());
  return `${hours}:${minutes} | ${day} ${month} ${year}`;
}

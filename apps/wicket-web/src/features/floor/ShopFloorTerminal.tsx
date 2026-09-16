import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { getItem, getOwnProfile, getWorkOrder } from "../../api/client";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { UNBACKED_FLOOR } from "../../api/view-models";
import { NotYetAvailable } from "../items/NotYetAvailable";
import "../items/items.css";
import { ScanField } from "./ScanField";
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
        workOrderId={workOrderId}
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
      <FloorStatusBar
        operatorName={profileQuery.data?.displayName ?? null}
      />
    </div>
  );
}

function FloorHeader({
  workOrderId,
  workOrder,
  item,
  woPending,
  woError,
  woErrorMessage,
  itemPending,
}: {
  workOrderId?: string;
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
      ? "UNAVAILABLE"
      : workOrder
        ? (workOrder.number ?? "UNNUMBERED")
        : "NO WORK ORDER";

  return (
    <header className="floor-header">
      <div className="floor-header__identity">
        <h1 className="floor-header__number mono">{heading}</h1>
        {workOrder ? (
          <span className="status-pill">{workOrder.statusLabel}</span>
        ) : null}
        <p className="floor-header__op">
          <span className="floor-header__op-label">OP</span>
          <span className="floor-header__op-gap">unavailable</span>
          <span className="floor-header__rule" aria-hidden="true">
            |
          </span>
          <span className="floor-header__wc">WC unavailable</span>
        </p>
        <p className="floor-header__need">
          {UNBACKED_FLOOR.operation.label} requires{" "}
          <span className="mono">{UNBACKED_FLOOR.operation.operationId}</span>
          {" · "}
          {UNBACKED_FLOOR.workCentre.label} requires{" "}
          <span className="mono">{UNBACKED_FLOOR.workCentre.operationId}</span>
          , which are not mounted on the engine yet.
        </p>
        <p className="floor-header__item">{itemLine(workOrder, item, itemPending)}</p>
        {woError ? <p className="floor-header__error">{woErrorMessage}</p> : null}
        {!workOrderId && !woPending ? (
          <p className="floor-header__need">
            Scan cannot load a work order until{" "}
            <span className="mono">{UNBACKED_FLOOR.identifierLookup.operationId}</span>{" "}
            is mounted. Open{" "}
            <span className="mono">/floor/work-orders/&lt;id&gt;</span> with a work-order
            UUID to bind live getWorkOrder data.
          </p>
        ) : null}
      </div>
      <div className="floor-header__drawing" aria-label="Part drawing">
        <NotYetAvailable capability={UNBACKED_FLOOR.drawing} />
      </div>
      <div className="floor-qty" aria-label="Ordered quantity">
        <p className="floor-qty__figures mono">
          <span className="floor-qty__unknown">—</span>
          <span className="floor-qty__sep"> / </span>
          <span>{workOrder ? workOrder.quantityAmount : "—"}</span>
        </p>
        <div className="floor-qty__rule" />
        <p className="floor-qty__label">ORDERED</p>
        <p className="floor-qty__need">
          {UNBACKED_FLOOR.remaining.label} requires operation{" "}
          <span className="mono">{UNBACKED_FLOOR.remaining.operationId}</span>, which
          is not mounted. getWorkOrder returns ordered quantity only.
        </p>
      </div>
    </header>
  );
}

function itemLine(
  workOrder: WorkOrderView | undefined,
  item: ItemMasterView | undefined,
  itemPending: boolean,
): string {
  if (!workOrder) {
    return "Item reference is empty until a work order is bound.";
  }
  if (item) {
    return `${item.number} Rev ${workOrder.revision} | ${item.description}`;
  }
  if (itemPending) {
    return `Loading item ${workOrder.itemId}`;
  }
  return `Item ${workOrder.itemId} Rev ${workOrder.revision}`;
}

function FloorActions() {
  return (
    <div className="floor-actions">
      <button
        type="button"
        className="floor-action floor-action--primary"
        disabled
        aria-describedby="clock-on-need"
      >
        <span className="floor-action__label">CLOCK ON</span>
        <span id="clock-on-need" className="floor-action__need">
          {UNBACKED_FLOOR.clockOn.label} requires operation{" "}
          <span className="mono">{UNBACKED_FLOOR.clockOn.operationId}</span>, which is
          not mounted on the engine yet.
        </span>
      </button>
      <button
        type="button"
        className="floor-action floor-action--secondary"
        disabled
        aria-describedby="report-qty-need"
      >
        <span className="floor-action__label">REPORT QTY</span>
        <span id="report-qty-need" className="floor-action__need">
          <span className="mono">completeWorkOrder</span> accepts quantity but
          completes the whole work order (quantity, location_id, If-Match). Incremental
          qty needs{" "}
          <span className="mono">{UNBACKED_FLOOR.reportQuantity.operationId}</span>,
          which is not mounted.
        </span>
      </button>
      <button
        type="button"
        className="floor-action floor-action--secondary"
        disabled
        aria-describedby="report-scrap-need"
      >
        <span className="floor-action__label">REPORT SCRAP</span>
        <span id="report-scrap-need" className="floor-action__need">
          {UNBACKED_FLOOR.reportScrap.label} requires operation{" "}
          <span className="mono">{UNBACKED_FLOOR.reportScrap.operationId}</span>, which
          is not mounted on the engine yet. Scrap on{" "}
          <span className="mono">completeWorkOrder</span> is a field of terminal
          complete, not shop-floor scrap reporting.
        </span>
      </button>
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
        aria-describedby="cert-need"
      >
        CERT
      </span>
      <span id="cert-need" className="floor-status__need">
        {UNBACKED_FLOOR.certification.label} requires operation{" "}
        <span className="mono">{UNBACKED_FLOOR.certification.operationId}</span>, which
        is not mounted on the engine yet.
      </span>
      <span
        className="status-pill"
        aria-disabled="true"
        aria-describedby="gage-need"
      >
        GAGE
      </span>
      <span className="floor-status__check" aria-hidden="true">
        <CheckGlyph />
      </span>
      <span id="gage-need" className="floor-status__need">
        {UNBACKED_FLOOR.gage.label} requires operation{" "}
        <span className="mono">{UNBACKED_FLOOR.gage.operationId}</span>, which is not
        mounted on the engine yet.{" "}
        <span className="mono">approveCalibration</span> is an approval, not
        current-cal status.
      </span>
      <FloorClock />
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

function CheckGlyph() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M2.5 8.5 L6.2 12.2 L13.5 3.8"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

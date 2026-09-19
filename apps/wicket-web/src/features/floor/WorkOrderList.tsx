import { useInfiniteQuery, useQueries } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useMemo, useState } from "react";
import { getItem, listWorkOrders } from "../../api/client";
import type { ItemMasterView, WorkOrderView } from "../../api/view-models";
import { ScanField } from "./ScanField";
import "./floor.css";

const PAGE_SIZE = 20;

const STATUS_FILTERS = [
  { value: "", label: "All" },
  { value: "draft", label: "Draft" },
  { value: "released", label: "Released" },
  { value: "in_process", label: "In Process" },
  { value: "completed", label: "Completed" },
  { value: "cancelled", label: "Cancelled" },
] as const;

export function WorkOrderList() {
  const [status, setStatus] = useState("");
  const listQuery = useInfiniteQuery({
    queryKey: ["work-orders", "listWorkOrders", status || null],
    queryFn: ({ pageParam }) =>
      listWorkOrders({
        limit: PAGE_SIZE,
        cursor: pageParam,
        status: status || undefined,
      }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) =>
      lastPage.hasMore && lastPage.nextCursor ? lastPage.nextCursor : undefined,
  });

  const workOrders = useMemo(
    () => listQuery.data?.pages.flatMap((page) => page.data) ?? [],
    [listQuery.data],
  );

  const itemIds = useMemo(
    () => [...new Set(workOrders.map((workOrder) => workOrder.itemId))],
    [workOrders],
  );

  const itemQueries = useQueries({
    queries: itemIds.map((itemId) => ({
      queryKey: ["items", "getItem", itemId],
      queryFn: () => getItem(itemId),
    })),
  });

  const itemsById = new Map<string, ItemMasterView>();
  itemIds.forEach((itemId, index) => {
    const item = itemQueries[index]?.data;
    if (item) {
      itemsById.set(itemId, item);
    }
  });

  return (
    <div className="floor-pick-page">
      <header className="floor-pick-header">
        <h1>Work orders</h1>
        <p className="floor-pick-lead">Pick the work order you are running.</p>
      </header>
      <ScanField compact />
      <div className="floor-filters" role="group" aria-label="Work order status">
        {STATUS_FILTERS.map((filter) => {
          const pressed = status === filter.value;
          return (
            <button
              key={filter.value || "all"}
              type="button"
              className={
                pressed ? "floor-filter floor-filter--active" : "floor-filter"
              }
              aria-pressed={pressed}
              onClick={() => setStatus(filter.value)}
            >
              {filter.label}
            </button>
          );
        })}
      </div>
      <WorkOrderResults
        isPending={listQuery.isPending}
        isError={listQuery.isError}
        errorMessage={
          listQuery.error instanceof Error
            ? listQuery.error.message
            : "Work order list request failed."
        }
        workOrders={workOrders}
        itemsById={itemsById}
        hasNextPage={Boolean(listQuery.hasNextPage)}
        isFetchingNextPage={listQuery.isFetchingNextPage}
        onMore={() => {
          void listQuery.fetchNextPage();
        }}
      />
    </div>
  );
}

function WorkOrderResults({
  isPending,
  isError,
  errorMessage,
  workOrders,
  itemsById,
  hasNextPage,
  isFetchingNextPage,
  onMore,
}: {
  isPending: boolean;
  isError: boolean;
  errorMessage: string;
  workOrders: WorkOrderView[];
  itemsById: Map<string, ItemMasterView>;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onMore: () => void;
}) {
  if (isPending) {
    return (
      <p className="floor-pick-status" role="status">
        Loading work orders.
      </p>
    );
  }
  if (isError) {
    return (
      <p className="floor-pick-status floor-pick-status--error" role="alert">
        {errorMessage}
      </p>
    );
  }
  if (workOrders.length === 0) {
    return (
      <p className="floor-pick-status" role="status">
        No work orders returned.
      </p>
    );
  }
  return (
    <div className="floor-pick-list">
      {workOrders.map((workOrder) => (
        <WorkOrderPick
          key={workOrder.id}
          workOrder={workOrder}
          item={itemsById.get(workOrder.itemId)}
        />
      ))}
      {hasNextPage ? (
        <button
          type="button"
          className="floor-more"
          onClick={onMore}
          disabled={isFetchingNextPage}
        >
          {isFetchingNextPage ? "LOADING MORE" : "MORE"}
        </button>
      ) : null}
    </div>
  );
}

function WorkOrderPick({
  workOrder,
  item,
}: {
  workOrder: WorkOrderView;
  item: ItemMasterView | undefined;
}) {
  const number = workOrder.number ?? "UNNUMBERED";
  return (
    <Link
      to="/floor/work-orders/$workOrderId"
      params={{ workOrderId: workOrder.id }}
      className="floor-pick"
    >
      <span className="floor-pick__number mono">{number}</span>
      <StatusMark status={workOrder.status} label={workOrder.statusLabel} />
      {item ? <span className="floor-pick__item mono">{item.number}</span> : null}
      <span className="floor-pick__qty">
        <span className="floor-pick__qty-label">ORDERED</span>
        <span className="floor-pick__qty-amount mono">
          {workOrder.quantityAmount}
        </span>
      </span>
    </Link>
  );
}

function StatusMark({ status, label }: { status: string; label: string }) {
  const shapeClass = STATUS_SHAPE_CLASS[status] ?? "floor-status-mark__shape";
  return (
    <span className="floor-status-mark">
      <span className={shapeClass} aria-hidden="true" />
      <span className="floor-status-mark__word">{label}</span>
    </span>
  );
}

const STATUS_SHAPE_CLASS: Record<string, string> = {
  draft: "floor-status-mark__shape floor-status-mark__shape--circle",
  released: "floor-status-mark__shape floor-status-mark__shape--square",
  in_process: "floor-status-mark__shape floor-status-mark__shape--triangle",
  completed: "floor-status-mark__shape floor-status-mark__shape--diamond",
  cancelled: "floor-status-mark__shape floor-status-mark__shape--octagon",
};

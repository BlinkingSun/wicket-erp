import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { useState, type KeyboardEvent } from "react";
import { listItems } from "../../api/client";
import "./items.css";

const PAGE_LIMIT = 50;

export function ItemList() {
  const navigate = useNavigate();
  const [prefix, setPrefix] = useState("");
  const trimmedPrefix = prefix.trim();

  const query = useInfiniteQuery({
    queryKey: ["items", "listItems", trimmedPrefix],
    initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam }) => {
      const args: {
        limit: number;
        cursor?: string;
        numberPrefix?: string;
      } = { limit: PAGE_LIMIT };
      if (pageParam) {
        args.cursor = pageParam;
      }
      if (trimmedPrefix) {
        args.numberPrefix = trimmedPrefix;
      }
      return listItems(args);
    },
    getNextPageParam: (lastPage) =>
      lastPage.hasMore && lastPage.nextCursor ? lastPage.nextCursor : undefined,
  });

  function openItem(itemId: string): void {
    void navigate({ to: "/office/items/$itemId", params: { itemId } });
  }

  function onRowKeyDown(
    event: KeyboardEvent<HTMLTableRowElement>,
    itemId: string,
  ): void {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openItem(itemId);
    }
  }

  const rows = query.data?.pages.flatMap((page) => page.rows) ?? [];
  const errorMessage =
    query.error instanceof Error
      ? query.error.message
      : "Item list request failed.";

  return (
    <section className="item-list" aria-label="Items">
      <header className="item-list__toolbar">
        <h2 className="item-list__title">Items</h2>
        <div className="item-list__search">
          <label htmlFor="item-list-prefix">Item number prefix</label>
          <input
            id="item-list-prefix"
            name="number_prefix"
            type="search"
            value={prefix}
            onChange={(event) => setPrefix(event.target.value)}
            autoComplete="off"
            spellCheck={false}
          />
        </div>
      </header>
      {query.isPending ? (
        <p className="trace-status" role="status">
          Loading items.
        </p>
      ) : query.isError && rows.length === 0 ? (
        <p className="query-error" role="alert">
          {errorMessage}
        </p>
      ) : rows.length === 0 ? (
        <div className="item-empty item-list__empty" role="status">
          <p className="item-empty__title">
            {trimmedPrefix ? "No matching item" : "Empty list"}
          </p>
          <p className="item-empty__detail">
            {trimmedPrefix
              ? `No item matched the prefix ${trimmedPrefix}.`
              : "The engine returned an empty page."}
          </p>
        </div>
      ) : (
        <>
          <div className="item-list__table-wrap">
            <table className="item-data-table item-list-table">
              <thead>
                <tr>
                  <th scope="col">Item number</th>
                  <th scope="col">Description</th>
                  <th scope="col">Kind</th>
                  <th scope="col">Status</th>
                  <th scope="col">Revision</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr
                    key={row.id}
                    className="item-list__row"
                    tabIndex={0}
                    onClick={() => openItem(row.id)}
                    onKeyDown={(event) => onRowKeyDown(event, row.id)}
                  >
                    <td className="mono">{row.number}</td>
                    <td>{row.description}</td>
                    <td>{row.kindLabel}</td>
                    <td>
                      <span className="status-pill">{row.statusLabel}</span>
                    </td>
                    <td className="mono">{row.revision}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {query.isFetchNextPageError ? (
            <p className="query-error" role="alert">
              {errorMessage}
            </p>
          ) : null}
          {query.hasNextPage ? (
            <button
              type="button"
              className="item-list__more"
              onClick={() => void query.fetchNextPage()}
              disabled={query.isFetchingNextPage}
            >
              {query.isFetchingNextPage ? "Loading more items." : "Load more"}
            </button>
          ) : null}
        </>
      )}
    </section>
  );
}

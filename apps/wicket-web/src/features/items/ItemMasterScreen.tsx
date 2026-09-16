import { useQuery } from "@tanstack/react-query";
import { getItem } from "../../api/client";
import { ItemMasterTabs } from "./ItemMasterTabs";
import { RevisionStrip } from "./ItemMasterPanels";
import "./items.css";

type ItemMasterScreenProps = {
  itemId: string;
};

const SAVE_DISABLED_TITLE =
  "No fields on this screen are editable yet. Save will apply when item fields can be edited here.";

export function ItemMasterScreen({ itemId }: ItemMasterScreenProps) {
  const query = useQuery({
    queryKey: ["items", "getItem", itemId],
    queryFn: () => getItem(itemId),
  });

  if (query.isPending) {
    return <p className="trace-status">Loading item master.</p>;
  }
  if (query.isError) {
    return (
      <p className="query-error">
        {query.error instanceof Error
          ? query.error.message
          : "Item master request failed."}
      </p>
    );
  }
  if (!query.data) {
    return null;
  }

  const item = query.data;

  return (
    <article className="item-master">
      <header className="item-master__header">
        <div className="item-master__title-row">
          <div className="item-master__title-block">
            <h2 className="item-master__number">{item.number}</h2>
            <span className="item-master__revision">Rev {item.revision}</span>
            <span className="status-pill">{item.statusLabel}</span>
          </div>
          <button
            type="button"
            className="item-master__save"
            disabled
            title={SAVE_DISABLED_TITLE}
          >
            Save
          </button>
        </div>
        <p className="item-master__subline">
          {item.kindLabel} • {item.stockUomLabel}
        </p>
        <hr className="item-master__rule" />
        <p className="item-master__description">{item.description}</p>
      </header>
      <RevisionStrip currentRevision={item.revision} />
      <ItemMasterTabs item={item} />
    </article>
  );
}

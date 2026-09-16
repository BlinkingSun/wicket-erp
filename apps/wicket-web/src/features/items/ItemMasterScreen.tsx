import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useId, useState } from "react";
import { getItem, updateItem } from "../../api/client";
import { ItemMasterTabs } from "./ItemMasterTabs";
import { RevisionStrip } from "./ItemMasterPanels";
import "./items.css";

type ItemMasterScreenProps = {
  itemId: string;
};

type DescriptionDraft = {
  itemId: string;
  text: string;
};

export function ItemMasterScreen({ itemId }: ItemMasterScreenProps) {
  const queryClient = useQueryClient();
  const descriptionFieldId = useId();

  const query = useQuery({
    queryKey: ["items", "getItem", itemId],
    queryFn: () => getItem(itemId),
  });

  const [descriptionDraft, setDescriptionDraft] = useState<DescriptionDraft | null>(
    null,
  );

  const savedItem = query.data;
  const draftDescription =
    descriptionDraft?.itemId === itemId
      ? descriptionDraft.text
      : (savedItem?.description ?? "");

  const saveMutation = useMutation({
    mutationFn: async () => {
      if (!savedItem) {
        throw new Error("Item is not loaded.");
      }
      return updateItem({
        itemId,
        patch: { description: draftDescription },
        version: savedItem.version,
        idempotencyKey: crypto.randomUUID(),
      });
    },
    onSuccess: (updated) => {
      queryClient.setQueryData(["items", "getItem", itemId], updated);
      setDescriptionDraft(null);
    },
  });

  const onSave = useCallback(() => {
    saveMutation.mutate();
  }, [saveMutation]);

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
  const dirty = draftDescription !== item.description;

  const canSave = dirty && !saveMutation.isPending;

  return (
    <article className="item-master">
      <header className="item-master__header">
        <div className="item-master__title-row">
          <div className="item-master__title-block">
            <h2 className="item-master__number">{item.number}</h2>
            <span className="item-master__revision">Rev {item.revision}</span>
            <span className="item-master__status-pill">{item.statusLabel}</span>
          </div>
          <button
            type="button"
            className="item-master__save"
            disabled={!canSave}
            onClick={onSave}
          >
            Save
          </button>
        </div>
        <p className="item-master__subline">
          {item.kindLabel} • {item.stockUomLabel}
        </p>
        <hr className="item-master__rule" />
        <label className="item-master__description-label" htmlFor={descriptionFieldId}>
          Description
        </label>
        <textarea
          id={descriptionFieldId}
          className="item-master__description"
          value={draftDescription}
          rows={2}
          onChange={(event) => {
            setDescriptionDraft({ itemId, text: event.target.value });
          }}
        />
        {saveMutation.isError ? (
          <p className="query-error">
            {saveMutation.error instanceof Error
              ? saveMutation.error.message
              : "Save failed."}
          </p>
        ) : null}
      </header>
      <RevisionStrip />
      <ItemMasterTabs item={item} />
    </article>
  );
}

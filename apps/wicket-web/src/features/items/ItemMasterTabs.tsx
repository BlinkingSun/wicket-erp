import { useState } from "react";
import type { ItemMasterTabId, ItemMasterView } from "../../api/view-models";
import { UNBACKED_TAB_CAPABILITIES } from "../../api/view-models";
import { BomTabPanel, InventoryTabPanel } from "./ItemMasterPanels";
import { NotYetAvailable } from "./NotYetAvailable";

const TAB_LABELS: Record<ItemMasterTabId, string> = {
  bom: "Bill of Material",
  routing: "Routing",
  inventory: "Inventory",
  documents: "Documents",
  "where-used": "Where Used",
};

type ItemMasterTabsProps = {
  item: ItemMasterView;
};

export function ItemMasterTabs({ item }: ItemMasterTabsProps) {
  const [active, setActive] = useState<ItemMasterTabId>("bom");

  return (
    <div className="item-tabs">
      <div className="item-tabs__bar" role="tablist" aria-label="Item master sections">
        {(
          Object.entries(TAB_LABELS) as [ItemMasterTabId, string][]
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            id={`item-tab-${id}`}
            aria-selected={active === id}
            aria-controls={`item-tabpanel-${id}`}
            className={
              active === id ? "item-tabs__tab item-tabs__tab--active" : "item-tabs__tab"
            }
            onClick={() => {
              setActive(id);
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        className="item-tabs__panel"
        role="tabpanel"
        id={`item-tabpanel-${active}`}
        aria-labelledby={`item-tab-${active}`}
      >
        <TabPanel active={active} item={item} />
      </div>
    </div>
  );
}

function TabPanel({
  active,
  item,
}: {
  active: ItemMasterTabId;
  item: ItemMasterView;
}) {
  if (active === "bom") {
    return <BomTabPanel />;
  }
  if (active === "inventory") {
    return <InventoryTabPanel itemId={item.id} />;
  }
  const capability = UNBACKED_TAB_CAPABILITIES[active];
  return (
    <section className="item-tab-panel" aria-label={capability.label}>
      <NotYetAvailable capability={capability} />
    </section>
  );
}

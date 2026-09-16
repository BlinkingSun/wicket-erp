import type {
  GenealogyBothTraceView,
  GenealogyTraceView,
  TraceQuantityView,
  TraceTreeNodeView,
} from "../../api/view-models";
import { truncateUuid } from "./truncateUuid";
import "./genealogy.css";

type GenealogyTraceProps = {
  trace: GenealogyTraceView | GenealogyBothTraceView;
};

export function GenealogyTrace({ trace }: GenealogyTraceProps) {
  if (trace.kind === "both") {
    return <GenealogyBothTrace trace={trace} />;
  }
  return (
    <section aria-label="Genealogy trace">
      <DirectionTrace direction={trace.direction} nodes={trace.nodes} />
    </section>
  );
}

function GenealogyBothTrace({ trace }: { trace: GenealogyBothTraceView }) {
  return (
    <div className="genealogy-trace">
      <section aria-label="Genealogy trace backward">
        <h3 className="genealogy-trace__heading">Backward</h3>
        <DirectionTrace direction={trace.backward.direction} nodes={trace.backward.nodes} />
      </section>
      <section aria-label="Genealogy trace forward">
        <h3 className="genealogy-trace__heading">Forward</h3>
        <DirectionTrace direction={trace.forward.direction} nodes={trace.forward.nodes} />
      </section>
    </div>
  );
}

function DirectionTrace({
  direction,
  nodes,
}: {
  direction: "forward" | "backward";
  nodes: TraceTreeNodeView[];
}) {
  return (
    <>
      <p className="trace-status">Direction {direction}</p>
      <div className="genealogy-trace__forest">
        {nodes.map((node) => (
          <TraceTreeNode key={`${node.lotId}-${node.posting}`} node={node} />
        ))}
      </div>
    </>
  );
}

function TraceTreeNode({ node }: { node: TraceTreeNodeView }) {
  return (
    <div className="trace-tree">
      <article className="node-card" data-lot-id={node.lotId} data-posting={node.posting}>
        <LotId lotId={node.lotId} />
        <NodeMeta node={node} />
        {node.children.map((child) => (
          <div key={`${child.lotId}-${child.posting}`} className="trace-tree__child-branch">
            <div className="trace-tree__edge-qty">
              Edge {formatQuantityLabel(child.edgeQuantity)}
            </div>
            <TraceTreeNode node={child} />
          </div>
        ))}
      </article>
    </div>
  );
}

function NodeMeta({ node }: { node: TraceTreeNodeView }) {
  return (
    <>
      <div className="node-card__meta mono">
        On hand {formatQuantityLabel(node.quantity)}
      </div>
      <div className="node-card__meta mono">
        Amount {node.amount} (currency {node.amountCurrency})
      </div>
      {node.itemId ? (
        <div className="node-card__meta">
          Item <RefId id={node.itemId} label="Item UUID" />
        </div>
      ) : null}
      {node.serial ? <div className="node-card__serials">Serial {node.serial}</div> : null}
      {node.locationId ? (
        <div className="node-card__meta">
          Location <RefId id={node.locationId} label="Location UUID" />
        </div>
      ) : null}
      {node.occurredAt ? (
        <div className="node-card__meta">
          <time dateTime={node.occurredAt}>{node.occurredAt}</time>
        </div>
      ) : null}
    </>
  );
}

function formatQuantityLabel(quantity: TraceQuantityView): string {
  const trimmed = quantity.amount
    .replace(/(\.\d*?)0+$/, "$1")
    .replace(/\.$/, "");
  return `${trimmed} ${quantity.dimension}`;
}

function LotId({ lotId }: { lotId: string }) {
  return (
    <abbr className="lot-id" title={lotId}>
      {truncateUuid(lotId)}
    </abbr>
  );
}

function RefId({ id, label }: { id: string; label: string }) {
  return (
    <abbr className="ref-id" title={id} aria-label={label}>
      {truncateUuid(id)}
    </abbr>
  );
}

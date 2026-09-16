import type { GenealogyTraceView, TraceEdgeView, TraceNodeView } from "../../api/view-models";

type GenealogyTraceProps = {
  trace: GenealogyTraceView;
};

export function GenealogyTrace({ trace }: GenealogyTraceProps) {
  const layers = layoutLayers(trace);
  return (
    <section aria-label="Genealogy trace">
      <p className="trace-status">
        Root {trace.root.identifier}
        <span className="mono"> {trace.root.lotId}</span>
      </p>
      <div className="trace-board">
        {layers.map((layer, index) => (
          <LayerBlock
            key={`layer-${index}`}
            nodes={layer}
            rootId={trace.root.lotId}
            outgoing={edgesFromLayer(layer, trace.edges)}
            showConnectors={index < layers.length - 1}
          />
        ))}
      </div>
    </section>
  );
}

function LayerBlock({
  nodes,
  rootId,
  outgoing,
  showConnectors,
}: {
  nodes: TraceNodeView[];
  rootId: string;
  outgoing: TraceEdgeView[];
  showConnectors: boolean;
}) {
  return (
    <>
      <div className="trace-layer">
        {nodes.map((node) => (
          <article
            key={node.lotId}
            className={node.lotId === rootId ? "node-card node-card--root" : "node-card"}
            data-lot-id={node.lotId}
          >
            {node.lotId === rootId ? <span className="status-pill">Root</span> : null}
            <div className="node-card__id">{node.identifier}</div>
            {node.quantityLabel ? (
              <div className="node-card__meta mono">{node.quantityLabel}</div>
            ) : null}
            {node.serials.length > 0 ? (
              <div className="node-card__serials">{node.serials.join(", ")}</div>
            ) : null}
          </article>
        ))}
      </div>
      {showConnectors ? (
        <div className="trace-connectors" aria-label="Trace edges">
          {outgoing.length === 0 ? (
            <div className="edge-label">
              <span className="edge-label__rule" />
            </div>
          ) : (
            outgoing.map((edge) => (
              <div
                key={`${edge.fromLotId}-${edge.toLotId}-${edge.kind}`}
                className="edge-label"
                data-edge-kind={edge.kind}
              >
                <span className="edge-label__rule" />
                <span>{edge.kind}</span>
                {edge.workOrder ? (
                  <span className="edge-label__wo">{edge.workOrder}</span>
                ) : null}
                <span className="edge-label__rule" />
              </div>
            ))
          )}
        </div>
      ) : null}
    </>
  );
}

function layoutLayers(trace: GenealogyTraceView): TraceNodeView[][] {
  const outgoing = new Map<string, string[]>();
  for (const node of trace.nodes) {
    outgoing.set(node.lotId, []);
  }
  for (const edge of trace.edges) {
    const list = outgoing.get(edge.fromLotId);
    if (list) {
      list.push(edge.toLotId);
    }
  }

  const depth = new Map<string, number>();
  const queue = [trace.root.lotId];
  depth.set(trace.root.lotId, 0);
  while (queue.length > 0) {
    const id = queue.shift();
    if (id === undefined) {
      break;
    }
    const current = depth.get(id) ?? 0;
    for (const next of outgoing.get(id) ?? []) {
      if (!depth.has(next)) {
        depth.set(next, current + 1);
        queue.push(next);
      }
    }
  }

  const byId = new Map(trace.nodes.map((node) => [node.lotId, node]));
  const layers: TraceNodeView[][] = [];
  for (const node of trace.nodes) {
    const layerIndex = depth.get(node.lotId) ?? 0;
    while (layers.length <= layerIndex) {
      layers.push([]);
    }
    layers[layerIndex]?.push(node);
  }

  if (layers.length === 0) {
    const rootNode = byId.get(trace.root.lotId);
    if (rootNode) {
      return [[rootNode]];
    }
  }
  return layers.filter((layer) => layer.length > 0);
}

function edgesFromLayer(layer: TraceNodeView[], edges: TraceEdgeView[]): TraceEdgeView[] {
  const ids = new Set(layer.map((node) => node.lotId));
  return edges.filter((edge) => ids.has(edge.fromLotId));
}

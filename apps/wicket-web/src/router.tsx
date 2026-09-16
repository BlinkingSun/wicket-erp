import {
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";
import type { TraceQueryDirection } from "./api/view-models";
import { ShopFloorTerminal } from "./features/floor/ShopFloorTerminal";
import { GenealogyScreen } from "./features/genealogy/GenealogyScreen";
import { ItemMasterScreen } from "./features/items/ItemMasterScreen";
import { FloorHome, FloorLayout } from "./modes/floor-layout";
import { OfficeHome, OfficeLayout } from "./modes/office-layout";
import { QualityLayout } from "./modes/quality-layout";
import { RootLayout } from "./modes/root-layout";

const rootRoute = createRootRoute({
  component: RootLayout,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/quality/genealogy" });
  },
});

const officeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/office",
  component: OfficeLayout,
});

const officeIndexRoute = createRoute({
  getParentRoute: () => officeRoute,
  path: "/",
  component: OfficeHome,
});

const officeItemsRoute = createRoute({
  getParentRoute: () => officeRoute,
  path: "items",
});

const officeItemsIndexRoute = createRoute({
  getParentRoute: () => officeItemsRoute,
  path: "/",
  component: OfficeHome,
});

const officeItemMasterRoute = createRoute({
  getParentRoute: () => officeItemsRoute,
  path: "$itemId",
  component: function ItemMasterRoute() {
    const { itemId } = officeItemMasterRoute.useParams();
    return <ItemMasterScreen itemId={itemId} />;
  },
});

const floorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/floor",
  component: FloorLayout,
});

const floorIndexRoute = createRoute({
  getParentRoute: () => floorRoute,
  path: "/",
  component: FloorHome,
});

const floorWorkOrdersRoute = createRoute({
  getParentRoute: () => floorRoute,
  path: "work-orders",
});

const floorWorkOrderRoute = createRoute({
  getParentRoute: () => floorWorkOrdersRoute,
  path: "$workOrderId",
  component: function FloorWorkOrderRoute() {
    const { workOrderId } = floorWorkOrderRoute.useParams();
    return <ShopFloorTerminal workOrderId={workOrderId} />;
  },
});

const qualityRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/quality",
  component: QualityLayout,
});

const qualityIndexRoute = createRoute({
  getParentRoute: () => qualityRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/quality/genealogy" });
  },
});

type GenealogySearch = {
  from_lot_id?: string;
  direction?: TraceQueryDirection;
};

function parseTraceDirection(value: unknown): TraceQueryDirection | undefined {
  if (value === "backward" || value === "forward" || value === "both") {
    return value;
  }
  return undefined;
}

const genealogyRoute = createRoute({
  getParentRoute: () => qualityRoute,
  path: "genealogy",
  validateSearch: (search: Record<string, unknown>): GenealogySearch => ({
    from_lot_id:
      typeof search.from_lot_id === "string" ? search.from_lot_id : undefined,
    direction: parseTraceDirection(search.direction),
  }),
  component: function GenealogyRoute() {
    const { from_lot_id, direction } = genealogyRoute.useSearch();
    return (
      <GenealogyScreen
        fromLotId={from_lot_id}
        direction={direction ?? "forward"}
      />
    );
  },
});

export const routeTree = rootRoute.addChildren([
  indexRoute,
  officeRoute.addChildren([
    officeIndexRoute,
    officeItemsRoute.addChildren([officeItemsIndexRoute, officeItemMasterRoute]),
  ]),
  floorRoute.addChildren([
    floorIndexRoute,
    floorWorkOrdersRoute.addChildren([floorWorkOrderRoute]),
  ]),
  qualityRoute.addChildren([qualityIndexRoute, genealogyRoute]),
]);

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

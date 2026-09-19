import {
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";
import type { TraceQueryDirection } from "./api/view-models";
import { LoginScreen } from "./features/auth/LoginScreen";
import { RootWithSession } from "./features/auth/RootWithSession";
import { hasSession, hydrateSessionFromCookie } from "./features/auth/session";
import { WorkOrderList } from "./features/floor/WorkOrderList";
import { ShopFloorTerminal } from "./features/floor/ShopFloorTerminal";
import { LotPicker } from "./features/genealogy/LotPicker";
import { GenealogyScreen } from "./features/genealogy/GenealogyScreen";
import { ItemList } from "./features/items/ItemList";
import { ItemMasterScreen } from "./features/items/ItemMasterScreen";
import { FloorHome, FloorLayout } from "./modes/floor-layout";
import { OfficeHome, OfficeLayout } from "./modes/office-layout";
import { QualityLayout } from "./modes/quality-layout";

function requireSession(location: {
  pathname: string;
  searchStr: string;
}): void {
  hydrateSessionFromCookie();
  if (hasSession()) {
    return;
  }
  throw redirect({
    to: "/login",
    search: { next: `${location.pathname}${location.searchStr}` },
  });
}

const rootRoute = createRootRoute({
  component: RootWithSession,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/quality/genealogy" });
  },
});

type LoginSearch = {
  next?: string;
};

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  validateSearch: (search: Record<string, unknown>): LoginSearch => ({
    next: typeof search.next === "string" ? search.next : undefined,
  }),
  component: function LoginRoute() {
    const { next } = loginRoute.useSearch();
    return <LoginScreen next={next} />;
  },
});

const officeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/office",
  beforeLoad: ({ location }) => {
    requireSession(location);
  },
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
  component: ItemList,
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
  beforeLoad: ({ location }) => {
    requireSession(location);
  },
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

const floorWorkOrdersIndexRoute = createRoute({
  getParentRoute: () => floorWorkOrdersRoute,
  path: "/",
  component: WorkOrderList,
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
  beforeLoad: ({ location }) => {
    requireSession(location);
  },
  component: QualityLayout,
});

const qualityIndexRoute = createRoute({
  getParentRoute: () => qualityRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/quality/genealogy" });
  },
});

const qualityLotsRoute = createRoute({
  getParentRoute: () => qualityRoute,
  path: "lots",
  component: LotPicker,
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
  loginRoute,
  officeRoute.addChildren([
    officeIndexRoute,
    officeItemsRoute.addChildren([officeItemsIndexRoute, officeItemMasterRoute]),
  ]),
  floorRoute.addChildren([
    floorIndexRoute,
    floorWorkOrdersRoute.addChildren([
      floorWorkOrdersIndexRoute,
      floorWorkOrderRoute,
    ]),
  ]),
  qualityRoute.addChildren([qualityIndexRoute, qualityLotsRoute, genealogyRoute]),
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

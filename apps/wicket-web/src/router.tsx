import {
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from "@tanstack/react-router";
import { GenealogyScreen } from "./features/genealogy/GenealogyScreen";
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
  direction?: "forward" | "backward";
};

const genealogyRoute = createRoute({
  getParentRoute: () => qualityRoute,
  path: "genealogy",
  validateSearch: (search: Record<string, unknown>): GenealogySearch => ({
    from_lot_id:
      typeof search.from_lot_id === "string" ? search.from_lot_id : undefined,
    direction:
      search.direction === "backward"
        ? "backward"
        : search.direction === "forward"
          ? "forward"
          : undefined,
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
  officeRoute.addChildren([officeIndexRoute]),
  floorRoute.addChildren([floorIndexRoute]),
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

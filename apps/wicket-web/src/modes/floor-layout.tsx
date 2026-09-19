import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { ShopFloorTerminal } from "../features/floor/ShopFloorTerminal";
import { ModeSwitch } from "./mode-switch";
import { SessionChip } from "./session-chip";

const JOB_PATH = /^\/floor\/work-orders\/[^/]+$/;

export function FloorLayout() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const onJob = JOB_PATH.test(pathname);

  return (
    <div className="floor-shell">
      <div className="floor-chrome">
        <ModeSwitch variant="floor" />
        <SessionChip size="floor" />
      </div>
      {onJob ? (
        <Link
          to="/floor/work-orders"
          className="floor-chrome-back"
          aria-label="Back to work orders"
          activeOptions={{ exact: true }}
        >
          BACK
        </Link>
      ) : null}
      <div className="floor-main">
        <Outlet />
      </div>
    </div>
  );
}

export function FloorHome() {
  return <ShopFloorTerminal />;
}

import { Outlet } from "@tanstack/react-router";
import { ShopFloorTerminal } from "../features/floor/ShopFloorTerminal";

export function FloorLayout() {
  return (
    <div className="floor-shell">
      <Outlet />
    </div>
  );
}

export function FloorHome() {
  return <ShopFloorTerminal />;
}

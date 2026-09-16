import { Outlet } from "@tanstack/react-router";
import { ModeSwitch } from "./mode-switch";

export function FloorLayout() {
  return (
    <div className="floor-shell">
      <header>
        <h1>Wicket</h1>
        <ModeSwitch />
      </header>
      <main>
        <Outlet />
      </main>
    </div>
  );
}

export function FloorHome() {
  return (
    <p className="shell-copy">
      Shop floor mode has no screens in this wave. The scan box does not work;
      human-identifier lookup does not exist.
    </p>
  );
}

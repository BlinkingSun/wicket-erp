import { Outlet } from "@tanstack/react-router";
import { ModeSwitch } from "./mode-switch";

export function OfficeLayout() {
  return (
    <div className="office-shell">
      <aside className="office-rail">
        <h1>Wicket</h1>
        <ModeSwitch />
        <p className="shell-copy">Office mode has no screens in this wave.</p>
      </aside>
      <main className="office-main">
        <Outlet />
      </main>
    </div>
  );
}

export function OfficeHome() {
  return (
    <p className="shell-copy">
      The office tree is an empty shell. Item master and the office data grid
      are not in this wave.
    </p>
  );
}

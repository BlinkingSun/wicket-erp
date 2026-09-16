import { Outlet } from "@tanstack/react-router";
import { ModeSwitch } from "./mode-switch";

export function QualityLayout() {
  return (
    <div className="quality-shell">
      <header className="quality-header">
        <div className="quality-brand">
          <h1>Wicket</h1>
          <p>Lot-and-Serial Genealogy Trace | Recall</p>
          <ModeSwitch />
        </div>
      </header>
      <main className="quality-main">
        <Outlet />
      </main>
    </div>
  );
}

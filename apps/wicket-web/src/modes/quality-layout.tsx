import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { ModeSwitch } from "./mode-switch";
import { SessionChip } from "./session-chip";

const QUALITY_MODULES = [
  { label: "Genealogy", to: "/quality/genealogy" as const },
  { label: "Lots", to: "/quality/lots" as const },
] as const;

export function QualityLayout() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  return (
    <div className="quality-shell">
      <aside className="mode-rail">
        <h1>Wicket</h1>
        <ModeSwitch />
        <nav aria-label="Quality modules">
          <ul className="mode-nav">
            {QUALITY_MODULES.map((entry) => {
              const active = pathname.startsWith(entry.to);
              return (
                <li key={entry.label}>
                  <Link to={entry.to} aria-current={active ? "page" : undefined}>
                    {entry.label}
                  </Link>
                </li>
              );
            })}
          </ul>
        </nav>
        <SessionChip />
      </aside>
      <div className="quality-body">
        <header className="quality-header">
          <div className="quality-brand">
            <p>Lot-and-Serial Genealogy Trace | Recall</p>
          </div>
        </header>
        <main className="quality-main">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

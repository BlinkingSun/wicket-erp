import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import "../features/items/items.css";
import { ModeSwitch } from "./mode-switch";

const OFFICE_MODULES = [
  { label: "Items", to: "/office/items" as const, enabled: true },
  { label: "Inventory", enabled: false },
  { label: "Production", enabled: false },
  { label: "Purchasing", enabled: false },
  { label: "Sales", enabled: false },
  { label: "Quality", enabled: false },
  { label: "Planning", enabled: false },
] as const;

export function OfficeLayout() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  return (
    <div className="office-shell">
      <aside className="office-rail">
        <h1>Wicket</h1>
        <ModeSwitch />
        <nav aria-label="Office modules">
          <ul className="office-nav">
            {OFFICE_MODULES.map((entry) => {
              if (entry.enabled && "to" in entry) {
                const active = pathname.startsWith(entry.to);
                return (
                  <li key={entry.label}>
                    <Link to={entry.to} aria-current={active ? "page" : undefined}>
                      {entry.label}
                    </Link>
                  </li>
                );
              }
              return (
                <li key={entry.label}>
                  <span className="office-nav__disabled" aria-disabled="true">
                    {entry.label}
                  </span>
                </li>
              );
            })}
          </ul>
        </nav>
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
      Open an item from the API using{" "}
      <span className="mono">/office/items/&lt;item-id&gt;</span>. Item master reads
      getItem and updateItem only.
    </p>
  );
}

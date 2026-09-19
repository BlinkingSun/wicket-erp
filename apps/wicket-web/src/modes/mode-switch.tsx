import { Link, useRouterState } from "@tanstack/react-router";
import { useState } from "react";

type ModeSwitchProps = {
  variant?: "rail" | "floor";
};

const DESTINATIONS = [
  { label: "Office", to: "/office" as const, prefix: "/office" },
  { label: "Floor", to: "/floor/work-orders" as const, prefix: "/floor" },
  { label: "Quality", to: "/quality/genealogy" as const, prefix: "/quality" },
] as const;

function isModeActive(pathname: string, prefix: string): boolean {
  return pathname === prefix || pathname.startsWith(`${prefix}/`);
}

export function ModeSwitch({ variant = "rail" }: ModeSwitchProps) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const [open, setOpen] = useState(false);

  const links = DESTINATIONS.map((entry) => {
    const active = isModeActive(pathname, entry.prefix);
    return (
      <Link
        key={entry.label}
        to={entry.to}
        aria-current={active ? "page" : undefined}
        onClick={() => {
          setOpen(false);
        }}
      >
        {entry.label}
      </Link>
    );
  });

  if (variant === "floor") {
    return (
      <nav className="floor-mode-switch" aria-label="Interaction modes">
        <button
          type="button"
          className="floor-mode-switch__toggle"
          aria-expanded={open}
          aria-controls="floor-mode-switch-targets"
          onClick={() => {
            setOpen((current) => !current);
          }}
        >
          Mode
        </button>
        {open ? (
          <div id="floor-mode-switch-targets" className="floor-mode-switch__targets">
            {links}
          </div>
        ) : null}
      </nav>
    );
  }

  return (
    <nav className="mode-switch" aria-label="Interaction modes">
      {links}
    </nav>
  );
}

import { Link } from "@tanstack/react-router";

export function ModeSwitch() {
  return (
    <nav className="mode-switch" aria-label="Interaction modes">
      <Link to="/office" activeOptions={{ exact: false }}>
        Office
      </Link>
      <Link to="/floor" activeOptions={{ exact: false }}>
        Floor
      </Link>
      <Link to="/quality/genealogy" activeOptions={{ exact: false }}>
        Quality
      </Link>
    </nav>
  );
}

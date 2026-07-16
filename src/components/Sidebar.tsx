import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { getAppSetting, getReleaseViewState } from "../lib/backend";
import { RELEASE_CATALOG } from "../lib/releaseHistory";

export interface NavItem {
  to: string;
  label: string;
}

export const NAV_ITEMS: NavItem[] = [
  { to: "/", label: "Mission Control" },
  { to: "/operations", label: "Operations" },
  { to: "/inventory", label: "Inventory" },
  { to: "/blueprints", label: "Blueprints" },
  { to: "/production", label: "Production" },
  { to: "/quartermaster", label: "Quartermaster" },
  { to: "/settings", label: "Settings" },
  { to: "/whats-new", label: "What's New" },
];

// Routes remain available while unfinished modules stay out of beta navigation.
export const ROUTE_ITEMS: NavItem[] = [
  ...NAV_ITEMS,
  { to: "/industry", label: "Industry" },
  { to: "/logistics", label: "Logistics" },
  { to: "/market", label: "Market Intelligence" },
  { to: "/planning", label: "Planning" },
  { to: "/intelligence", label: "Intelligence" },
  { to: "/reports", label: "Reports" },
];

function BrandGlyph() {
  return (
    <svg className="brand-glyph" viewBox="0 0 26 26" aria-hidden="true">
      <path d="M13 1 L24 7.5 L24 18.5 L13 25 L2 18.5 L2 7.5 Z" fill="none" stroke="var(--furnace)" strokeWidth="1.6" />
      <path d="M13 6 L19 9.5 L19 16.5 L13 20 L7 16.5 L7 9.5 Z" fill="var(--furnace)" opacity="0.25" />
      <line x1="13" y1="6" x2="13" y2="20" stroke="var(--furnace)" strokeWidth="1.2" />
    </svg>
  );
}

export function Sidebar() {
  const [releaseUnseen, setReleaseUnseen] = useState(false);
  useEffect(() => {
    const refresh = () => Promise.all([getReleaseViewState(), getAppSetting("release_notes_indicator_enabled")])
      .then(([state, enabled]) => setReleaseUnseen(state.unseen && enabled !== "false"))
      .catch(() => setReleaseUnseen(false));
    refresh();
    window.addEventListener("rendernorth-release-viewed", refresh);
    window.addEventListener("rendernorth-release-preference", refresh);
    return () => {
      window.removeEventListener("rendernorth-release-viewed", refresh);
      window.removeEventListener("rendernorth-release-preference", refresh);
    };
  }, []);
  return (
    <nav className="sidebar" aria-label="Modules">
      <div className="brand">
        <div className="brand-mark">
          <BrandGlyph />
          <div className="brand-name">
            RenderNorth<br />
            <span>Industrial</span>
          </div>
        </div>
        <div className="brand-sub">THE INDUSTRIAL OPERATING SYSTEM</div>
        <div className="brand-version">v{RELEASE_CATALOG.publicVersion} · Open Beta</div>
      </div>

      <div className="nav-group">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) => (isActive ? "nav-link active" : "nav-link")}
          >
            {item.label}
            {item.to === "/whats-new" && releaseUnseen && <span className="release-nav-badge">NEW</span>}
            <span className="nav-tick">◤</span>
          </NavLink>
        ))}
      </div>
    </nav>
  );
}

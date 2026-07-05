import { NavLink } from "react-router-dom";

export interface NavItem {
  to: string;
  label: string;
}

export const NAV_ITEMS: NavItem[] = [
  { to: "/", label: "Mission Control" },
  { to: "/targets", label: "Build Targets" },
  { to: "/inventory", label: "Inventory" },
  { to: "/production", label: "Production" },
  { to: "/industry", label: "Industry" },
  { to: "/logistics", label: "Logistics" },
  { to: "/market", label: "Market Intelligence" },
  { to: "/planning", label: "Planning" },
  { to: "/intelligence", label: "Intelligence" },
  { to: "/reports", label: "Reports" },
  { to: "/settings", label: "Settings" },
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
            <span className="nav-tick">◤</span>
          </NavLink>
        ))}
      </div>
    </nav>
  );
}

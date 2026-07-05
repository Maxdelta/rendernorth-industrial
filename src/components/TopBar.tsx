import { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { NAV_ITEMS } from "./Sidebar";

function titleFor(pathname: string): string {
  const hit = NAV_ITEMS.find((i) => i.to === pathname);
  return hit ? hit.label : "RenderNorth Industrial";
}

function eveClock(): string {
  // EVE runs on UTC.
  return new Date().toISOString().slice(11, 19) + " EVE";
}

export function TopBar() {
  const { pathname } = useLocation();
  const [clock, setClock] = useState(eveClock());

  useEffect(() => {
    const id = setInterval(() => setClock(eveClock()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <header className="topbar">
      <h1 className="topbar-title">{titleFor(pathname)}</h1>
      <span className="badge">Demo data</span>
      <span className="badge cyan">Sprint 002</span>
      <div className="topbar-clock">{clock}</div>
    </header>
  );
}

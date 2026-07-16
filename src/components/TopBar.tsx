import { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { ROUTE_ITEMS } from "./Sidebar";
import { SHOW_DEVELOPMENT_UI } from "../lib/runtimeMode";
import { setApplicationSectionTitle } from "../lib/backend";
import { UpdateNotification } from "./UpdateNotification";

function titleFor(pathname: string): string {
  const hit = ROUTE_ITEMS.find((i) => i.to === pathname);
  return hit ? hit.label : "Mission Control";
}

function eveClock(): string {
  // EVE runs on UTC.
  return new Date().toISOString().slice(11, 19) + " EVE";
}

export function TopBar() {
  const { pathname } = useLocation();
  const [clock, setClock] = useState(eveClock());
  const section = titleFor(pathname);

  useEffect(() => {
    void setApplicationSectionTitle(section).catch((error) => {
      console.error("failed to update application title", error);
    });
  }, [section]);

  useEffect(() => {
    const id = setInterval(() => setClock(eveClock()), 1000);
    return () => clearInterval(id);
  }, []);

  return (
    <header className="topbar">
      <h1 className="topbar-title">{section}</h1>
      {SHOW_DEVELOPMENT_UI && <>
        <span className="badge">Demo data</span>
        <span className="badge cyan">Sprint 002</span>
      </>}
      <UpdateNotification />
      <div className="topbar-clock">{clock}</div>
    </header>
  );
}

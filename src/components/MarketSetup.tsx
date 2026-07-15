import { useEffect, useState } from "react";
import { getMarketProfile, refreshMarketPrices, type MarketProfile } from "../lib/backend";
import { Panel } from "./Panel";

function age(timestamp: string | null): string {
  if (!timestamp) return "Never refreshed";
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(timestamp).getTime()) / 1000));
  if (seconds < 60) return `${seconds}s old`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m old`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h old`;
  return `${Math.floor(seconds / 86400)}d old`;
}

export function MarketSetup({ onReadyChange }: { onReadyChange?: (ready: boolean) => void }) {
  const [profile, setProfile] = useState<MarketProfile | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function load() {
    const next = await getMarketProfile(); setProfile(next); onReadyChange?.(Boolean(next.fetchedAt));
  }
  useEffect(() => { load().catch((cause) => setError(String(cause))); }, []);
  async function refresh() {
    setWorking(true); setError(null);
    try { await refreshMarketPrices(); await load(); }
    catch (cause) { setError(String(cause)); onReadyChange?.(false); }
    finally { setWorking(false); }
  }
  return <Panel title="Market Data" keel={profile?.fetchedAt ? (profile.stale ? "furnace" : "nominal") : "furnace"} className="dash-hero" headerRight={<button className="target-select enabled" onClick={refresh} disabled={working}>{working ? "Refreshing…" : "Refresh Market"}</button>}>
    <p className="ph-mission">Market state is explicit and local. RenderNorth prices from the selected cached market snapshot; it never hides whether that snapshot is current.</p>
    {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
    <div className="res-summary-grid">
      <div className="res-summary-cell"><div className="res-summary-label">Current market</div><div className="ops-field-value">{profile?.displayName ?? "Unavailable"}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Last refresh</div><div className="ops-field-value">{profile?.fetchedAt ? new Date(profile.fetchedAt).toLocaleString() : "Never"}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Age</div><div className={`ops-field-value ${profile?.stale ? "furnace" : "nominal"}`}>{age(profile?.fetchedAt ?? null)} · {profile?.stale ? "STALE" : "CURRENT"}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Orders / pages</div><div className="ops-field-value">{profile?.orderCount.toLocaleString() ?? 0} / {profile?.pageCount ?? 0}</div></div>
    </div>
  </Panel>;
}

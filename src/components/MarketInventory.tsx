import { useEffect, useMemo, useState } from "react";
import {
  getMarketProfile,
  getMarketQuote,
  formatVolume,
  refreshMarketPrices,
  searchInventoryMarket,
  searchMarketTypes,
  type InventorySearchInput,
  type InventorySearchResult,
  type MarketProfile,
  type MarketQuote,
  type TypeSearchResult,
  type WeightedFill,
} from "../lib/backend";
import { Panel } from "./Panel";

type SearchMode = "inventory" | "market";

const isk = (value: number | null) =>
  value == null ? "No market orders" : `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })} ISK`;

const fillValue = (fill: WeightedFill, unit = false) => {
  if (!fill.sufficient) {
    return fill.filled === 0
      ? "No market orders"
      : `Insufficient volume (${fill.filled.toLocaleString()} / ${fill.requested.toLocaleString()})`;
  }
  return isk(unit ? fill.unitPrice : fill.totalPrice);
};

function ProfileSummary({ profile }: { profile: MarketProfile }) {
  return <div className="data-source">Active market: {profile.displayName} · The Forge · station {profile.locationId} · {profile.stale ? "STALE" : "CURRENT"} · {profile.fetchedAt ? new Date(profile.fetchedAt).toLocaleString() : "Never refreshed"} · {profile.orderCount.toLocaleString()} orders / {profile.pageCount} pages</div>;
}

export function MarketInventory() {
  const [mode, setMode] = useState<SearchMode>("inventory");
  const [input, setInput] = useState<InventorySearchInput>({ positiveOnly: true, sort: "name" });
  const [inventoryResult, setInventoryResult] = useState<InventorySearchResult | null>(null);
  const [profile, setProfile] = useState<MarketProfile | null>(null);
  const [marketQuery, setMarketQuery] = useState("");
  const [typeResults, setTypeResults] = useState<TypeSearchResult[]>([]);
  const [typeSearched, setTypeSearched] = useState(false);
  const [selectedType, setSelectedType] = useState<TypeSearchResult | null>(null);
  const [quantity, setQuantity] = useState("1");
  const [marketQuote, setMarketQuote] = useState<MarketQuote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function loadInventory(next = input) {
    setLoading(true);
    setError(null);
    try {
      setInventoryResult(await searchInventoryMarket(next));
      setProfile(await getMarketProfile());
    } catch (cause) {
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }

  async function loadQuote(type = selectedType, requested = Number(quantity)) {
    if (!type || !Number.isSafeInteger(requested) || requested < 1) {
      setMarketQuote(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      setMarketQuote(await getMarketQuote(type.typeId, requested));
      setProfile(await getMarketProfile());
    } catch (cause) {
      setMarketQuote(null);
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (mode !== "inventory") return;
    const id = setTimeout(() => loadInventory(input), 150);
    return () => clearTimeout(id);
  }, [mode, input]);

  useEffect(() => {
    if (mode !== "market") return;
    const query = marketQuery.trim();
    if (!query) {
      setTypeResults([]);
      setTypeSearched(false);
      return;
    }
    let active = true;
    setTypeSearched(false);
    const id = setTimeout(() => {
      setLoading(true);
      setError(null);
      searchMarketTypes(query)
        .then((results) => active && setTypeResults(results))
        .catch((cause) => active && setError(`Static type search failed: ${String(cause)}`))
        .finally(() => {
          if (active) {
            setTypeSearched(true);
            setLoading(false);
          }
        });
    }, 150);
    return () => { active = false; clearTimeout(id); };
  }, [mode, marketQuery]);

  useEffect(() => {
    if (mode !== "market" || !selectedType) return;
    const requested = Number(quantity);
    if (!Number.isSafeInteger(requested) || requested < 1) {
      setMarketQuote(null);
      return;
    }
    const id = setTimeout(() => loadQuote(selectedType, requested), 150);
    return () => clearTimeout(id);
  }, [mode, selectedType, quantity]);

  useEffect(() => {
    getMarketProfile().then(setProfile).catch((cause) => setError(String(cause)));
  }, []);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      await refreshMarketPrices();
      if (mode === "inventory") await loadInventory(input);
      else await loadQuote();
      setProfile(await getMarketProfile());
    } catch (cause) {
      setError(String(cause));
      setLoading(false);
    }
  }

  const values = useMemo(() => {
    const rows = inventoryResult?.rows ?? [];
    return {
      owners: [...new Set(rows.map((row) => row.owner))],
      sources: [...new Set(rows.map((row) => row.source))],
      categories: [...new Set(rows.flatMap((row) => row.categoryName ? [row.categoryName] : []))],
      groups: [...new Set(rows.flatMap((row) => row.groupName ? [row.groupName] : []))],
      regions: [...new Set(rows.flatMap((row) => row.regionName ? [row.regionName] : []))],
      systems: [...new Set(rows.flatMap((row) => row.systemName ? [row.systemName] : []))],
      locations: [...new Set(rows.map((row) => row.locationName))],
    };
  }, [inventoryResult]);

  const set = (key: keyof InventorySearchInput, value: unknown) =>
    setInput((current) => ({ ...current, [key]: value || null }));

  const selectFilters: Array<[keyof InventorySearchInput, string, string[]]> = [
    ["character", "All characters", values.owners],
    ["source", "All owned sources", values.sources],
    ["category", "All categories", values.categories],
    ["group", "All groups", values.groups],
    ["region", "All owned regions", values.regions],
    ["system", "All owned systems", values.systems],
    ["location", "Owned item location", values.locations],
  ];

  const noMarketOrders = marketQuote && marketQuote.acquisition.filled === 0 && marketQuote.liquidation.filled === 0;
  const validMarketQuote = marketQuote?.acquisition.sufficient && marketQuote.liquidation.sufficient;

  return (
    <Panel
      title="Inventory Search & Market Valuation"
      keel="furnace"
      className="dash-hero"
      headerRight={<button className="target-select enabled" onClick={refresh} disabled={loading}>{loading ? "Working…" : "Refresh Prices"}</button>}
    >
      <div className="new-op-mode-toggle" style={{ marginBottom: 12 }}>
        <button className={mode === "inventory" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("inventory")}>My Inventory</button>
        <button className={mode === "market" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("market")}>Market Search</button>
      </div>
      <p className="ph-mission">{mode === "inventory" ? "Search synchronized and manual owned inventory, then value those stacks from the cached market snapshot." : "Search imported CCP static types and price any selected item directly from the cached market snapshot. Ownership is not required."}</p>
      {profile && <ProfileSummary profile={profile} />}
      {error && <div className="sd-error"><div className="conflict-desc">{mode === "inventory" ? "Inventory or market query failed" : "Market search failed"}: {error}</div></div>}

      {mode === "inventory" ? <>
        <div className="manual-inv-form" style={{ flexWrap: "wrap" }}>
          <input className="sd-path-input" style={{ minWidth: 300 }} placeholder="Search owned type, owner, location, path, flag, or source" value={input.text ?? ""} onChange={(event) => set("text", event.target.value)} />
          {selectFilters.map(([key, label, options]) => <select key={key} value={String(input[key] ?? "")} onChange={(event) => set(key, event.target.value)}><option value="">{label}</option>{options.map((option) => <option key={option}>{option}</option>)}</select>)}
          <select value={input.resolved == null ? "all" : input.resolved ? "resolved" : "unresolved"} onChange={(event) => setInput((current) => ({ ...current, resolved: event.target.value === "all" ? null : event.target.value === "resolved" }))}><option value="all">All owned resolution states</option><option value="resolved">Resolved</option><option value="unresolved">Unresolved</option></select>
          <select value={input.sort ?? "name"} onChange={(event) => set("sort", event.target.value)}><option value="name">Sort: Name</option><option value="quantity">Quantity</option><option value="owner">Owner</option><option value="location">Owned item location</option><option value="unit_acquisition">Unit acquisition price</option><option value="replacement">Replacement value</option><option value="unit_liquidation">Unit liquidation price</option><option value="liquidation">Liquidation value</option><option value="synced">Last synchronized</option></select>
          <label><input type="checkbox" checked={input.positiveOnly ?? false} onChange={(event) => setInput((current) => ({ ...current, positiveOnly: event.target.checked }))} /> Quantity &gt; 0</label>
        </div>
        {inventoryResult?.rows.length === 0 ? <div className="data-source" style={{ marginTop: 14 }}>No owned inventory matches this search.</div> : inventoryResult && <>
          <div className="res-summary-grid" style={{ margin: "12px 0" }}>
            <div className="res-summary-cell"><div className="res-summary-label">Matching stacks</div><div className="res-summary-value">{inventoryResult.matchingStacks.toLocaleString()}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Matching units</div><div className="res-summary-value">{inventoryResult.matchingUnits.toLocaleString()}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Total volume</div><div className="ops-field-value">{formatVolume(inventoryResult.totalVolumeM3)}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Replacement value</div><div className="ops-field-value">{isk(inventoryResult.totalReplacementValue)}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Liquidation value</div><div className="ops-field-value">{isk(inventoryResult.totalLiquidationValue)}</div></div>
          </div>
          <div className="inv-table" style={{ overflowX: "auto" }}>
            <div className="inv-row" style={{ gridTemplateColumns: "1.5fr .7fr .7fr .8fr .9fr 1.7fr .8fr .9fr .9fr .9fr .9fr", minWidth: 1750 }}><div>Type</div><div>Quantity</div><div>Unit m³</div><div>Stack m³</div><div>Owner</div><div>Owned item location</div><div>Unit buy cost</div><div>Replacement</div><div>Unit sell value</div><div>Liquidation</div><div>Price state</div></div>
            {inventoryResult.rows.map((row) => <div className="inv-row" key={row.stackKey} style={{ gridTemplateColumns: "1.5fr .7fr .7fr .8fr .9fr 1.7fr .8fr .9fr .9fr .9fr .9fr", minWidth: 1750 }}>
              <div className="inv-name">{row.typeName}<div className="bts-result-meta">{row.groupName ?? row.categoryName ?? "—"} · {row.source}</div></div><div>{row.quantity.toLocaleString()}</div>
              <div>{formatVolume(row.unitVolumeM3)}</div><div>{formatVolume(row.stackVolumeM3)}</div>
              <div>{row.owner}</div>
              <div>{row.locationName}<div className="bts-result-meta">{[row.systemName, row.regionName, ...row.containerPath].filter(Boolean).join(" → ")}</div></div>
              <div>{fillValue(row.quote.acquisition, true)}</div><div>{fillValue(row.quote.acquisition)}</div><div>{fillValue(row.quote.liquidation, true)}</div><div>{fillValue(row.quote.liquidation)}</div>
              <div className={`inv-status ${row.quote.stale ? "furnace" : "nominal"}`}>{row.quote.stale ? "STALE" : "CURRENT"}<div className="bts-result-meta">{row.quote.marketProfile} · {row.quote.source}<br />{row.quote.fetchedAt ? new Date(row.quote.fetchedAt).toLocaleString() : "No snapshot"}</div></div>
            </div>)}
          </div>
        </>}
      </> : <>
        <div className="manual-inv-form" style={{ flexWrap: "wrap" }}>
          <input className="sd-path-input" style={{ minWidth: 360 }} placeholder="Search imported EVE type name" value={marketQuery} onChange={(event) => { setMarketQuery(event.target.value); setSelectedType(null); setMarketQuote(null); }} />
          <label className="new-op-field"><span className="ops-field-label">Requested quantity</span><input className="sd-path-input" type="number" min="1" step="1" value={quantity} onChange={(event) => setQuantity(event.target.value)} /></label>
        </div>
        {typeSearched && typeResults.length === 0 && <div className="data-source" style={{ marginTop: 14 }}>No matching type.</div>}
        {typeResults.length > 0 && !selectedType && <div className="bts-results" style={{ marginTop: 10 }}>{typeResults.map((type) => <button className="bts-result" key={type.typeId} onClick={() => setSelectedType(type)}><span>{type.name}</span><span className="bts-result-meta">Type {type.typeId} · {type.groupName} · {type.categoryName} · {formatVolume(type.unitVolumeM3)}</span></button>)}</div>}
        {selectedType && <div style={{ marginTop: 14 }}>
          <div className="data-source">Selected: {selectedType.name} · Type ID {selectedType.typeId}</div>
          {!Number.isSafeInteger(Number(quantity)) || Number(quantity) < 1 ? <div className="sd-error"><div className="conflict-desc">Requested quantity must be a whole number of at least 1.</div></div> : marketQuote && <>
            <div className="res-summary-grid" style={{ margin: "12px 0" }}>
              <div className="res-summary-cell"><div className="res-summary-label">Item</div><div className="ops-field-value">{selectedType.name}</div><div className="bts-result-meta">Type ID {selectedType.typeId}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Requested quantity</div><div className="res-summary-value">{marketQuote.quantity.toLocaleString()}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Unit volume</div><div className="ops-field-value">{formatVolume(marketQuote.unitVolumeM3)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Requested volume</div><div className="ops-field-value">{formatVolume(marketQuote.requestedVolumeM3)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Acquisition unit price</div><div className="ops-field-value">{fillValue(marketQuote.acquisition, true)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Total acquisition cost</div><div className="ops-field-value">{fillValue(marketQuote.acquisition)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Liquidation unit price</div><div className="ops-field-value">{fillValue(marketQuote.liquidation, true)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Total liquidation value</div><div className="ops-field-value">{fillValue(marketQuote.liquidation)}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Available sell volume</div><div className="res-summary-value">{marketQuote.acquisition.availableVolume.toLocaleString()}</div></div>
              <div className="res-summary-cell"><div className="res-summary-label">Available buy volume</div><div className="res-summary-value">{marketQuote.liquidation.availableVolume.toLocaleString()}</div></div>
            </div>
            <div className="data-source">{marketQuote.marketProfile} · {marketQuote.source} · {marketQuote.stale ? "STALE" : "CURRENT"} · {marketQuote.fetchedAt ? new Date(marketQuote.fetchedAt).toLocaleString() : "No snapshot"}</div>
            {noMarketOrders ? <div className="tree-warning-list"><div className="conflict-row"><div className="conflict-type">No market orders</div><div className="conflict-desc">No cached sell or buy orders exist for this type at the selected market.</div></div></div> : <div className="tree-warning-list">
              {!marketQuote.acquisition.sufficient && <div className="conflict-row"><div className="conflict-type">Insufficient sell volume</div><div className="conflict-desc">{marketQuote.acquisition.filled.toLocaleString()} of {marketQuote.quantity.toLocaleString()} units can be acquired from cached sell orders.</div></div>}
              {!marketQuote.liquidation.sufficient && <div className="conflict-row"><div className="conflict-type">Insufficient buy volume</div><div className="conflict-desc">{marketQuote.liquidation.filled.toLocaleString()} of {marketQuote.quantity.toLocaleString()} units can be liquidated into cached buy orders.</div></div>}
              {validMarketQuote && <div className="conflict-row"><div className="conflict-type">Valid market quote</div><div className="conflict-desc">Cached sell and buy depth can satisfy the requested quantity.</div></div>}
            </div>}
          </>}
        </div>}
      </>}
    </Panel>
  );
}

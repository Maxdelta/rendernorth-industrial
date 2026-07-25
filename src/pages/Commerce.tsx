import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import { Panel } from "../components/Panel";
import { getAuthenticationConfig } from "../lib/authentication";
import {
  getCommerceOverview,
  getContractDashboard,
  getContractDetail,
  getMarketOrderDashboard,
  getMarketOrderHistoryDashboard,
  listCharacters,
  markAllMarketOrderHistorySeen,
  markMarketOrderHistorySeen,
  syncAllCommerce,
  syncAllContracts,
  syncAllMarketOrders,
  syncCharacterContracts,
  syncCharacterMarketOrders,
  type CharacterSummary,
  type CommerceOverview,
  type CommerceSyncOverview,
  type ContractDashboard,
  type ContractDetail,
  type MarketOrderDashboard,
  type MarketOrderHistoryDashboard,
} from "../lib/backend";
import { formatCompactIsk } from "../lib/isk.js";
import { activeOrderQuantityLabel, commerceKpiActive, commerceSyncAvailability, commerceTabAttributes, completedOrderActivityLabel, filterCompletedOrders, toggleCommerceKpiFilter, type CommerceKpiFilterId } from "../lib/commerceUiState.js";

type Tab = "orders" | "contracts";
type OrderView = "active" | "completed";
type KpiTone = "normal" | "positive" | "warning" | "risk" | "neutral" | "exposure";

function IskAmount({ value, className = "" }: { value: string | null; className?: string }) {
  const formatted = formatCompactIsk(value);
  return <span className={`commerce-isk ${className}`.trim()} title={formatted.exact} aria-label={formatted.exact} tabIndex={0}>{formatted.compact}</span>;
}

function KpiCard({ label, value, money, note, tone = "normal", symbol = "◆", active = false, onActivate }: { label: string; value?: string; money?: string | null; note?: string; tone?: KpiTone; symbol?: string; active?: boolean; onActivate?: () => void }) {
  const content = <>
    <div className="commerce-kpi-label"><span aria-hidden="true">{symbol}</span>{label}</div>
    <div className="commerce-kpi-value">{money !== undefined ? <IskAmount value={money}/> : value}</div>
    {note&&<div className="commerce-kpi-note">{note}</div>}
    {onActivate&&<div className="commerce-kpi-filter-state" aria-hidden="true">{active ? "Filtered · click to clear" : "Click to filter"}</div>}
  </>;
  return onActivate
    ? <button type="button" className={`commerce-kpi commerce-kpi-action ${tone}${active ? " active" : ""}`} aria-pressed={active} onClick={onActivate}>{content}</button>
    : <div className={`commerce-kpi ${tone}`}>{content}</div>;
}

const timeLeft = (seconds: number) => seconds < 0 ? "Expired" : seconds < 3600 ? `${Math.ceil(seconds / 60)}m` : seconds < 86400 ? `${Math.ceil(seconds / 3600)}h` : `${Math.ceil(seconds / 86400)}d`;
const date = (value: string) => new Date(value).toLocaleString();
const relativeAge = (value: string) => {
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(value).getTime()) / 1000));
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
};
const contractState = (status: string, remainingSeconds: number) => {
  if (status.startsWith("finished")) return "Finished";
  if (["cancelled", "rejected", "failed", "deleted", "reversed"].includes(status)) return status.replaceAll("_", " ").replace(/^./, value => value.toUpperCase());
  return timeLeft(remainingSeconds);
};

function SyncStatus({ label, state }: { label: string; state: CommerceSyncOverview }) {
  const statusLabel = state.status.replace(/^./, value => value.toUpperCase());
  const timestamp = state.oldestSuccessAt ? date(state.oldestSuccessAt) : "Never";
  const age = state.oldestSuccessAt ? relativeAge(state.oldestSuccessAt) : "Never";
  const aggregate = state.relevantCharacters > 1 ? ` · ${state.currentCharacters}/${state.relevantCharacters} characters current` : "";
  const errors = state.errorCharacters > 0 ? ` · ${state.errorCharacters} error${state.errorCharacters === 1 ? "" : "s"}` : "";
  return <div className={`commerce-sync-item ${state.status}`} title={`${label}: ${statusLabel}. Oldest relevant successful synchronization: ${timestamp}${aggregate}${errors}`}>
    <div className="commerce-sync-label">{label}</div>
    <div className="commerce-sync-state"><span aria-hidden="true">●</span>{statusLabel}</div>
    <div className="commerce-sync-age" tabIndex={0} aria-label={`${label} oldest relevant successful synchronization ${timestamp}`}>{age}{aggregate}</div>
  </div>;
}

export function CommercePage() {
  const [tab, setTab] = useState<Tab>("orders");
  const [characters, setCharacters] = useState<CharacterSummary[]>([]);
  const [clientId, setClientId] = useState("");
  const [orders, setOrders] = useState<MarketOrderDashboard | null>(null);
  const [history, setHistory] = useState<MarketOrderHistoryDashboard | null>(null);
  const [contracts, setContracts] = useState<ContractDashboard | null>(null);
  const [overview, setOverview] = useState<CommerceOverview | null>(null);
  const [detail, setDetail] = useState<ContractDetail | null>(null);
  const [detailTarget, setDetailTarget] = useState<number | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [syncing, setSyncing] = useState<"orders" | "contracts" | "commerce" | null>(null);
  const [syncFeedback, setSyncFeedback] = useState<string | null>(null);
  const [refreshVersion, setRefreshVersion] = useState(0);
  const [character, setCharacter] = useState("all");
  const [search, setSearch] = useState("");
  const [threshold, setThreshold] = useState(7);
  const [side, setSide] = useState("all");
  const [orderView, setOrderView] = useState<OrderView>("active");
  const [historyActivity, setHistoryActivity] = useState("all");
  const [historyDays, setHistoryDays] = useState(0);
  const [location, setLocation] = useState("");
  const [sort, setSort] = useState("expiring");
  const [direction, setDirection] = useState("all");
  const [status, setStatus] = useState("all");
  const [contractType, setContractType] = useState("all");
  const [availability, setAvailability] = useState("all");
  const [startLocation, setStartLocation] = useState("");
  const [endLocation, setEndLocation] = useState("");
  const [advancedOrders, setAdvancedOrders] = useState(false);
  const [advancedContracts, setAdvancedContracts] = useState(false);
  const detailModalRef = useRef<HTMLDivElement>(null);
  const detailCloseRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    listCharacters().then(setCharacters).catch(value => setError(String(value)));
    getAuthenticationConfig().then(value => setClientId(value.clientId)).catch(value => setError(String(value)));
  }, []);
  useEffect(() => {
    let active = true;
    getMarketOrderHistoryDashboard()
      .then(value => { if (active) setHistory(value); })
      .catch(value => { if (active) setError(String(value)); });
    return () => { active = false; };
  }, [refreshVersion]);
  const characterId = character === "all" ? null : Number(character);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    const call = tab === "orders"
      ? orderView === "active"
        ? getMarketOrderDashboard({ characterId, side, expiringDays: threshold, location, search, sort }).then(value => { if (active) setOrders(value); })
        : getMarketOrderHistoryDashboard().then(value => { if (active) setHistory(value); })
      : getContractDashboard({ characterId, status, contractType, availability, direction, startLocation, endLocation, search, expiringDays: threshold }).then(value => { if (active) setContracts(value); });
    call.catch(value => { if (active) setError(String(value)); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [tab, orderView, characterId, side, threshold, location, search, sort, status, contractType, availability, direction, startLocation, endLocation, refreshVersion]);
  useEffect(() => {
    let active = true;
    getCommerceOverview(characterId, threshold).then(value => { if (active) setOverview(value); }).catch(value => { if (active) setError(String(value)); });
    return () => { active = false; };
  }, [characterId, threshold, refreshVersion]);
  useEffect(() => {
    if (detailTarget === null) return;
    const routeOutlet = document.querySelector<HTMLElement>(".route-outlet");
    const previousOverflow = routeOutlet?.style.overflowY ?? "";
    const previousScrollbarGutter = routeOutlet?.style.scrollbarGutter ?? "";
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (routeOutlet) {
      routeOutlet.style.scrollbarGutter = "stable";
      routeOutlet.style.overflowY = "hidden";
    }
    detailCloseRef.current?.focus();
    const handleModalKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeDetail();
        return;
      }
      if (event.key !== "Tab" || !detailModalRef.current) return;
      const focusable = Array.from(detailModalRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )).filter(element => !element.hasAttribute("hidden"));
      if (focusable.length === 0) {
        event.preventDefault();
        detailModalRef.current.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && (document.activeElement === first || !detailModalRef.current.contains(document.activeElement))) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", handleModalKeyDown);
    return () => {
      document.removeEventListener("keydown", handleModalKeyDown);
      if (routeOutlet) {
        routeOutlet.style.overflowY = previousOverflow;
        routeOutlet.style.scrollbarGutter = previousScrollbarGutter;
      }
      previousFocus?.focus();
    };
  }, [detailTarget]);

  const charOptions = useMemo(() => characters.filter(value => value.enabled), [characters]);
  const syncAvailability = useMemo(() => commerceSyncAvailability(characters, characterId), [characters, characterId]);
  const completedRows = useMemo(() => filterCompletedOrders(history?.rows ?? [], {
    character,
    activity: historyActivity,
    search,
    location,
    days: historyDays,
  }), [history, character, historyActivity, search, location, historyDays]);
  const kpiState = { tab, side, direction, status };
  const ordersTabAttributes = commerceTabAttributes(tab, "orders");
  const contractsTabAttributes = commerceTabAttributes(tab, "contracts");
  const setRelativeTab = (event: KeyboardEvent<HTMLButtonElement>, next: Tab) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    setTab(next);
    const sibling = event.key === "ArrowRight" ? event.currentTarget.nextElementSibling : event.currentTarget.previousElementSibling;
    if (sibling instanceof HTMLButtonElement) sibling.focus();
  };
  async function openDetail(characterIdValue: number, contractId: number) {
    setDetailTarget(contractId); setDetail(null); setDetailError(null); setDetailLoading(true);
    try { setDetail(await getContractDetail(clientId, characterIdValue, contractId)); } catch (value) { setDetailError(String(value)); } finally { setDetailLoading(false); }
  }
  function closeDetail() { setDetailTarget(null); setDetail(null); setDetailError(null); setDetailLoading(false); }
  function activateKpi(filterId: CommerceKpiFilterId) {
    const next = toggleCommerceKpiFilter(kpiState, filterId);
    setTab(next.tab);
    setSide(next.side);
    setDirection(next.direction);
    setStatus(next.status);
  }
  function isKpiActive(filterId: CommerceKpiFilterId) { return commerceKpiActive(kpiState, filterId); }
  async function markHistorySeen(characterIdValue: number, orderId: number) {
    try {
      await markMarketOrderHistorySeen(characterIdValue, orderId);
      setRefreshVersion(value => value + 1);
    } catch (value) { setError(String(value)); }
  }
  async function markAllHistorySeen() {
    try {
      const count = await markAllMarketOrderHistorySeen();
      setSyncFeedback(`${count.toLocaleString()} completed order${count === 1 ? "" : "s"} marked read.`);
      setRefreshVersion(value => value + 1);
    } catch (value) { setError(String(value)); }
  }
  async function handleCommerceSync(kind: "orders" | "contracts" | "commerce") {
    if (!clientId || syncing) return;
    setSyncing(kind); setSyncFeedback(null); setError(null);
    try {
      const results = characterId === null
        ? kind === "orders" ? await syncAllMarketOrders(clientId) : kind === "contracts" ? await syncAllContracts(clientId) : await syncAllCommerce(clientId)
        : kind === "orders" ? [await syncCharacterMarketOrders(clientId, characterId)] : kind === "contracts" ? [await syncCharacterContracts(clientId, characterId)] : await Promise.all([syncCharacterMarketOrders(clientId, characterId), syncCharacterContracts(clientId, characterId)]);
      const failures = results.flatMap(result => result.error ? [result.error] : []);
      if (failures.length) setError(failures.join("; "));
      setSyncFeedback(failures.length ? "Commerce synchronization completed with errors." : `${kind === "orders" ? "Market Orders" : kind === "contracts" ? "Contracts" : "Commerce"} synchronized successfully.`);
      setCharacters(await listCharacters());
      setRefreshVersion(value => value + 1);
    } catch (value) {
      setError(String(value)); setSyncFeedback("Commerce synchronization failed; the previous successful snapshot remains available.");
    } finally { setSyncing(null); }
  }

  return <div className="page-stack commerce-page">
    <Panel title="Personal Commerce" keel="furnace">
      <p className="ph-mission">Read-only personal commerce across individually authorized characters. Corporation orders, corporation contracts, wallet activity, and trading actions are excluded.</p>
      <div className="commerce-tabs" role="tablist" aria-label="Commerce modules">
        <button id="commerce-orders-tab" role="tab" {...ordersTabAttributes} aria-controls="commerce-orders-panel" className={tab === "orders" ? "active" : ""} onClick={() => setTab("orders")} onKeyDown={event => setRelativeTab(event, "contracts")}><span aria-hidden="true">▤</span><span><strong>Market Orders {history && history.summary.newOrders > 0 ? <em className="commerce-new-badge">{history.summary.newOrders.toLocaleString()} New</em> : null}</strong><small>Active orders and completed activity</small></span></button>
        <button id="commerce-contracts-tab" role="tab" {...contractsTabAttributes} aria-controls="commerce-contracts-panel" className={tab === "contracts" ? "active" : ""} onClick={() => setTab("contracts")} onKeyDown={event => setRelativeTab(event, "orders")}><span aria-hidden="true">◇</span><span><strong>Contracts</strong><small>Issued, assigned, and accepted</small></span></button>
      </div>
      <div className="commerce-sync-controls" aria-label="Commerce synchronization controls">
        <button className="target-select enabled" onClick={() => handleCommerceSync("orders")} disabled={!clientId || !syncAvailability.marketOrders || syncing !== null}>{syncing === "orders" ? "Syncing Market Orders…" : "Sync Market Orders"}</button>
        <button className="target-select enabled" onClick={() => handleCommerceSync("contracts")} disabled={!clientId || !syncAvailability.contracts || syncing !== null}>{syncing === "contracts" ? "Syncing Contracts…" : "Sync Contracts"}</button>
        <button className="target-select enabled" onClick={() => handleCommerceSync("commerce")} disabled={!clientId || !syncAvailability.commerce || syncing !== null}>{syncing === "commerce" ? "Syncing Commerce…" : "Sync Commerce"}</button>
        <span className="commerce-sync-scope">{characterId === null ? "All enabled characters" : charOptions.find(value => value.characterId === characterId)?.name ?? "Selected character"}</span>
      </div>
      {syncFeedback&&<div className="setup-feedback" role="status">{syncFeedback}</div>}
      {error&&<div className="sd-error"><div className="conflict-desc">{error}</div></div>}
      <div className="commerce-filters" aria-label={`${tab === "orders" ? "Market order" : "Contract"} filters`}>
        <label><span>Character</span><select value={character} onChange={event => setCharacter(event.target.value)}><option value="all">All Characters</option>{charOptions.map(value => <option key={value.characterId} value={value.characterId}>{value.name}</option>)}</select></label>
        <label className="commerce-search-filter"><span>Search</span><input value={search} onChange={event => setSearch(event.target.value)} placeholder={tab === "orders" ? "Item or character" : "Title, item, or character"}/></label>
        {tab === "orders" && orderView === "active" && <><label><span>Expiring Soon</span><select value={threshold} onChange={event => setThreshold(Number(event.target.value))}><option value={1}>24 hours</option><option value={3}>3 days</option><option value={7}>7 days</option><option value={14}>14 days</option></select></label><label><span>Order Type</span><select value={side} onChange={event => setSide(event.target.value)}><option value="all">All Active Orders</option><option value="Buy">Buy</option><option value="Sell">Sell</option><option value="expiring">Expiring Soon</option></select></label></>}
        {tab === "orders" && orderView === "completed" && <><label><span>Activity</span><select value={historyActivity} onChange={event => setHistoryActivity(event.target.value)}><option value="all">All Activity</option><option value="Buy">Buy Orders</option><option value="Sell">Sell Orders</option><option value="Bought">Bought</option><option value="Sold">Sold</option><option value="Cancelled">Cancelled</option><option value="Expired">Expired</option></select></label><label><span>First Observed</span><select value={historyDays} onChange={event => setHistoryDays(Number(event.target.value))}><option value={0}>All available</option><option value={1}>Today</option><option value={7}>This week</option><option value={30}>30 days</option><option value={90}>90 days</option></select></label><label><span>Location</span><input value={location} onChange={event => setLocation(event.target.value)} placeholder="Station or structure"/></label></>}
        {tab === "contracts" && <><label><span>Expiring Soon</span><select value={threshold} onChange={event => setThreshold(Number(event.target.value))}><option value={1}>24 hours</option><option value={3}>3 days</option><option value={7}>7 days</option><option value={14}>14 days</option></select></label><label><span>Direction</span><select value={direction} onChange={event => setDirection(event.target.value)}><option value="all">All</option><option>Issued</option><option>Assigned</option><option>Accepted</option></select></label><label><span>Status</span><select value={status} onChange={event => setStatus(event.target.value)}><option value="all">All statuses</option><option value="outstanding">Outstanding</option><option value="in_progress">In Progress</option><option value="finished">Finished</option><option value="finished_issuer">Finished — Issuer</option><option value="finished_contractor">Finished — Contractor</option><option value="cancelled">Cancelled</option><option value="rejected">Rejected</option><option value="failed">Failed</option><option value="deleted">Deleted</option><option value="reversed">Reversed</option><option value="expired">Expired</option><option value="expiring">Expiring Soon</option></select></label></>}
      </div>
      {(tab === "contracts" || orderView === "active")&&<button className="commerce-advanced-toggle" aria-expanded={tab === "orders" ? advancedOrders : advancedContracts} onClick={() => tab === "orders" ? setAdvancedOrders(value => !value) : setAdvancedContracts(value => !value)}>{(tab === "orders" ? advancedOrders : advancedContracts) ? "Hide" : "Show"} Advanced Filters</button>}
      {tab === "orders" && orderView === "active" && advancedOrders&&<div className="commerce-filters commerce-advanced-filters"><label><span>Station or Structure</span><input value={location} onChange={event => setLocation(event.target.value)} placeholder="Location"/></label><label><span>Sort</span><select value={sort} onChange={event => setSort(event.target.value)}><option value="expiring">Expiring first</option><option value="value">Highest remaining value</option><option value="fill_high">Highest fill %</option><option value="fill_low">Lowest fill %</option><option value="newest">Newest</option><option value="oldest">Oldest</option><option value="item">Item name</option><option value="character">Character</option></select></label></div>}
      {tab === "contracts" && advancedContracts&&<div className="commerce-filters commerce-advanced-filters"><label><span>Contract Type</span><select value={contractType} onChange={event => setContractType(event.target.value)}><option value="all">All types</option><option value="item_exchange">Item Exchange</option><option value="courier">Courier</option><option value="auction">Auction</option><option value="loan">Loan</option><option value="unknown">Unknown</option></select></label><label><span>Availability</span><select value={availability} onChange={event => setAvailability(event.target.value)}><option value="all">All availability</option><option value="public">Public</option><option value="personal">Private</option><option value="corporation">Corporation</option><option value="alliance">Alliance</option></select></label><label><span>Start Location</span><input value={startLocation} onChange={event => setStartLocation(event.target.value)} placeholder="Name or ID"/></label><label><span>End Location</span><input value={endLocation} onChange={event => setEndLocation(event.target.value)} placeholder="Name or ID"/></label></div>}
      {loading&&<div className="setup-feedback">Loading commerce data…</div>}
    </Panel>

    {overview&&<div className="commerce-overview">
      <div className="commerce-exposure-card">
        <div className="commerce-kpi-label"><span aria-hidden="true">⬡</span>Total Commerce Exposure <span className="commerce-info" role="img" tabIndex={0} aria-label="Total Commerce Exposure includes Remaining Sell Value, Remaining Buy Commitment, Open Contract Value, and Collateral Exposure. It excludes escrow, rewards, finished contracts, realized revenue, and wallet balances." title="Included: Remaining Sell Value + Remaining Buy Commitment + Open Contract Value + Collateral Exposure. Excluded: escrow, rewards, finished contracts, realized revenue, and wallet balances.">i</span></div>
        <div className="commerce-exposure-value"><IskAmount value={overview.totalCommerceExposureIsk}/></div>
        <div className="commerce-kpi-note">Sell value + buy commitment + active contract value + active collateral. Escrow, rewards, and finished contracts excluded.</div>
      </div>
      <div className="commerce-sync" aria-label="Current Commerce synchronization state"><div className="commerce-sync-heading">Current synchronization state</div><SyncStatus label="Active Orders" state={overview.marketOrdersSync}/><SyncStatus label="Completed Orders" state={overview.marketHistorySync}/><SyncStatus label="Contracts" state={overview.contractsSync}/></div>
    </div>}

    {tab === "orders"&&<section id="commerce-orders-panel" role="tabpanel" aria-labelledby="commerce-orders-tab">
      <div className="commerce-order-tabs" role="tablist" aria-label="Market order views">
        <button className={orderView === "active" ? "active" : ""} aria-selected={orderView === "active"} onClick={() => setOrderView("active")}>Active Orders</button>
        <button className={orderView === "completed" ? "active" : ""} aria-selected={orderView === "completed"} onClick={() => setOrderView("completed")}>Completed Orders {history && history.summary.newOrders > 0 ? <span className="commerce-new-badge">{history.summary.newOrders.toLocaleString()} New</span> : null}</button>
      </div>
      {orderView === "active"&&orders&&<>
        <div className="commerce-summary"><KpiCard label="Active Orders" value={orders.summary.activeOrders.toLocaleString()} tone="positive" symbol="●"/><KpiCard label="Sell Orders" value={orders.summary.sellOrders.toLocaleString()} tone="positive" symbol="▲" active={isKpiActive("orders-sell")} onActivate={() => activateKpi("orders-sell")}/><KpiCard label="Buy Orders" value={orders.summary.buyOrders.toLocaleString()} tone="normal" symbol="▼" active={isKpiActive("orders-buy")} onActivate={() => activateKpi("orders-buy")}/><KpiCard label="Remaining Sell Value" money={orders.summary.remainingSellValueIsk}/><KpiCard label="Remaining Buy Commitment" money={orders.summary.remainingBuyCommitmentIsk}/><KpiCard label="Total Escrow" money={orders.summary.totalEscrowIsk} tone="neutral"/><KpiCard label="Expiring Soon" value={orders.summary.expiringSoon.toLocaleString()} note={`Within ${threshold} day${threshold === 1 ? "" : "s"}`} tone="warning" symbol="!" active={isKpiActive("orders-expiring")} onActivate={() => activateKpi("orders-expiring")}/></div>
        <Panel title="Active Personal Market Orders" keel="coolant"><p className="data-source">Quantity is shown as remaining / original. CCP’s active-order endpoint returns open orders only.</p>{orders.rows.length === 0 ? <p className="empty-state">No active personal market orders match these filters.</p> : <div className="commerce-table-wrap"><table className="commerce-table"><thead><tr><th>Item</th><th>Buy/Sell</th><th>Character</th><th>Location</th><th>Price</th><th>Quantity</th><th>Filled</th><th>Filled %</th><th>Issued</th><th>Expires</th><th>Time Left</th><th>Escrow</th><th>Last Synced</th></tr></thead><tbody>{orders.rows.map(row => <tr key={`${row.characterId}-${row.orderId}`}><td><strong>{row.itemName}</strong><small>Type {row.typeId}</small></td><td><span className={`commerce-kind ${row.side.toLowerCase()}`}>{row.side}</span></td><td>{row.characterName}</td><td>{row.locationName}<small>{row.solarSystemName}{row.regionName ? ` · ${row.regionName}` : ""}</small></td><td><IskAmount value={row.priceIsk}/></td><td>{activeOrderQuantityLabel(row)}<small>remaining / original</small></td><td>{row.quantityFilled.toLocaleString()}</td><td>{row.fillPercentage.toFixed(1)}%</td><td>{date(row.issuedAt)}</td><td>{date(row.expiresAt)}</td><td>{timeLeft(row.remainingSeconds)}</td><td><IskAmount value={row.escrowIsk}/></td><td>{date(row.lastSynced)}</td></tr>)}</tbody></table></div>}</Panel>
      </>}
      {orderView === "completed"&&history&&<>
        <div className="commerce-summary"><KpiCard label="New Orders" value={history.summary.newOrders.toLocaleString()} tone={history.summary.newOrders ? "warning" : "neutral"} symbol="●"/><KpiCard label="Completed Today" value={history.summary.completedToday.toLocaleString()} note="First observed today"/><KpiCard label="This Week" value={history.summary.completedThisWeek.toLocaleString()} note="First observed in 7 days"/><KpiCard label="Cancelled" value={history.summary.cancelled.toLocaleString()} tone="neutral"/><KpiCard label="Expired" value={history.summary.expired.toLocaleString()} tone="neutral"/></div>
        <Panel title="Completed Order Activity" keel="coolant">
          <div className="commerce-activity-head"><p className="data-source">CCP provides cancelled and expired orders from up to 90 days. RenderNorth derives bought/sold quantity from original minus remaining. Activity dates are when RenderNorth first observed each history record because CCP does not provide a close timestamp.</p><button className="target-select" onClick={markAllHistorySeen} disabled={history.summary.newOrders === 0}>Mark All Read</button></div>
          {completedRows.length === 0 ? <p className="empty-state">No completed personal market orders match these filters.</p> : <div className="commerce-table-wrap"><table className="commerce-table commerce-history-table"><thead><tr><th>Status</th><th>Item</th><th>Activity</th><th>Character</th><th>Location</th><th>Price</th><th>Quantity</th><th>CCP State</th><th>Issued</th><th>Expected Expiry</th><th>First Observed</th><th>Last Observed</th><th></th></tr></thead><tbody>{completedRows.map(row => <tr key={`${row.characterId}-${row.orderId}`} className={row.seen ? "" : "commerce-history-new"}><td><span className={row.seen ? "commerce-seen" : "commerce-new-badge"}>{row.seen ? "Seen" : "New"}</span></td><td><strong>{row.itemName}</strong><small>Type {row.typeId} · Order {row.orderId}</small></td><td><span className={`commerce-kind ${row.side.toLowerCase()}`}>{completedOrderActivityLabel(row)}</span></td><td>{row.characterName}</td><td>{row.locationName}<small>{row.solarSystemName}{row.regionName ? ` · ${row.regionName}` : ""}</small></td><td><IskAmount value={row.priceIsk}/></td><td>{row.completedQuantity.toLocaleString()} completed<small>{row.volumeRemain.toLocaleString()} remaining / {row.volumeTotal.toLocaleString()} original</small></td><td>{row.esiState}</td><td>{date(row.issuedAt)}</td><td>{date(row.expiresAt)}</td><td>{date(row.firstSeenAt)}</td><td>{date(row.lastObservedAt)}</td><td>{!row.seen&&<button className="target-select" onClick={() => markHistorySeen(row.characterId, row.orderId)}>Mark Read</button>}</td></tr>)}</tbody></table></div>}
        </Panel>
      </>}
    </section>}

    {tab === "contracts"&&contracts&&<section id="commerce-contracts-panel" role="tabpanel" aria-labelledby="commerce-contracts-tab">
      <div className="commerce-summary"><KpiCard label="Outstanding Contracts" value={contracts.summary.outstandingContracts.toLocaleString()} tone="positive" symbol="●" active={isKpiActive("contracts-outstanding")} onActivate={() => activateKpi("contracts-outstanding")}/><KpiCard label="Assigned to Me" value={contracts.summary.assignedToMe.toLocaleString()} active={isKpiActive("contracts-assigned")} onActivate={() => activateKpi("contracts-assigned")}/><KpiCard label="Issued by Me" value={contracts.summary.issuedByMe.toLocaleString()} active={isKpiActive("contracts-issued")} onActivate={() => activateKpi("contracts-issued")}/><KpiCard label="In Progress" value={contracts.summary.inProgress.toLocaleString()} tone="positive" symbol="▶" active={isKpiActive("contracts-progress")} onActivate={() => activateKpi("contracts-progress")}/><KpiCard label="Expiring Soon" value={contracts.summary.expiringSoon.toLocaleString()} tone="warning" symbol="!" active={isKpiActive("contracts-expiring")} onActivate={() => activateKpi("contracts-expiring")}/><KpiCard label="Finished" value={contracts.summary.completedOrFinished.toLocaleString()} tone="neutral" symbol="✓" active={isKpiActive("contracts-finished")} onActivate={() => activateKpi("contracts-finished")}/><KpiCard label="Collateral Exposure" money={contracts.summary.totalCollateralExposureIsk} tone="risk" symbol="!"/><KpiCard label="Outstanding Rewards" money={contracts.summary.outstandingRewardsIsk} tone="positive"/><KpiCard label="Open Contract Value" money={contracts.summary.outstandingContractValueIsk}/></div>
      <Panel title="Personal Contracts" keel="coolant"><p className="data-source">CCP returns contracts where the character is issuer, acceptor, or assignee, limited to the last 30 days unless still in progress. Items and auction bids load only when viewed.</p>{contracts.rows.length === 0 ? <p className="empty-state">No personal contracts match these filters.</p> : <div className="commerce-table-wrap"><table className="commerce-table"><thead><tr><th>Title</th><th>Type</th><th>Character</th><th>Direction</th><th>Availability</th><th>Status</th><th>Start</th><th>End</th><th>Price</th><th>Reward</th><th>Collateral</th><th>Issued</th><th>Expires</th><th>Time / State</th><th>Last Synced</th><th></th></tr></thead><tbody>{contracts.rows.map(row => <tr key={`${row.characterId}-${row.contractId}`}><td><strong>{row.title}</strong><small>Contract {row.contractId}</small></td><td>{row.contractType.replaceAll("_", " ")}</td><td>{row.characterName}</td><td>{row.direction}</td><td>{row.availability}</td><td>{row.status.replaceAll("_", " ")}</td><td>{row.startLocationName ?? (row.startLocationId ? `Location ${row.startLocationId}` : "—")}</td><td>{row.endLocationName ?? (row.endLocationId ? `Location ${row.endLocationId}` : "—")}</td><td><IskAmount value={row.priceIsk}/></td><td><IskAmount value={row.rewardIsk}/></td><td><IskAmount value={row.collateralIsk}/></td><td>{date(row.dateIssued)}</td><td>{date(row.dateExpired)}</td><td>{contractState(row.status, row.remainingSeconds)}</td><td>{date(row.lastSynced)}</td><td><button className="target-select" onClick={() => openDetail(row.characterId, row.contractId)}>View Details</button></td></tr>)}</tbody></table></div>}</Panel>
    </section>}

    {detailTarget !== null&&<div className="contract-detail-overlay" role="dialog" aria-modal="true" aria-label={`Contract ${detailTarget} details`}><div className="contract-detail-modal" ref={detailModalRef} tabIndex={-1}><div className="contract-detail-head"><div><span className="panel-kicker">Contract Details</span><h2>Contract {detailTarget}</h2></div><button className="target-select" ref={detailCloseRef} onClick={closeDetail}>Close</button></div>{detailLoading&&<div className="setup-feedback">Loading contract metadata, items, and applicable bids…</div>}{detailError&&<div className="sd-error"><div className="conflict-title">Unable to load contract details</div><div className="conflict-desc">{detailError}</div></div>}{detail&&<><div className="contract-detail-grid"><div><span>Title</span><strong>{detail.contract.title}</strong></div><div><span>Type</span><strong>{detail.contract.contractType}</strong></div><div><span>Direction</span><strong>{detail.contract.direction}</strong></div><div><span>Availability</span><strong>{detail.contract.availability}</strong></div><div><span>Status</span><strong>{detail.contract.status}</strong></div><div><span>Character</span><strong>{detail.contract.characterName}</strong></div><div><span>Issuer ID</span><strong>{detail.contract.issuerId}</strong></div><div><span>Issuer Corporation ID</span><strong>{detail.contract.issuerCorporationId}</strong></div><div><span>Assignee ID</span><strong>{detail.contract.assigneeId || "—"}</strong></div><div><span>Acceptor ID</span><strong>{detail.contract.acceptorId || "—"}</strong></div><div><span>Start</span><strong>{detail.contract.startLocationName ?? detail.contract.startLocationId ?? "—"}</strong></div><div><span>Destination</span><strong>{detail.contract.endLocationName ?? detail.contract.endLocationId ?? "—"}</strong></div><div><span>Price</span><strong><IskAmount value={detail.contract.priceIsk}/></strong></div><div><span>Reward</span><strong><IskAmount value={detail.contract.rewardIsk}/></strong></div><div><span>Collateral</span><strong><IskAmount value={detail.contract.collateralIsk}/></strong></div><div><span>Buyout</span><strong><IskAmount value={detail.contract.buyoutIsk}/></strong></div><div><span>Issued</span><strong>{date(detail.contract.dateIssued)}</strong></div><div><span>Expires</span><strong>{date(detail.contract.dateExpired)}</strong></div><div><span>Source</span><strong>{detail.contract.source}</strong></div><div><span>Last Synced</span><strong>{date(detail.contract.lastSynced)}</strong></div></div>{detail.itemsError&&<div className="sd-error">Items: {detail.itemsError}</div>}<div className="contract-items"><div><h3>Items Offered</h3>{detail.items.filter(item => item.included).length === 0 ? <p className="empty-state">No offered items returned.</p> : detail.items.filter(item => item.included).map(item => <p key={item.recordId}>{item.itemName} × {item.quantity.toLocaleString()}</p>)}</div><div><h3>Items Requested</h3>{detail.items.filter(item => !item.included).length === 0 ? <p className="empty-state">No requested items returned.</p> : detail.items.filter(item => !item.included).map(item => <p key={item.recordId}>{item.itemName} × {item.quantity.toLocaleString()}</p>)}</div></div>{detail.contract.contractType === "auction"&&<div><h3>Auction Bids</h3>{detail.bidsError&&<div className="sd-error">Bids: {detail.bidsError}</div>}{detail.bids.length === 0 ? <p className="empty-state">No bids returned.</p> : detail.bids.map(bid => <p key={bid.bidId}><IskAmount value={bid.amountIsk}/> · bidder {bid.bidderId} · {date(bid.dateBid)}</p>)}</div>}</>}</div></div>}
  </div>;
}

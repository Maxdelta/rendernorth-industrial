import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { activeOrderQuantityLabel, commerceKpiActive, commerceSyncAvailability, commerceTabAttributes, completedOrderActivityLabel, contractKpiCounts, contractTimeline, filterCompletedOrders, filterContracts, groupContractItems, switchCommerceTab, toggleCommerceKpiFilter } from "./commerceUiState.js";

test("active Commerce tab exposes selected and keyboard state", () => {
  assert.deepEqual(commerceTabAttributes("orders", "orders"), { "aria-selected": true, tabIndex: 0 });
  assert.deepEqual(commerceTabAttributes("orders", "contracts"), { "aria-selected": false, tabIndex: -1 });
});

test("switching Commerce modules preserves independent filter state", () => {
  const orderFilters = { side: "Sell", location: "Jita", sort: "value" };
  const contractFilters = { direction: "Issued", status: "outstanding", availability: "public" };
  const switched = switchCommerceTab({ tab: "orders", orderFilters, contractFilters }, "contracts");
  assert.equal(switched.tab, "contracts");
  assert.strictEqual(switched.orderFilters, orderFilters);
  assert.strictEqual(switched.contractFilters, contractFilters);
});

test("Buy, Sell, and Expiring Soon KPI filters update the existing order filter", () => {
  const initial = { tab: "orders", side: "all", direction: "all", status: "all" };
  assert.equal(toggleCommerceKpiFilter(initial, "orders-buy").side, "Buy");
  assert.equal(toggleCommerceKpiFilter(initial, "orders-sell").side, "Sell");
  assert.equal(toggleCommerceKpiFilter(initial, "orders-expiring").side, "expiring");
});

test("clicking an active KPI toggles it off without separate active state", () => {
  const active = { tab: "orders", side: "Buy", direction: "all", status: "all" };
  assert.equal(commerceKpiActive(active, "orders-buy"), true);
  const cleared = toggleCommerceKpiFilter(active, "orders-buy");
  assert.equal(cleared.side, "all");
  assert.equal(commerceKpiActive(cleared, "orders-buy"), false);
});

test("contract KPI filters switch tabs and preserve unrelated manual filters", () => {
  const initial = { tab: "orders", side: "Sell", direction: "all", status: "all", search: "Raven" };
  const outstanding = toggleCommerceKpiFilter(initial, "contracts-outstanding");
  assert.equal(outstanding.tab, "contracts");
  assert.equal(outstanding.status, "outstanding");
  assert.equal(outstanding.side, "Sell");
  assert.equal(outstanding.search, "Raven");
  assert.equal(commerceKpiActive(outstanding, "contracts-outstanding"), true);
});

test("Assigned, Issued, Finished, and contract Expiring KPI filters share manual fields", () => {
  const initial = { tab: "contracts", side: "all", direction: "all", status: "all", activity: "all", readState: "all" };
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-assigned").direction, "Assigned");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-issued").direction, "Issued");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-accepted").status, "in_progress");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-finished").activity, "completed");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-cancelled").activity, "cancelled");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-expired").activity, "expired");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-new").readState, "new");
  assert.equal(toggleCommerceKpiFilter(initial, "contracts-expiring").status, "expiring");
});

test("manual filter changes drive KPI active feedback", () => {
  const manual = { tab: "contracts", side: "all", direction: "Issued", status: "all", activity: "completed" };
  assert.equal(commerceKpiActive(manual, "contracts-issued"), true);
  assert.equal(commerceKpiActive(manual, "contracts-finished"), true);
  assert.equal(commerceKpiActive({ ...manual, direction: "all" }, "contracts-issued"), false);
});

test("Commerce sync availability follows enabled characters and granted scopes", () => {
  const characters = [
    { characterId: 1, enabled: true, marketOrderScopeGranted: true, contractScopeGranted: true },
    { characterId: 2, enabled: false, marketOrderScopeGranted: true, contractScopeGranted: true },
    { characterId: 3, enabled: true, marketOrderScopeGranted: true, contractScopeGranted: false },
  ];
  assert.deepEqual(commerceSyncAvailability(characters), { marketOrders: true, contracts: true, commerce: true });
  assert.deepEqual(commerceSyncAvailability(characters, 3), { marketOrders: true, contracts: false, commerce: false });
  assert.deepEqual(commerceSyncAvailability(characters, 2), { marketOrders: false, contracts: false, commerce: false });
});

test("Commerce page exposes all three existing-command sync controls", () => {
  const source = readFileSync(new URL("../pages/Commerce.tsx", import.meta.url), "utf8");
  for (const label of ["Sync Market Orders", "Sync Contracts", "Sync Commerce"]) assert.ok(source.includes(`"${label}"`));
  assert.match(source, /syncAllMarketOrders/);
  assert.match(source, /syncAllContracts/);
  assert.match(source, /syncAllCommerce/);
});

test("Commerce sticky headers are paint-contained inside their table scroll region", () => {
  const css = readFileSync(new URL("../styles/app.css", import.meta.url), "utf8");
  assert.match(css, /\.commerce-table-wrap\s*\{[^}]*isolation:isolate;[^}]*contain:paint;/s);
  assert.match(css, /\.commerce-table th\s*\{[^}]*position:sticky;[^}]*top:0;[^}]*z-index:4;[^}]*background-color:#0d141d;[^}]*background-clip:padding-box;/s);
  assert.match(css, /\.commerce-summary\s*\{[^}]*position:relative;/s);
});

test("Contract Details uses an opaque isolated surface and a blur-independent dark backdrop", () => {
  const css = readFileSync(new URL("../styles/app.css", import.meta.url), "utf8");
  assert.match(css, /\.contract-detail-overlay\s*\{[^}]*position:fixed;[^}]*inset:0;[^}]*z-index:2000;[^}]*background:rgba\(2,6,10,.9\);[^}]*backdrop-filter:blur\(8px\)/s);
  assert.match(css, /\.contract-detail-modal\s*\{[^}]*overscroll-behavior:contain;[^}]*opacity:1;[^}]*background-color:#0d141d;/s);
  assert.match(css, /@supports not \(\(-webkit-backdrop-filter:blur\(1px\)\) or \(backdrop-filter:blur\(1px\)\)\)\s*\{[^}]*background:rgba\(2,6,10,.95\);/s);
});

test("Contract Details locks background scrolling and preserves keyboard modal behavior", () => {
  const source = readFileSync(new URL("../pages/Commerce.tsx", import.meta.url), "utf8");
  assert.match(source, /routeOutlet\.style\.scrollbarGutter = "stable"/);
  assert.match(source, /routeOutlet\.style\.overflowY = "hidden"/);
  assert.match(source, /event\.key === "Escape"/);
  assert.match(source, /event\.key !== "Tab"/);
  assert.match(source, /detailCloseRef\.current\?\.focus\(\)/);
  assert.match(source, /previousFocus\?\.focus\(\)/);
});

test("active quantity uses unambiguous remaining / original wording", () => {
  assert.equal(activeOrderQuantityLabel({ volumeRemain: 1250, volumeTotal: 4000 }), "1,250 / 4,000");
});

test("completed activity uses Sold, Bought, and Completed wording deterministically", () => {
  assert.equal(completedOrderActivityLabel({ side: "Sell", volumeRemain: 2, volumeTotal: 10 }), "Sold 8");
  assert.equal(completedOrderActivityLabel({ side: "Buy", volumeRemain: 2, volumeTotal: 10 }), "Bought 8");
  assert.equal(completedOrderActivityLabel({ side: "Sell", volumeRemain: 10, volumeTotal: 10 }), "Completed");
});

test("completed order filters combine character, side/state, date, item, and location", () => {
  const now = Date.parse("2026-07-25T12:00:00Z");
  const rows = [
    { characterId: 1, characterName: "Maxdelta", itemName: "Tritanium", locationName: "Jita 4-4", side: "Sell", esiState: "expired", volumeRemain: 0, volumeTotal: 10, firstSeenAt: "2026-07-25T10:00:00Z" },
    { characterId: 2, characterName: "Sabre side", itemName: "Pyerite", locationName: "Amarr", side: "Buy", esiState: "cancelled", volumeRemain: 5, volumeTotal: 10, firstSeenAt: "2026-07-01T10:00:00Z" },
  ];
  assert.equal(filterCompletedOrders(rows, { character: 1, activity: "sold", search: "trit", location: "jita", days: 1 }, now).length, 1);
  assert.equal(filterCompletedOrders(rows, { activity: "cancelled" }, now)[0].characterId, 2);
  assert.equal(filterCompletedOrders(rows, { activity: "bought", days: 7 }, now).length, 0);
});

test("completed order client filtering remains deterministic for 2,500 records", () => {
  const rows = Array.from({ length: 2500 }, (_, index) => ({
    characterId: index % 2,
    characterName: `Pilot ${index % 2}`,
    itemName: index % 5 === 0 ? "Tritanium" : "Pyerite",
    locationName: index % 3 === 0 ? "Jita" : "Amarr",
    side: index % 2 === 0 ? "Sell" : "Buy",
    esiState: index % 4 === 0 ? "cancelled" : "expired",
    volumeRemain: 0,
    volumeTotal: 10,
    firstSeenAt: "2026-07-25T10:00:00Z",
  }));
  const filtered = filterCompletedOrders(rows, { activity: "sold", search: "trit", location: "jita" }, Date.parse("2026-07-25T12:00:00Z"));
  assert.equal(filtered.length, 84);
});

test("Completed Orders exposes unread badge and local read controls", () => {
  const source = readFileSync(new URL("../pages/Commerce.tsx", import.meta.url), "utf8");
  assert.match(source, /Completed Orders/);
  assert.match(source, /history\.summary\.newOrders > 0/);
  assert.match(source, /commerce-new-badge/);
  assert.match(source, /markMarketOrderHistorySeen/);
  assert.match(source, /markAllMarketOrderHistorySeen/);
  assert.match(source, /Mark All Read/);
});

test("contract activity filtering combines tabs, activity, seen state, KPI fields, and search", () => {
  const rows = [
    { contractId: 1, characterId: 1, characterName: "Maxdelta", title: "Minerals", searchText: "Tritanium", direction: "Issued", status: "outstanding", contractType: "item_exchange", availability: "personal", startLocationId: 600, startLocationName: "Jita", endLocationId: null, endLocationName: null, remainingSeconds: 3600, activityCategory: "active", seen: false, dateIssued: "2026-07-25T09:00:00Z", dateCompleted: null, firstObservedAt: "2026-07-25T10:00:00Z" },
    { contractId: 2, characterId: 2, characterName: "Sabre side", title: "Ships", searchText: "Hobgoblin II", direction: "Accepted", status: "finished", contractType: "courier", availability: "public", startLocationId: 601, startLocationName: "Amarr", endLocationId: 602, endLocationName: "Dodixie", remainingSeconds: -1, activityCategory: "completed", seen: false, dateIssued: "2026-07-24T09:00:00Z", dateCompleted: "2026-07-25T10:00:00Z", firstObservedAt: "2026-07-25T10:05:00Z" },
    { contractId: 3, characterId: 1, characterName: "Maxdelta", title: "Modules", searchText: "Damage Control II", direction: "Issued", status: "cancelled", contractType: "item_exchange", availability: "personal", startLocationId: 600, startLocationName: "Jita", endLocationId: null, endLocationName: null, remainingSeconds: -1, activityCategory: "cancelled", seen: true, dateIssued: "2026-06-01T09:00:00Z", dateCompleted: null, firstObservedAt: "2026-06-15T10:00:00Z" },
  ];
  assert.equal(filterContracts(rows, { view: "active", status: "expiring", thresholdDays: 1 }).length, 1);
  assert.equal(filterContracts(rows, { view: "activity", activity: "completed", readState: "new", character: 2, search: "hobgoblin", startLocation: "amarr", days: 1 }, Date.parse("2026-07-25T12:00:00Z")).length, 1);
  assert.equal(filterContracts(rows, { view: "activity", activity: "cancelled", readState: "seen", direction: "Issued" }).length, 1);
  assert.equal(filterContracts(rows, { view: "activity", days: 7 }, Date.parse("2026-07-25T12:00:00Z")).length, 1);
});

test("contract KPI counts drive all nine operational cards", () => {
  const rows = [
    { activityCategory: "active", status: "outstanding", direction: "Assigned", remainingSeconds: 100, seen: false },
    { activityCategory: "active", status: "in_progress", direction: "Accepted", remainingSeconds: 999999, seen: false },
    { activityCategory: "completed", status: "finished", direction: "Issued", remainingSeconds: -1, seen: false },
    { activityCategory: "cancelled", status: "cancelled", direction: "Issued", remainingSeconds: -1, seen: true },
    { activityCategory: "expired", status: "outstanding", direction: "Issued", remainingSeconds: -1, seen: false },
  ];
  assert.deepEqual(contractKpiCounts(rows, 1), { outstanding: 1, assigned: 1, issued: 0, accepted: 1, completed: 1, cancelled: 1, expired: 1, expiring: 1, newActivity: 2 });
});

test("New Activity KPI reflects manual read-state filters and clears on second click", () => {
  const manual = { tab: "contracts", side: "all", direction: "all", status: "all", activity: "all", readState: "new" };
  assert.equal(commerceKpiActive(manual, "contracts-new"), true);
  assert.equal(toggleCommerceKpiFilter(manual, "contracts-new").readState, "all");
});

test("grouped contract items preserve raw records while aggregating identical presentation rows", () => {
  const raw = [
    { recordId: 1, typeId: 2456, itemName: "Hobgoblin II", quantity: 10, singleton: false, included: true },
    { recordId: 2, typeId: 2456, itemName: "Hobgoblin II", quantity: 8, singleton: false, included: true },
    { recordId: 3, typeId: 2456, itemName: "Hobgoblin II", quantity: 2, singleton: false, included: false },
  ];
  const snapshot = structuredClone(raw);
  const grouped = groupContractItems(raw);
  assert.deepEqual(raw, snapshot);
  assert.equal(grouped.length, 2);
  assert.equal(grouped.find(row => row.included).quantity, 18);
  assert.deepEqual(grouped.find(row => row.included).recordIds, [1, 2]);
});

test("contract timeline renders only CCP timestamps and never invents cancellation time", () => {
  const events = contractTimeline({ dateIssued: "2026-07-01T00:00:00Z", dateAccepted: "2026-07-02T00:00:00Z", dateCompleted: null, dateExpired: "2026-07-30T00:00:00Z", activityCategory: "cancelled" });
  assert.deepEqual(events.map(event => event.label), ["Issued", "Accepted", "Expiry"]);
  assert.equal(events.some(event => event.label === "Cancelled"), false);
});

test("contract client filtering remains deterministic for 5,000 synchronized rows", () => {
  const rows = Array.from({ length: 5000 }, (_, index) => ({
    contractId: index, characterId: index % 2, characterName: `Pilot ${index % 2}`, title: index % 5 === 0 ? "Minerals" : "Ships",
    direction: index % 2 === 0 ? "Issued" : "Accepted", status: index % 3 === 0 ? "finished" : "cancelled",
    contractType: "item_exchange", availability: "personal", startLocationId: 600, startLocationName: index % 4 === 0 ? "Jita" : "Amarr",
    endLocationId: null, endLocationName: null, remainingSeconds: -1, activityCategory: index % 3 === 0 ? "completed" : "cancelled", seen: index % 7 === 0,
    searchText: "", dateIssued: "2026-07-01T00:00:00Z", dateCompleted: null, firstObservedAt: "2026-07-25T10:00:00Z",
  }));
  const filtered = filterContracts(rows, { view: "activity", activity: "completed", readState: "new", search: "minerals", startLocation: "jita" });
  assert.equal(filtered.length, 72);
});

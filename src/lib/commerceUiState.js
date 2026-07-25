export function commerceTabAttributes(activeTab, candidateTab) {
  const active = activeTab === candidateTab;
  return { "aria-selected": active, tabIndex: active ? 0 : -1 };
}

export function switchCommerceTab(state, nextTab) {
  return { ...state, tab: nextTab };
}

export const commerceKpiFilters = Object.freeze({
  "orders-sell": { tab: "orders", field: "side", value: "Sell", clearValue: "all" },
  "orders-buy": { tab: "orders", field: "side", value: "Buy", clearValue: "all" },
  "orders-expiring": { tab: "orders", field: "side", value: "expiring", clearValue: "all" },
  "contracts-outstanding": { tab: "contracts", field: "status", value: "outstanding", clearValue: "all" },
  "contracts-assigned": { tab: "contracts", field: "direction", value: "Assigned", clearValue: "all" },
  "contracts-issued": { tab: "contracts", field: "direction", value: "Issued", clearValue: "all" },
  "contracts-progress": { tab: "contracts", field: "status", value: "in_progress", clearValue: "all" },
  "contracts-expiring": { tab: "contracts", field: "status", value: "expiring", clearValue: "all" },
  "contracts-finished": { tab: "contracts", field: "status", value: "finished", clearValue: "all" },
});

export function commerceKpiActive(state, filterId) {
  const definition = commerceKpiFilters[filterId];
  return Boolean(definition && state.tab === definition.tab && state[definition.field] === definition.value);
}

export function toggleCommerceKpiFilter(state, filterId) {
  const definition = commerceKpiFilters[filterId];
  if (!definition) return state;
  const active = state.tab === definition.tab && state[definition.field] === definition.value;
  return { ...state, tab: definition.tab, [definition.field]: active ? definition.clearValue : definition.value };
}

export function commerceSyncAvailability(characters, selectedCharacterId = null) {
  const eligible = characters.filter(character => character.enabled && (selectedCharacterId === null || character.characterId === selectedCharacterId));
  const marketOrders = eligible.some(character => character.marketOrderScopeGranted);
  const contracts = eligible.some(character => character.contractScopeGranted);
  return { marketOrders, contracts, commerce: marketOrders && contracts };
}

export function activeOrderQuantityLabel(row) {
  return `${Number(row.volumeRemain).toLocaleString("en-US")} / ${Number(row.volumeTotal).toLocaleString("en-US")}`;
}

export function completedOrderActivityLabel(row) {
  const completed = Math.max(0, Number(row.volumeTotal) - Number(row.volumeRemain));
  if (completed === 0) return "Completed";
  return `${row.side === "Buy" ? "Bought" : "Sold"} ${completed.toLocaleString("en-US")}`;
}

export function filterCompletedOrders(rows, filters, nowMs = Date.now()) {
  const character = filters.character ?? "all";
  const activity = String(filters.activity ?? "all").toLowerCase();
  const search = String(filters.search ?? "").trim().toLowerCase();
  const location = String(filters.location ?? "").trim().toLowerCase();
  const days = Number(filters.days ?? 0);
  const cutoff = days > 0 ? nowMs - days * 86_400_000 : Number.NEGATIVE_INFINITY;
  return rows.filter(row => {
    const completed = Math.max(0, Number(row.volumeTotal) - Number(row.volumeRemain));
    const side = String(row.side).toLowerCase();
    const state = String(row.esiState).toLowerCase();
    const activityMatches =
      activity === "all" ||
      activity === side ||
      activity === state ||
      (activity === "sold" && side === "sell" && completed > 0) ||
      (activity === "bought" && side === "buy" && completed > 0);
    return (character === "all" || Number(character) === Number(row.characterId)) &&
      activityMatches &&
      (!search || String(row.itemName).toLowerCase().includes(search) || String(row.characterName).toLowerCase().includes(search)) &&
      (!location || String(row.locationName).toLowerCase().includes(location)) &&
      new Date(row.firstSeenAt).getTime() >= cutoff;
  });
}

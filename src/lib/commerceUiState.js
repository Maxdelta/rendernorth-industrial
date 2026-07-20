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

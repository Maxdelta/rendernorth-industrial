export function commerceTabAttributes(activeTab, candidateTab) {
  const active = activeTab === candidateTab;
  return { "aria-selected": active, tabIndex: active ? 0 : -1 };
}

export function switchCommerceTab(state, nextTab) {
  return { ...state, tab: nextTab };
}

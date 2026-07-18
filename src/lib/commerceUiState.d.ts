export type CommerceTab = "orders" | "contracts";
export interface CommerceFilterState<OrderFilters, ContractFilters> {
  tab: CommerceTab;
  orderFilters: OrderFilters;
  contractFilters: ContractFilters;
}
export function commerceTabAttributes(activeTab: CommerceTab, candidateTab: CommerceTab): { "aria-selected": boolean; tabIndex: number };
export function switchCommerceTab<OrderFilters, ContractFilters>(state: CommerceFilterState<OrderFilters, ContractFilters>, nextTab: CommerceTab): CommerceFilterState<OrderFilters, ContractFilters>;

export type CommerceTab = "orders" | "contracts";
export interface CommerceFilterState<OrderFilters, ContractFilters> {
  tab: CommerceTab;
  orderFilters: OrderFilters;
  contractFilters: ContractFilters;
}
export function commerceTabAttributes(activeTab: CommerceTab, candidateTab: CommerceTab): { "aria-selected": boolean; tabIndex: number };
export function switchCommerceTab<OrderFilters, ContractFilters>(state: CommerceFilterState<OrderFilters, ContractFilters>, nextTab: CommerceTab): CommerceFilterState<OrderFilters, ContractFilters>;
export type CommerceKpiFilterId = "orders-sell" | "orders-buy" | "orders-expiring" | "contracts-outstanding" | "contracts-assigned" | "contracts-issued" | "contracts-progress" | "contracts-expiring" | "contracts-finished";
export interface CommerceKpiState { tab: CommerceTab; side: string; direction: string; status: string; [key: string]: string; }
export const commerceKpiFilters: Readonly<Record<CommerceKpiFilterId, { tab: CommerceTab; field: "side" | "direction" | "status"; value: string; clearValue: string }>>;
export function commerceKpiActive(state: CommerceKpiState, filterId: CommerceKpiFilterId): boolean;
export function toggleCommerceKpiFilter(state: CommerceKpiState, filterId: CommerceKpiFilterId): CommerceKpiState;
export interface CommerceSyncCharacter { characterId: number; enabled: boolean; marketOrderScopeGranted: boolean; contractScopeGranted: boolean; }
export function commerceSyncAvailability(characters: CommerceSyncCharacter[], selectedCharacterId?: number | null): { marketOrders: boolean; contracts: boolean; commerce: boolean };
export function activeOrderQuantityLabel(row: { volumeRemain: number; volumeTotal: number }): string;
export function completedOrderActivityLabel(row: { side: string; volumeRemain: number; volumeTotal: number }): string;
export function filterCompletedOrders<T extends { characterId: number; characterName: string; itemName: string; locationName: string; side: string; esiState: string; volumeRemain: number; volumeTotal: number; firstSeenAt: string }>(rows: T[], filters: { character?: string | number; activity?: string; search?: string; location?: string; days?: number }, nowMs?: number): T[];

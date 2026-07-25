export type CommerceTab = "orders" | "contracts";
export interface CommerceFilterState<OrderFilters, ContractFilters> {
  tab: CommerceTab;
  orderFilters: OrderFilters;
  contractFilters: ContractFilters;
}
export function commerceTabAttributes(activeTab: CommerceTab, candidateTab: CommerceTab): { "aria-selected": boolean; tabIndex: number };
export function switchCommerceTab<OrderFilters, ContractFilters>(state: CommerceFilterState<OrderFilters, ContractFilters>, nextTab: CommerceTab): CommerceFilterState<OrderFilters, ContractFilters>;
export type CommerceKpiFilterId = "orders-sell" | "orders-buy" | "orders-expiring" | "contracts-outstanding" | "contracts-assigned" | "contracts-issued" | "contracts-accepted" | "contracts-progress" | "contracts-expiring" | "contracts-finished" | "contracts-cancelled" | "contracts-expired" | "contracts-new";
export interface CommerceKpiState { tab: CommerceTab; side: string; direction: string; status: string; [key: string]: string; }
export const commerceKpiFilters: Readonly<Record<CommerceKpiFilterId, { tab: CommerceTab; field: "side" | "direction" | "status" | "activity" | "readState"; value: string; clearValue: string }>>;
export function commerceKpiActive(state: CommerceKpiState, filterId: CommerceKpiFilterId): boolean;
export function toggleCommerceKpiFilter(state: CommerceKpiState, filterId: CommerceKpiFilterId): CommerceKpiState;
export interface CommerceSyncCharacter { characterId: number; enabled: boolean; marketOrderScopeGranted: boolean; contractScopeGranted: boolean; }
export function commerceSyncAvailability(characters: CommerceSyncCharacter[], selectedCharacterId?: number | null): { marketOrders: boolean; contracts: boolean; commerce: boolean };
export function activeOrderQuantityLabel(row: { volumeRemain: number; volumeTotal: number }): string;
export function completedOrderActivityLabel(row: { side: string; volumeRemain: number; volumeTotal: number }): string;
export function filterCompletedOrders<T extends { characterId: number; characterName: string; itemName: string; locationName: string; side: string; esiState: string; volumeRemain: number; volumeTotal: number; firstSeenAt: string }>(rows: T[], filters: { character?: string | number; activity?: string; search?: string; location?: string; days?: number }, nowMs?: number): T[];
export function filterContracts<T extends { characterId:number; characterName:string; title:string; direction:string; status:string; contractType:string; availability:string; startLocationId:number|null; startLocationName:string|null; endLocationId:number|null; endLocationName:string|null; remainingSeconds:number; activityCategory:string; seen:boolean; searchText?:string; dateIssued:string; dateCompleted:string|null; firstObservedAt:string }>(rows:T[], filters:{ view?:"active"|"activity"; character?:string|number; activity?:string; readState?:string; status?:string; direction?:string; contractType?:string; availability?:string; search?:string; startLocation?:string; endLocation?:string; thresholdDays?:number; days?:number },nowMs?:number):T[];
export function contractKpiCounts(rows:Array<{activityCategory:string;status:string;direction:string;remainingSeconds:number;seen:boolean}>,thresholdDays?:number):{outstanding:number;assigned:number;issued:number;accepted:number;completed:number;cancelled:number;expired:number;expiring:number;newActivity:number};
export function groupContractItems<T extends {recordId:number;typeId:number;itemName:string;quantity:number;singleton:boolean;included:boolean}>(items:T[]):Array<{typeId:number;itemName:string;quantity:number;singleton:boolean;included:boolean;recordIds:number[]}>;
export function contractTimeline(contract:{dateIssued:string;dateAccepted:string|null;dateCompleted:string|null;dateExpired:string;activityCategory:string}):Array<{label:string;timestamp:string}>;

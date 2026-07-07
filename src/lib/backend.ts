// The only file allowed to touch Tauri IPC. Everywhere else imports from here.
// In a plain browser (vite dev without the Rust shell) it falls back to mock data,
// so frontend work never requires the Rust toolchain.

export interface TierCoverage {
  label: string;
  coverage: number; // 0..1
}

export interface MissingMaterial {
  name: string;
  quantity: number;
  category: string;
}

export interface Recommendation {
  title: string;
  reason: string;
  ruleId: string;
  /** Machine-readable inputs the rule fired on (Determinism Doctrine). */
  inputs: Record<string, number | string> | null;
}

export interface SelectedTarget {
  projectId: number;
  name: string;
  className: string;
  overallProgress: number; // 0..1
  tiers: TierCoverage[]; // labels come from data — never hardcoded per hull
}

export interface BuildTargetSummary {
  projectId: number;
  name: string;
  className: string;
  status: string;
  overallProgress: number;
  isSelected: boolean;
}

export interface FactoryHealth {
  health: number; // 0..1
  status: string; // Blocked / Degraded / Operational (derived deterministically)
  idleSlots: number;
  blockedJobs: number;
  missingInputs: number;
  iskLockedInJobs: number;
  projectedFinishDays: number;
}

export interface MissionControl {
  source: "sqlite" | "mock";
  asOf: string;
  runningJobs: number;
  idleCharacters: number;
  idleBpos: number;
  walletIsk: number;
  factoryHealth: FactoryHealth;
  selectedTarget: SelectedTarget;
  missingMaterials: MissingMaterial[];
  recommendation: Recommendation;
}

export interface DbHealth {
  ok: boolean;
  schemaVersion: number;
  dbPath: string;
}

// ---------------------------------------------------------------------------
// Inventory Engine — the single data core. Every category is a view over
// one inventory; nothing here is specific to a ship, structure, or item.

export interface InventoryCategory {
  key: string;
  label: string;
  sortOrder: number;
  itemCount: number;
}

export interface InventoryItem {
  itemId: number;
  typeName: string;
  categoryKey: string;
  categoryLabel: string;
  quantity: number;
  locationName: string;
  ownerName: string;
  unitValue: number;
  totalValue: number;
  reservedQuantity: number;
  availableQuantity: number;
  /** Nets out soft allocations too — stricter than availableQuantity. */
  freeQuantity: number;
  allocatedOperation: string | null;
  reservedOperation: string | null;
  state: string;
  stateLabel: string;
  status: string;
}

export interface InventorySummary {
  totalAssets: number;
  estimatedValue: number;
  uniqueItemTypes: number;
  locations: number;
  reservedValue: number;
  availableValue: number;
}

// ---------------------------------------------------------------------------
// Operation domain — the object every other system revolves around. An
// Operation is industrial intent ("Build Avatar", "Manufacture 500 Capital
// Construction Parts"); it never owns inventory.

export interface OperationSummary {
  operationId: number;
  goal: string;
  targetTypeName: string | null;
  priority: number;
  status: string;
  progress: number;
  deadline: string | null;
  /** Derived from dependencies, never a hand-set literal. */
  isBlocked: boolean;
}

export interface OperationTimelineEntry {
  id: number;
  label: string;
  status: string;
  sortOrder: number;
  targetDate: string | null;
}

export interface OperationDependency {
  dependsOnOperationId: number;
  dependsOnGoal: string;
  dependsOnStatus: string;
  reason: string;
}

export interface OperationDetail {
  operationId: number;
  goal: string;
  targetTypeName: string | null;
  priority: number;
  status: string;
  progress: number;
  notes: string;
  deadline: string | null;
  isBlocked: boolean;
  dependencies: OperationDependency[];
  /** Owned by the Operation Engine; not rendered by the Workspace UI yet. */
  timeline: OperationTimelineEntry[];
}

export interface OperationHealth {
  totalOperations: number;
  activeOperations: number;
  blockedOperations: number;
  healthyFraction: number;
}

export interface OperationsDashboard {
  health: OperationHealth;
  currentOperations: OperationSummary[];
  priorityQueue: OperationSummary[];
  blocked: OperationSummary[];
  upcomingCompletions: OperationSummary[];
}

// ---------------------------------------------------------------------------
// Reservation domain — sits between the Inventory Engine and the Operation
// Engine. Inventory Engine owns what exists; Reservation Engine owns who
// owns it, reservation quantities, history, and conflicts. Read-only this
// sprint — nothing here creates, releases, or transfers a reservation yet.

export interface ReservationRecord {
  reservationId: number;
  itemId: number;
  itemName: string;
  operationId: number | null;
  operationGoal: string | null;
  quantity: number;
  reason: string;
  createdAt: string;
  releasedAt: string | null;
  isActive: boolean;
}

export interface ReservationEvent {
  id: number;
  reservationId: number;
  /** reserved / released / transferred / expired */
  eventType: string;
  quantity: number;
  fromOperationId: number | null;
  toOperationId: number | null;
  reason: string;
  createdAt: string;
}

export interface ReservationDetail {
  record: ReservationRecord;
  history: ReservationEvent[];
}

export interface ReservationConflict {
  itemId: number;
  itemName: string;
  /** overlapping_operations / exceeds_stock / missing_inventory */
  conflictType: string;
  description: string;
}

export interface ReservationSummary {
  activeReservationCount: number;
  totalReservedQuantity: number;
  itemsWithReservations: number;
  conflictCount: number;
}

/** Mission Control's Inventory Commitment panel. Every field is derived live — see the Rust doc comment for exact formulas. */
export interface InventoryCommitment {
  totalInventory: number;
  reserved: number;
  available: number;
  blocked: number;
  unallocated: number;
}

/** The Operations Workspace's Reservations section, scoped to one operation. */
export interface OperationReservations {
  reservedMinerals: number;
  reservedComponents: number;
  reservedPi: number;
  missingReservations: number;
}

// ---------------------------------------------------------------------------
// Blueprint domain — a sibling to Inventory, Operation, and Reservation.
// Blueprint Engine owns industrial capability: what blueprints exist,
// BPO/BPC, ME/TE, runs remaining, research/copy status, and whether an
// operation's required blueprints are actually on hand. Read-only this
// sprint — nothing here starts research, starts a copy, or acquires one.

export interface BlueprintRecord {
  blueprintId: number;
  typeName: string;
  isCopy: boolean;
  meLevel: number;
  teLevel: number;
  runsRemaining: number | null;
  ownerName: string;
  locationName: string;
  /** idle / researching / copying / in_use */
  status: string;
  linkedOperation: string | null;
}

export interface BlueprintDetail {
  record: BlueprintRecord;
  requiredBy: string[];
}

export interface BlueprintSummary {
  totalBlueprints: number;
  bpoCount: number;
  bpcCount: number;
  researchComplete: number;
  copies: number;
  missingForOperations: number;
}

export interface BlueprintRequirement {
  typeName: string;
  reason: string;
  isOwned: boolean;
  warningStatus: string | null;
}

export interface BlueprintReadiness {
  operationId: number;
  operationGoal: string;
  required: BlueprintRequirement[];
  ownedCount: number;
  missingCount: number;
  warningCount: number;
}

export interface MissingBlueprintReport {
  typeName: string;
  requiredByOperations: string[];
}

function inTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export async function getMissionControl(): Promise<MissionControl> {
  if (inTauri()) {
    try {
      return await invoke<MissionControl>("get_mission_control");
    } catch (err) {
      console.error("get_mission_control failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.getMockMissionControl();
}

export async function listBuildTargets(): Promise<BuildTargetSummary[]> {
  if (inTauri()) {
    try {
      return await invoke<BuildTargetSummary[]>("list_build_targets");
    } catch (err) {
      console.error("list_build_targets failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.listMockTargets();
}

export async function selectBuildTarget(projectId: number): Promise<MissionControl> {
  if (inTauri()) {
    try {
      return await invoke<MissionControl>("select_build_target", { projectId });
    } catch (err) {
      console.error("select_build_target failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/mock");
  return mock.selectMockTarget(projectId);
}

export async function getInventorySummary(): Promise<InventorySummary> {
  if (inTauri()) {
    try {
      return await invoke<InventorySummary>("get_inventory_summary");
    } catch (err) {
      console.error("get_inventory_summary failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/inventoryMock");
  return mock.getMockInventorySummary();
}

export async function listInventoryCategories(): Promise<InventoryCategory[]> {
  if (inTauri()) {
    try {
      return await invoke<InventoryCategory[]>("list_inventory_categories");
    } catch (err) {
      console.error("list_inventory_categories failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/inventoryMock");
  return mock.listMockInventoryCategories();
}

export async function listInventoryItems(categoryKey?: string): Promise<InventoryItem[]> {
  if (inTauri()) {
    try {
      return await invoke<InventoryItem[]>("list_inventory_items", { categoryKey: categoryKey ?? null });
    } catch (err) {
      console.error("list_inventory_items failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/inventoryMock");
  return mock.listMockInventoryItems(categoryKey);
}

export async function getOperationsDashboard(): Promise<OperationsDashboard> {
  if (inTauri()) {
    try {
      return await invoke<OperationsDashboard>("get_operations_dashboard");
    } catch (err) {
      console.error("get_operations_dashboard failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/operationMock");
  return mock.getMockOperationsDashboard();
}

export async function getOperationDetail(operationId: number): Promise<OperationDetail> {
  if (inTauri()) {
    try {
      return await invoke<OperationDetail>("get_operation_detail", { operationId });
    } catch (err) {
      console.error("get_operation_detail failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/operationMock");
  return mock.getMockOperationDetail(operationId);
}

export async function getInventoryCommitment(): Promise<InventoryCommitment> {
  if (inTauri()) {
    try {
      return await invoke<InventoryCommitment>("get_inventory_commitment");
    } catch (err) {
      console.error("get_inventory_commitment failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.getMockInventoryCommitment();
}

export async function getReservationSummary(): Promise<ReservationSummary> {
  if (inTauri()) {
    try {
      return await invoke<ReservationSummary>("get_reservation_summary");
    } catch (err) {
      console.error("get_reservation_summary failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.getMockReservationSummary();
}

export async function listActiveReservations(): Promise<ReservationRecord[]> {
  if (inTauri()) {
    try {
      return await invoke<ReservationRecord[]>("list_active_reservations");
    } catch (err) {
      console.error("list_active_reservations failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.listMockActiveReservations();
}

export async function getReservationDetail(reservationId: number): Promise<ReservationDetail> {
  if (inTauri()) {
    try {
      return await invoke<ReservationDetail>("get_reservation_detail", { reservationId });
    } catch (err) {
      console.error("get_reservation_detail failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.getMockReservationDetail(reservationId);
}

export async function getReservationConflicts(): Promise<ReservationConflict[]> {
  if (inTauri()) {
    try {
      return await invoke<ReservationConflict[]>("get_reservation_conflicts");
    } catch (err) {
      console.error("get_reservation_conflicts failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.getMockReservationConflicts();
}

export async function getOperationReservations(operationId: number): Promise<OperationReservations> {
  if (inTauri()) {
    try {
      return await invoke<OperationReservations>("get_operation_reservations", { operationId });
    } catch (err) {
      console.error("get_operation_reservations failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/reservationMock");
  return mock.getMockOperationReservations(operationId);
}

export async function getBlueprintSummary(): Promise<BlueprintSummary> {
  if (inTauri()) {
    try {
      return await invoke<BlueprintSummary>("get_blueprint_summary");
    } catch (err) {
      console.error("get_blueprint_summary failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.getMockBlueprintSummary();
}

export async function listBlueprints(): Promise<BlueprintRecord[]> {
  if (inTauri()) {
    try {
      return await invoke<BlueprintRecord[]>("list_blueprints");
    } catch (err) {
      console.error("list_blueprints failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.listMockBlueprints();
}

export async function getBlueprintDetail(blueprintId: number): Promise<BlueprintDetail> {
  if (inTauri()) {
    try {
      return await invoke<BlueprintDetail>("get_blueprint_detail", { blueprintId });
    } catch (err) {
      console.error("get_blueprint_detail failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.getMockBlueprintDetail(blueprintId);
}

export async function getMissingBlueprintReport(): Promise<MissingBlueprintReport[]> {
  if (inTauri()) {
    try {
      return await invoke<MissingBlueprintReport[]>("get_missing_blueprint_report");
    } catch (err) {
      console.error("get_missing_blueprint_report failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.getMockMissingBlueprintReport();
}

export async function getBlueprintReadinessAll(): Promise<BlueprintReadiness[]> {
  if (inTauri()) {
    try {
      return await invoke<BlueprintReadiness[]>("get_blueprint_readiness_all");
    } catch (err) {
      console.error("get_blueprint_readiness_all failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.getMockBlueprintReadinessAll();
}

export async function getBlueprintReadinessForOperation(operationId: number): Promise<BlueprintReadiness> {
  if (inTauri()) {
    try {
      return await invoke<BlueprintReadiness>("get_blueprint_readiness_for_operation", { operationId });
    } catch (err) {
      console.error("get_blueprint_readiness_for_operation failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/blueprintMock");
  return mock.getMockBlueprintReadinessForOperation(operationId);
}

export async function healthCheck(): Promise<DbHealth | null> {
  if (!inTauri()) return null;
  try {
    return await invoke<DbHealth>("health_check");
  } catch (err) {
    console.error("health_check failed:", err);
    return { ok: false, schemaVersion: -1, dbPath: "unavailable" };
  }
}

export function formatIsk(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1e12) return (value / 1e12).toFixed(1) + "T ISK";
  if (abs >= 1e9) return (value / 1e9).toFixed(1) + "B ISK";
  if (abs >= 1e6) return (value / 1e6).toFixed(1) + "M ISK";
  return value.toLocaleString() + " ISK";
}

export function formatQty(value: number): string {
  if (value >= 1_000_000) return (value / 1_000_000).toFixed(1).replace(/\.0$/, "") + "m";
  if (value >= 10_000) return Math.round(value / 1000) + "k";
  return value.toLocaleString();
}

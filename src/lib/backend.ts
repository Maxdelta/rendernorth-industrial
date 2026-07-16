// The only file allowed to touch Tauri IPC. Everywhere else imports from here.
// In a plain browser (vite dev without the Rust shell) it falls back to mock data,
// so frontend work never requires the Rust toolchain.
import { RELEASE_CATALOG } from "./releaseHistory";

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
  /** True for the Sprint 001–007 seeded scenario operations. */
  isDemo: boolean;
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
  isDemo: boolean;
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
  typeId: number | null;
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
  locationId: number | null;
  locationFlag: string | null;
  quantity: number;
  source: string;
  lastSynced: string | null;
  isOwned: boolean;
  resolvedLocation: ResolvedLocation | null;
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

// ---------------------------------------------------------------------------
// Production Requirement domain — a sibling to Inventory, Operation,
// Reservation, and Blueprint. Production Requirement Engine owns required
// inputs: what an operation actually needs to complete. Coverage and
// shortage are always derived live against inventory — read-only this
// sprint, nothing here declares or adjusts a requirement.

export interface RequirementLine {
  requirementId: number;
  operationId: number;
  operationGoal: string;
  categoryKey: string;
  categoryLabel: string;
  typeName: string;
  requiredQuantity: number;
  ownedQuantity: number;
  reservedForOperation: number;
  shortage: number;
  coverageFraction: number;
  isSatisfied: boolean;
}

export interface RequirementSource {
  sourceKind: string;
  contributedQuantity: number;
  note: string;
}

export interface RequirementDetail {
  line: RequirementLine;
  sources: RequirementSource[];
}

export interface RequirementCategory {
  categoryKey: string;
  categoryLabel: string;
  sortOrder: number;
}

export interface RequirementShortage {
  typeName: string;
  operationGoal: string;
  requiredQuantity: number;
  ownedQuantity: number;
  shortage: number;
}

export interface CriticalBottleneck {
  typeName: string;
  operationsRequiring: string[];
  ownedQuantity: number;
}

export interface RequirementSummary {
  totalRequirements: number;
  satisfied: number;
  missing: number;
  coveragePercent: number;
  criticalBottleneckCount: number;
}

export interface CategoryCoverage {
  categoryKey: string;
  categoryLabel: string;
  requiredTotal: number;
  ownedTotal: number;
  coverageFraction: number;
  missingCount: number;
}

export interface OperationRequirementBreakdown {
  operationId: number;
  operationGoal: string;
  categories: CategoryCoverage[];
  lines: RequirementLine[];
  totalRequirements: number;
  missingCount: number;
  coveragePercent: number;
}

// ---------------------------------------------------------------------------
// Static Data Import (Sprint 008) — official EVE reference data, loaded
// from a local directory the user selects. No network access, ever.

export interface ImportSummary {
  id: number;
  sourcePath: string;
  format: string;
  sourceBuild: string | null;
  importedAt: string;
  status: string; // success / partial / failed
  typeCount: number;
  groupCount: number;
  categoryCount: number;
  blueprintCount: number;
  materialCount: number;
  errorSummary: string | null;
}

export interface SdeFileStatus { name: string; present: boolean; }
export interface SdeInspection {
  path: string;
  valid: boolean;
  status: string;
  sourceBuild: string | null;
  files: SdeFileStatus[];
  error: string | null;
}

export interface AboutInfo {
  applicationName: string;
  version: string;
  build: string;
  gitCommit: string;
  databaseVersion: string;
  migrationVersion: number;
  rustVersion: string;
  website: string;
  github: string;
  issues: string;
  support: string;
  discordInvite: string;
  setupGuide: string;
  releaseStatus: string;
  discordUsername: string;
}

export interface TypeSearchResult {
  typeId: number;
  name: string;
  groupName: string;
  categoryName: string;
  isManufacturable: boolean;
  unitVolumeM3: number | null;
  producingBlueprintTypeId: number | null;
}

// ---------------------------------------------------------------------------
// Real operation creation (Sprint 008)

export type InventoryScope =
  | "build_location_only"
  | "same_solar_system"
  | "within_n_jumps"
  | "selected_locations"
  | "all_included_inventory";

export interface NewOperationInput {
  goal: string;
  priority: number;
  deadline?: string | null;
  notes?: string | null;
  typeId?: number | null;
  quantityRequested?: number | null;
  blueprintMode?: "owned" | "assumed" | null;
  ownedBlueprintId?: number | null;
  assumedMe?: number | null;
  assumedTe?: number | null;
  assumedIsBpc?: boolean | null;
  assumedRuns?: number | null;
  /** Captured now, not yet enforced — only "all_included_inventory" actually filters anything (i.e. nothing) today. */
  inventoryScope?: InventoryScope | null;
}

export interface CreatedOperation {
  operationId: number;
}

// ---------------------------------------------------------------------------
// Real, blueprint-derived production plan (Sprint 008) — distinct from the
// Sprint 007 demo-seeded RequirementLine/RequirementSummary above.

export interface RequirementTreeNode {
  typeId: number;
  typeName: string;
  isLeaf: boolean;
  runs: number;
  producedQuantity: number;
  neededQuantity: number;
  unitVolumeM3: number | null;
  requiredVolumeM3: number | null;
  perRunQuantity: number;
  ownedQuantity: number;
  ownedVolumeM3: number | null;
  /** Which inventory source contributed ownedQuantity — "Manual Inventory" today; demo inventory never contributes. */
  ownedSource: string;
  reservedQuantity: number;
  availableQuantity: number;
  missingQuantity: number;
  missingVolumeM3: number | null;
  coverageFraction: number;
  isSatisfied: boolean;
  meApplied: number;
  meSource: string;
  /** Validation Mode (Sprint 009) — the blueprint producing this node; null for leaves. */
  blueprintTypeId: number | null;
  /** Always "manufacturing" this sprint. */
  activity: string;
  /** Ancestor chain, e.g. "Apostle > Capital Armor Plates > Mechanical Parts" — engineering verification only. */
  calculationPath: string;
  children: RequirementTreeNode[];
}

export interface LeafTotal {
  typeId: number;
  typeName: string;
  groupName: string | null;
  categoryName: string | null;
  /** Validation Mode (Sprint 009) — every distinct ancestor chain that contributed to this leaf's total. */
  calculationPaths: string[];
  requiredQuantity: number;
  unitVolumeM3: number | null;
  requiredVolumeM3: number | null;
  ownedQuantity: number;
  ownedVolumeM3: number | null;
  ownedSource: string;
  reservedQuantity: number;
  availableQuantity: number;
  missingQuantity: number;
  missingVolumeM3: number | null;
  coverageFraction: number;
  isSatisfied: boolean;
}

export interface ProductionPlan {
  operationId: number;
  operationGoal: string;
  buildTargetTypeId: number;
  buildTargetName: string;
  requestedQuantity: number;
  blueprintMode: string;
  me: number;
  te: number;
  totalRuns: number;
  producedQuantity: number;
  /** Captured, not yet enforced — see NewOperationInput.inventoryScope. */
  inventoryScope: string;
  tree: RequirementTreeNode;
  leafTotals: LeafTotal[];
  totalRequiredVolumeM3: number | null;
  totalOwnedVolumeM3: number | null;
  totalMissingVolumeM3: number | null;
  warnings: string[];
  synchronizedBlueprints: Array<{itemId:number;ownerName:string;isCopy:boolean;me:number;te:number;runsRemaining:number|null;source:string}>;
}

// ---------------------------------------------------------------------------
// Manual inventory (Sprint 008) — real, user-entered stock.

export interface ManualInventoryEntry {
  id: number;
  typeId: number;
  typeName: string;
  quantity: number;
  unitVolumeM3: number | null;
  stackVolumeM3: number | null;
  locationName: string;
  createdAt: string;
  updatedAt: string;
}

export interface NewManualInventoryEntry {
  typeId: number;
  quantity: number;
  locationName?: string | null;
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

export async function getRequirementSummary(): Promise<RequirementSummary> {
  if (inTauri()) {
    try {
      return await invoke<RequirementSummary>("get_requirement_summary");
    } catch (err) {
      console.error("get_requirement_summary failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.getMockRequirementSummary();
}

export async function listRequirementLines(operationId?: number, categoryKey?: string): Promise<RequirementLine[]> {
  if (inTauri()) {
    try {
      return await invoke<RequirementLine[]>("list_requirement_lines", {
        operationId: operationId ?? null,
        categoryKey: categoryKey ?? null,
      });
    } catch (err) {
      console.error("list_requirement_lines failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.listMockRequirementLines(operationId, categoryKey);
}

export async function getRequirementDetail(requirementId: number): Promise<RequirementDetail> {
  if (inTauri()) {
    try {
      return await invoke<RequirementDetail>("get_requirement_detail", { requirementId });
    } catch (err) {
      console.error("get_requirement_detail failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.getMockRequirementDetail(requirementId);
}

export async function listRequirementCategories(): Promise<RequirementCategory[]> {
  if (inTauri()) {
    try {
      return await invoke<RequirementCategory[]>("list_requirement_categories");
    } catch (err) {
      console.error("list_requirement_categories failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.listMockRequirementCategories();
}

export async function getRequirementShortages(): Promise<RequirementShortage[]> {
  if (inTauri()) {
    try {
      return await invoke<RequirementShortage[]>("get_requirement_shortages");
    } catch (err) {
      console.error("get_requirement_shortages failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.getMockRequirementShortages();
}

export async function getCriticalBottlenecks(): Promise<CriticalBottleneck[]> {
  if (inTauri()) {
    try {
      return await invoke<CriticalBottleneck[]>("get_critical_bottlenecks");
    } catch (err) {
      console.error("get_critical_bottlenecks failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.getMockCriticalBottlenecks();
}

export async function getOperationRequirementBreakdown(operationId: number): Promise<OperationRequirementBreakdown> {
  if (inTauri()) {
    try {
      return await invoke<OperationRequirementBreakdown>("get_operation_requirement_breakdown", { operationId });
    } catch (err) {
      console.error("get_operation_requirement_breakdown failed, falling back to mock:", err);
    }
  }
  const mock = await import("../data/productionMock");
  return mock.getMockOperationRequirementBreakdown(operationId);
}

// ---------------------------------------------------------------------------
// Sprint 008: real, backend-only features. These have no demo/mock
// equivalent — importing your own static data, creating a real operation,
// and calculating a real plan only make sense against the actual local
// database. In browser preview (no Tauri shell) reads return empty/neutral
// results so the UI can show a clear "open the desktop app" state rather
// than crashing; mutations reject with an explicit error.

const BROWSER_PREVIEW_ERROR =
  "This feature reads and writes your local database and is only available in the RenderNorth Industrial desktop app.";

export async function importStaticData(dirPath: string): Promise<ImportSummary> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<ImportSummary>("import_static_data", { dirPath });
}

/** Official CCP JSONL SDE import (Sprint 008.2) — unverified against a real export in this environment; see docs/REAL_PRODUCTION_PLANNER.md. */
export async function importOfficialSde(dirPath: string): Promise<ImportSummary> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<ImportSummary>("import_official_sde", { dirPath });
}

export async function getLatestImport(): Promise<ImportSummary | null> {
  if (!inTauri()) return null;
  try {
    return await invoke<ImportSummary | null>("get_latest_import");
  } catch (err) {
    console.error("get_latest_import failed:", err);
    return null;
  }
}

export async function searchEveTypes(query: string, limit = 25): Promise<TypeSearchResult[]> {
  if (!inTauri()) return [];
  try {
    return await invoke<TypeSearchResult[]>("search_eve_types", { query, limit });
  } catch (err) {
    console.error("search_eve_types failed:", err);
    return [];
  }
}

export async function createRealOperation(input: NewOperationInput): Promise<CreatedOperation> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<CreatedOperation>("create_real_operation", { input });
}

/** Sprint 009.1 — refuses demo operations server-side; deletes the operation and every row it owns, transactionally. */
export async function deleteRealOperation(operationId: number): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("delete_real_operation", { operationId });
}

export async function calculateProductionPlan(operationId: number): Promise<ProductionPlan> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<ProductionPlan>("calculate_production_plan", { operationId });
}

export async function listManualInventory(): Promise<ManualInventoryEntry[]> {
  if (!inTauri()) return [];
  try {
    return await invoke<ManualInventoryEntry[]>("list_manual_inventory");
  } catch (err) {
    console.error("list_manual_inventory failed:", err);
    return [];
  }
}

export async function addManualInventoryEntry(input: NewManualInventoryEntry): Promise<number> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<number>("add_manual_inventory_entry", { input });
}

/** Paste Inventory bulk confirm (Sprint 008.2) — frontend has already parsed/matched/merged; this just persists. */
export async function addManualInventoryBulk(entries: NewManualInventoryEntry[]): Promise<number> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<number>("add_manual_inventory_bulk", { entries });
}

export async function updateManualInventoryQuantity(id: number, quantity: number): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("update_manual_inventory_quantity", { id, quantity });
}

export async function removeManualInventoryEntry(id: number): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("remove_manual_inventory_entry", { id });
}

// ---------------------------------------------------------------------------
// Generic app settings (Sprint 008.1) — reuses the existing app_meta
// key/value pattern. In browser preview (no Tauri shell) these fall back to
// an in-memory value for the session, since there is no local database to
// persist to; this is a disclosed limitation, not a silent one.

const browserSettingsFallback = new Map<string, string>();

export async function getAppSetting(key: string): Promise<string | null> {
  if (!inTauri()) return browserSettingsFallback.get(key) ?? null;
  try {
    return await invoke<string | null>("get_app_setting", { key });
  } catch (err) {
    console.error("get_app_setting failed:", err);
    return null;
  }
}

export async function setAppSetting(key: string, value: string): Promise<void> {
  if (!inTauri()) {
    browserSettingsFallback.set(key, value);
    return;
  }
  await invoke<void>("set_app_setting", { key, value });
}

export interface ReleaseViewState { currentVersion: string; lastViewedVersion: string | null; unseen: boolean; }
export type UpdateStatus = "not_checked" | "checking" | "up_to_date" | "update_available" | "check_failed" | "offline" | "skipped" | "disabled";
export type UpdateFrequency = "daily" | "weekly" | "never";
export interface UpdateState {
  installedVersion: string;
  latestVersion: string | null;
  latestReleaseTitle: string | null;
  releaseDate: string | null;
  releaseUrl: string | null;
  installerUrl: string | null;
  portableUrl: string | null;
  summary: string | null;
  lastChecked: string | null;
  status: UpdateStatus;
  lastError: string | null;
  skippedVersion: string | null;
  reminderUntil: string | null;
  releaseInCatalog: boolean;
}
export interface UpdatePreferences {
  automaticallyCheck: boolean;
  frequency: UpdateFrequency;
  includePrerelease: boolean;
}

export async function getReleaseViewState(): Promise<ReleaseViewState> {
  if (!inTauri()) {
    const currentVersion = RELEASE_CATALOG.publicVersion;
    const lastViewedVersion = browserSettingsFallback.get("release_notes_last_viewed") ?? null;
    return { currentVersion, lastViewedVersion, unseen: lastViewedVersion !== currentVersion };
  }
  return invoke<ReleaseViewState>("get_release_view_state");
}

export async function markCurrentReleaseViewed(): Promise<ReleaseViewState> {
  if (!inTauri()) {
    browserSettingsFallback.set("release_notes_last_viewed", RELEASE_CATALOG.publicVersion);
    return getReleaseViewState();
  }
  return invoke<ReleaseViewState>("mark_current_release_viewed");
}

const browserUpdateState: UpdateState = {
  installedVersion: RELEASE_CATALOG.publicVersion,
  latestVersion: RELEASE_CATALOG.publicVersion,
  latestReleaseTitle: null,
  releaseDate: null,
  releaseUrl: "https://github.com/Maxdelta/rendernorth-industrial/releases",
  installerUrl: null,
  portableUrl: null,
  summary: null,
  lastChecked: null,
  status: "not_checked",
  lastError: null,
  skippedVersion: null,
  reminderUntil: null,
  releaseInCatalog: true,
};

export async function getUpdateState(): Promise<UpdateState> {
  if (!inTauri()) return { ...browserUpdateState };
  return invoke<UpdateState>("get_update_state");
}

export async function getUpdatePreferences(): Promise<UpdatePreferences> {
  if (!inTauri()) return { automaticallyCheck: true, frequency: "daily", includePrerelease: RELEASE_CATALOG.releases[0]?.channel !== "Stable" };
  return invoke<UpdatePreferences>("get_update_preferences");
}

export async function saveUpdatePreferences(preferences: UpdatePreferences): Promise<UpdatePreferences> {
  if (!inTauri()) return preferences;
  return invoke<UpdatePreferences>("save_update_preferences", { preferences });
}

export async function checkForUpdates(manual = false): Promise<UpdateState> {
  if (!inTauri()) return { ...browserUpdateState, status: "up_to_date", lastChecked: new Date().toISOString() };
  return invoke<UpdateState>("check_for_updates", { manual });
}

export async function remindUpdateLater(): Promise<UpdateState> {
  if (!inTauri()) return { ...browserUpdateState, status: "skipped" };
  return invoke<UpdateState>("remind_update_later");
}

export async function skipUpdateVersion(): Promise<UpdateState> {
  if (!inTauri()) return { ...browserUpdateState, status: "skipped", skippedVersion: browserUpdateState.latestVersion };
  return invoke<UpdateState>("skip_update_version");
}

export async function clearSkippedUpdate(): Promise<UpdateState> {
  if (!inTauri()) return { ...browserUpdateState, skippedVersion: null };
  return invoke<UpdateState>("clear_skipped_update");
}

export async function inspectSdeDirectory(dirPath: string): Promise<SdeInspection> {
  if (!inTauri()) {
    return { path: dirPath, valid: false, status: "browser_preview", sourceBuild: null, files: [], error: BROWSER_PREVIEW_ERROR };
  }
  return invoke<SdeInspection>("inspect_sde_directory", { dirPath });
}

export async function pickSdeDirectory(): Promise<string | null> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<string | null>("pick_sde_directory");
}

export interface AuthenticationInfo { mode: "official" | "custom"; clientId: string; applicationName: string; }

export async function pickSdeArchive(): Promise<string | null> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<string | null>("pick_sde_archive");
}

export async function openSelectedFolder(path: string): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("open_selected_folder", { path });
}

export async function getAboutInfo(): Promise<AboutInfo> {
  if (!inTauri()) {
    return { applicationName: "RenderNorth Industrial", version: RELEASE_CATALOG.publicVersion, build: "development", gitCommit: "unavailable", databaseVersion: "unavailable", migrationVersion: 0, rustVersion: "unavailable", website: "https://rendernorth.com", github: "https://github.com/Maxdelta/rendernorth-industrial", issues: "https://github.com/Maxdelta/rendernorth-industrial/issues", support: "https://buymeacoffee.com/maxdelta", discordInvite: "https://discord.gg/XycCz6ppx", setupGuide: "https://github.com/Maxdelta/rendernorth-industrial/blob/main/docs/OPEN_BETA_ONBOARDING.md", releaseStatus: "Open Beta 0.1.1", discordUsername: "maxdelta0089" };
  }
  return invoke<AboutInfo>("get_about_info");
}

export async function getAuthenticationInfo(): Promise<AuthenticationInfo> {
  if (!inTauri()) return { mode: "official", clientId: "f6321a78ea0e4ed78fc52ab2ba85d502", applicationName: "Official RenderNorth Industrial" };
  return invoke<AuthenticationInfo>("get_authentication_info");
}

export async function saveCustomAuthentication(clientId: string): Promise<AuthenticationInfo> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<AuthenticationInfo>("save_custom_authentication", { clientId });
}

export async function restoreOfficialAuthenticationConfig(): Promise<AuthenticationInfo> {
  if (!inTauri()) return getAuthenticationInfo();
  return invoke<AuthenticationInfo>("restore_official_authentication");
}

export async function exportDiagnostics(): Promise<string | null> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<string | null>("export_diagnostics");
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!inTauri()) { window.open(url, "_blank", "noopener,noreferrer"); return; }
  return invoke<void>("open_external_url", { url });
}

export async function setApplicationSectionTitle(section: string): Promise<void> {
  document.title = `RenderNorth Industrial — ${section}`;
  if (!inTauri()) return;
  return invoke<void>("set_application_section", { section });
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

export function formatVolume(value: number | null): string {
  return value == null
    ? "Unknown m³"
    : `${value.toLocaleString(undefined, { maximumFractionDigits: 6 })} m³`;
}

// ---------------------------------------------------------------------------
// Sprint 011A — EVE SSO Character Authentication Foundation. Authentication
// only. See docs/ESI_INTEGRATION.md and docs/SPRINT_011A_SETUP.md.
// ---------------------------------------------------------------------------

export interface CharacterSummary {
  characterId: number;
  name: string;
  enabled: boolean;
  /** "authorized" | "expired" | "revoked" */
  authorizationStatus: string;
  lastLoginAt: string | null;
  assetScopeGranted: boolean;
  syncStatus: string;
  lastSyncAt: string | null;
  assetCount: number;
  pageCount: number;
  syncError: string | null;
  blueprintScopeGranted: boolean;
  blueprintSyncStatus: string;
  blueprintLastSyncAt: string | null;
  blueprintCount: number;
  blueprintPageCount: number;
  blueprintSyncError: string | null;
  structureScopeGranted: boolean;
}

export interface AssetSyncResult { characterId: number; status: string; assetCount: number; pageCount: number; error: string | null; }
export interface ResolvedLocation { rawLocationId:number; displayName:string; locationKind:string; solarSystemName:string|null; constellationName:string|null; regionName:string|null; containerPath:string[]; fullPath:string[]; resolutionStatus:string; resolutionSource:string; lastResolved:string|null; }
export interface SyncedAsset {
  characterId: number; characterOwner: string; typeId: number; typeName: string; quantity: number;
  unitVolumeM3:number|null; stackVolumeM3:number|null;
  itemId: number; locationId: number; locationType: string; locationFlag: string; singleton: boolean;
  source: string; lastSynced: string; resolvedLocation: ResolvedLocation;
}
export interface LocationRefreshResult { resolved:number; inaccessible:number; errors:string[]; }
export interface WeightedFill { unitPrice:number|null; totalPrice:number|null; requested:number; filled:number; availableVolume:number; sufficient:boolean; }
export interface MarketQuote { typeId:number; quantity:number; unitVolumeM3:number|null; requestedVolumeM3:number|null; acquisition:WeightedFill; liquidation:WeightedFill; marketProfile:string; fetchedAt:string|null; expiresAt:string|null; stale:boolean; source:string; }
export interface MarketProfile { profileId:number; displayName:string; regionId:number; locationId:number; refreshIntervalSeconds:number; status:string; fetchedAt:string|null; expiresAt:string|null; orderCount:number; pageCount:number; lastError:string|null; stale:boolean; }
export interface InventorySearchInput { text?:string|null; character?:string|null; source?:string|null; category?:string|null; group?:string|null; region?:string|null; system?:string|null; location?:string|null; resolved?:boolean|null; positiveOnly?:boolean|null; sort?:string|null; }
export interface InventorySearchRow { stackKey:string; typeId:number; typeName:string; quantity:number; unitVolumeM3:number|null; stackVolumeM3:number|null; owner:string; source:string; groupName:string|null; categoryName:string|null; locationName:string; systemName:string|null; constellationName:string|null; regionName:string|null; containerPath:string[]; locationFlag:string|null; resolved:boolean; lastSynced:string; quote:MarketQuote; }
export interface InventorySearchResult { rows:InventorySearchRow[]; matchingStacks:number; matchingUnits:number; totalVolumeM3:number|null; totalReplacementValue:number|null; totalLiquidationValue:number|null; }
export interface CostAssumptions { salesTaxPercent:number; brokerFeePercent:number; manufacturingJobCost:number; haulingCost:number; otherCost:number; }
export interface CostAssumptionState { assumptions:CostAssumptions; savedAt:string|null; }
export type RevenueBasis = "immediate" | "listed";
export interface MaterialCostLine { typeId:number; typeName:string; requiredQuantity:number; ownedQuantity:number; shortageQuantity:number; unitVolumeM3:number|null; requiredVolumeM3:number|null; ownedVolumeM3:number|null; purchaseVolumeM3:number|null; unitAcquisitionPrice:number|null; replacementValue:number|null; ownedOpportunityCost:number|null; shortageCashCost:number|null; availableVolume:number; }
export interface OperationEconomics { operationId:number; marketProfile:string; materials:MaterialCostLine[]; totalMaterialReplacementCost:number|null; ownedMaterialOpportunityCost:number|null; cashRequired:number|null; totalMaterialVolumeM3:number|null; totalPurchaseVolumeM3:number|null; outputImmediateSaleValue:number|null; outputListedSaleValue:number|null; revenueBasis:RevenueBasis; revenueBasisMessage:string|null; grossProfit:number|null; salesTax:number|null; brokerFee:number|null; manufacturingJobCost:number; haulingCost:number; otherCost:number; estimatedNetProfit:number|null; margin:number|null; roi:number|null; stale:boolean; priceTimestamp:string|null; }
export type ProcurementStatus = "Needed" | "Planned" | "Purchased" | "Skipped" | "Fulfilled Manually";
export interface ShoppingLine { typeId:number; itemName:string; requiredQuantity:number; ownedQuantity:number; shortageQuantity:number; unitVolumeM3:number|null; totalVolumeM3:number|null; acquisitionUnitPrice:number|null; totalAcquisitionCost:number|null; availableMarketVolume:number; marketStatus:string; priceTimestamp:string|null; procurementStatus:ProcurementStatus; notes:string; lastUpdated:string|null; }
export interface ShoppingSummary { missingItemTypes:number; totalMissingUnits:number; totalPurchaseCost:number|null; totalPurchaseVolumeM3:number|null; pricedLines:number; unpricedLines:number; insufficientVolumeLines:number; }
export interface ShoppingExports { eveMultiBuy:string; discordReport:string; csv:string; }
export interface OperationShoppingList { operationId:number; operationName:string; marketProfile:string; lines:ShoppingLine[]; summary:ShoppingSummary; exports:ShoppingExports; }
export interface ProcurementState { operationId:number; typeId:number; status:ProcurementStatus; notes:string; updatedAt:string; }
export interface DoctrineInput { name:string; description:string; category:string; fleetNotes:string; isActive:boolean; version:number; }
export interface Doctrine { doctrineId:number; name:string; description:string; category:string; fleetNotes:string; isActive:boolean; version:number; fitCount:number; updatedAt:string; }
export interface DoctrineFitItem { typeId:number; itemName:string; itemKind:string; quantity:number; }
export interface DoctrineFit { fitId:number; doctrineId:number; name:string; hullTypeId:number; hullName:string; desiredQuantity:number; originalEftText:string; sourceName:string; items:DoctrineFitItem[]; }
export interface FitReadiness { fitId:number; name:string; hullName:string; desired:number; readyNow:number; readyAfterBuild:number; stillMissing:number; coveragePercent:number; status:string; }
export interface DoctrineItem { typeId:number; itemName:string; itemKind:string; required:number; owned:number; reserved:number; available:number; missing:number; manufacturable:boolean; ownedBlueprint:boolean; action:string; unitVolumeM3:number|null; totalMissingVolumeM3:number|null; purchaseUnitPrice:number|null; purchaseCost:number|null; marketStatus:string; blockedReason:string|null; blockedDetail:string|null; }
export interface BuildMaterialDetail { typeId:number; itemName:string; requiredQuantity:number; ownedQuantity:number; missingQuantity:number; unitVolumeM3:number|null; missingVolumeM3:number|null; acquisitionUnitPrice:number|null; totalAcquisitionCost:number|null; marketStatus:string; }
export interface BuildDetail { typeId:number; itemName:string; missingQuantity:number; manufacturingRuns:number; outputQuantity:number; blueprintSource:string|null; blueprintOwner:string|null; me:number|null; te:number|null; rawMaterialCost:number|null; rawMaterialVolumeM3:number|null; missingBlueprintWarning:string|null; unmanufacturableWarning:string|null; warnings:string[]; rawMaterials:BuildMaterialDetail[]; }
export interface ReasonCount { reason:string; count:number; }
export interface ActionSummary { readyItemTypes:number; buildItemTypes:number; buyItemTypes:number; blockedItemTypes:number; missingBlueprints:number; unpricedLines:number; insufficientVolumeLines:number; }
export interface QuartermasterExports { buyFinishedGoods:ShoppingExports; buyManufacturingInputs:ShoppingExports; combinedPurchaseList:ShoppingExports; }
export interface QuartermasterSummary { buyCost:number; buyVolumeM3:number; buildInputCost:number; buildInputVolumeM3:number; totalCompletionCost:number; totalHaulingVolumeM3:number; totalsComplete:boolean; excludedLines:number; readyNow:number; readyAfterBuild:number; stillMissingAfterBuild:number; target:number; coverageNowPercent:number; coverageAfterBuildPercent:number; overallStatus:string; canFullyField:boolean; blockingComponents:string[]; shoppingReady:boolean; productionReady:boolean; actionSummary:ActionSummary; blockedReasons:ReasonCount[]; partialReasons:ReasonCount[]; }
export interface DoctrineAnalysis { doctrine:Doctrine; fits:FitReadiness[]; items:DoctrineItem[]; buildDetails:BuildDetail[]; buyLines:ShoppingLine[]; buildInputLines:ShoppingLine[]; summary:QuartermasterSummary; shopping:QuartermasterExports; }
export interface BlueprintSyncResult { characterId:number; status:string; blueprintCount:number; pageCount:number; error:string|null; }

/** Blocking on the Rust side (opens the browser, waits on the OAuth callback) — this call can take up to 3 minutes. */
export async function addCharacter(clientId: string): Promise<CharacterSummary> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<CharacterSummary>("add_character", { clientId });
}

export async function removeCharacter(characterId: number): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("remove_character", { characterId });
}

export async function setCharacterEnabled(characterId: number, enabled: boolean): Promise<void> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<void>("set_character_enabled", { characterId, enabled });
}

export async function listCharacters(): Promise<CharacterSummary[]> {
  if (!inTauri()) return [];
  return invoke<CharacterSummary[]>("list_characters");
}

export async function syncCharacterAssets(clientId: string, characterId: number): Promise<AssetSyncResult> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<AssetSyncResult>("sync_character_assets", { clientId, characterId });
}

export async function syncAllCharacterAssets(clientId: string): Promise<AssetSyncResult[]> {
  if (!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<AssetSyncResult[]>("sync_all_character_assets", { clientId });
}

export async function listSyncedAssets(): Promise<SyncedAsset[]> {
  if (!inTauri()) return [];
  return invoke<SyncedAsset[]>("list_synced_assets");
}
export async function refreshAssetLocations(clientId:string):Promise<LocationRefreshResult>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<LocationRefreshResult>("refresh_asset_locations",{clientId});}
export async function getMarketProfile():Promise<MarketProfile>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<MarketProfile>("get_market_profile");}
export async function refreshMarketPrices():Promise<{orderCount:number;pageCount:number;fetchedAt:string;expiresAt:string}>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke("refresh_market_prices");}
export async function searchInventoryMarket(input:InventorySearchInput):Promise<InventorySearchResult>{if(!inTauri())return {rows:[],matchingStacks:0,matchingUnits:0,totalVolumeM3:null,totalReplacementValue:null,totalLiquidationValue:null};return invoke<InventorySearchResult>("search_inventory_market",{input});}
export async function getMarketQuote(typeId:number,quantity:number):Promise<MarketQuote>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<MarketQuote>("get_market_quote",{typeId,quantity});}
export async function searchMarketTypes(query:string,limit=25):Promise<TypeSearchResult[]>{if(!inTauri())return [];return invoke<TypeSearchResult[]>("search_eve_types",{query,limit});}
export async function getOperationCostAssumptions(operationId:number):Promise<CostAssumptionState>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<CostAssumptionState>("get_operation_cost_assumptions",{operationId});}
export async function saveOperationCostAssumptions(operationId:number,assumptions:CostAssumptions):Promise<CostAssumptionState>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<CostAssumptionState>("save_operation_cost_assumptions",{operationId,assumptions});}
export async function resetOperationCostAssumptions(operationId:number):Promise<CostAssumptionState>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<CostAssumptionState>("reset_operation_cost_assumptions",{operationId});}
export async function getOperationEconomics(operationId:number,revenueBasis:RevenueBasis|null=null):Promise<OperationEconomics>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<OperationEconomics>("get_operation_economics",{operationId,revenueBasis});}
export async function getOperationShoppingList(operationId:number):Promise<OperationShoppingList>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<OperationShoppingList>("get_operation_shopping_list",{operationId});}
export async function updateProcurementLine(operationId:number,typeId:number,status:ProcurementStatus,notes:string):Promise<ProcurementState>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<ProcurementState>("update_procurement_line",{operationId,typeId,status,notes});}
export async function resetProcurementState(operationId:number):Promise<number>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<number>("reset_procurement_state",{operationId});}
export async function listDoctrines():Promise<Doctrine[]>{if(!inTauri())return [];return invoke<Doctrine[]>("list_doctrines");}
export async function createDoctrine(input:DoctrineInput):Promise<Doctrine>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<Doctrine>("create_doctrine",{input});}
export async function updateDoctrine(doctrineId:number,input:DoctrineInput):Promise<Doctrine>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<Doctrine>("update_doctrine",{doctrineId,input});}
export async function listDoctrineFits(doctrineId:number):Promise<DoctrineFit[]>{if(!inTauri())return [];return invoke<DoctrineFit[]>("list_doctrine_fits",{doctrineId});}
export async function importDoctrineEftText(doctrineId:number,text:string,sourceName:string):Promise<DoctrineFit>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<DoctrineFit>("import_doctrine_eft_text",{doctrineId,text,sourceName});}
export async function importDoctrineEftFile(doctrineId:number,path:string):Promise<DoctrineFit>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<DoctrineFit>("import_doctrine_eft_file",{doctrineId,path});}
export async function importDoctrineEftFolder(doctrineId:number,path:string):Promise<DoctrineFit[]>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<DoctrineFit[]>("import_doctrine_eft_folder",{doctrineId,path});}
export async function setDoctrineFitQuantity(fitId:number,quantity:number):Promise<DoctrineFit>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<DoctrineFit>("set_doctrine_fit_quantity",{fitId,quantity});}
export async function deleteDoctrineFit(fitId:number):Promise<void>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<void>("delete_doctrine_fit",{fitId});}
export async function analyzeDoctrine(doctrineId:number):Promise<DoctrineAnalysis>{if(!inTauri())throw new Error(BROWSER_PREVIEW_ERROR);return invoke<DoctrineAnalysis>("analyze_doctrine",{doctrineId});}

export async function syncCharacterBlueprints(clientId:string,characterId:number):Promise<BlueprintSyncResult>{
  if(!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<BlueprintSyncResult>("sync_character_blueprints",{clientId,characterId});
}
export async function syncAllCharacterBlueprints(clientId:string):Promise<BlueprintSyncResult[]>{
  if(!inTauri()) throw new Error(BROWSER_PREVIEW_ERROR);
  return invoke<BlueprintSyncResult[]>("sync_all_character_blueprints",{clientId});
}

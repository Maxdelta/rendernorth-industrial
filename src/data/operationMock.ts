// Sprint 004 demo dataset — browser-mode fallback for the Operation Engine.
// Mirrors the seed rows in migration 0004 and the derivation logic in
// src-tauri/src/operation/repository.rs (is_blocked, health), so browser
// mode and the Tauri shell show the same shape of data.
import type {
  OperationDependency,
  OperationDetail,
  OperationHealth,
  OperationsDashboard,
  OperationSummary,
  OperationTimelineEntry,
} from "../lib/backend";

interface RawOperation {
  operationId: number;
  goal: string;
  targetTypeName: string | null;
  priority: number;
  status: string;
  progress: number;
  notes: string;
  deadline: string | null;
}

interface RawDependency {
  operationId: number;
  dependsOnOperationId: number;
  reason: string;
}

interface RawTimelineEntry extends OperationTimelineEntry {
  operationId: number;
}

const OPERATIONS: RawOperation[] = [
  { operationId: 1, goal: "Build Avatar", targetTypeName: "Avatar (Titan — Amarr capital hull)", priority: 1, status: "active", progress: 0.72, notes: "Primary capital project. Waiting on capital component and PI supply lines.", deadline: "2026-09-01" },
  { operationId: 2, goal: "Build Navy Revelation", targetTypeName: "Navy Revelation (Dreadnought — Amarr faction capital)", priority: 2, status: "planned", progress: 0.28, notes: "Faction hull; LP store dependency for Purple Loyalty Tokens.", deadline: "2026-11-15" },
  { operationId: 3, goal: "Build Apostle", targetTypeName: "Apostle (Force Auxiliary — Amarr capital)", priority: 3, status: "planned", progress: 0.52, notes: "Secondary support hull. Not yet resourced ahead of Avatar.", deadline: null },
  { operationId: 4, goal: "Manufacture Capital Construction Parts", targetTypeName: "Capital Construction Parts (Capital component)", priority: 1, status: "active", progress: 0.95, notes: "Feeds Avatar directly. Near complete on this run.", deadline: "2026-07-20" },
  { operationId: 5, goal: "Manufacture Broadcast Nodes", targetTypeName: "Broadcast Node (PI — P4 advanced commodity)", priority: 2, status: "planned", progress: 0.68, notes: "PI chain restock in progress; feeds Avatar's advanced components.", deadline: "2026-07-10" },
  { operationId: 6, goal: "Prepare Titan Components", targetTypeName: null, priority: 4, status: "planned", progress: 0.10, notes: "Cross-hull staging operation consolidating capital component stockpiles ahead of the next Titan-class build. Not tied to a single build target.", deadline: null },
];

const DEPENDENCIES: RawDependency[] = [
  { operationId: 1, dependsOnOperationId: 4, reason: "Avatar cannot complete without full Capital Construction Parts coverage" },
  { operationId: 1, dependsOnOperationId: 5, reason: "Advanced component line needs the Broadcast Node PI chain" },
  { operationId: 6, dependsOnOperationId: 4, reason: "Shares the Capital Construction Parts supply with Avatar" },
];

const TIMELINE: RawTimelineEntry[] = [
  { operationId: 1, id: 1, label: "Mineral stockpile complete", status: "done", sortOrder: 1, targetDate: "2026-05-10" },
  { operationId: 1, id: 2, label: "Capital components online", status: "in_progress", sortOrder: 2, targetDate: "2026-08-01" },
  { operationId: 1, id: 3, label: "Hull assembly begins", status: "pending", sortOrder: 3, targetDate: "2026-09-01" },
  { operationId: 2, id: 4, label: "LP store token accumulation", status: "in_progress", sortOrder: 1, targetDate: "2026-09-30" },
  { operationId: 2, id: 5, label: "Faction component sourcing", status: "pending", sortOrder: 2, targetDate: "2026-11-15" },
  { operationId: 3, id: 6, label: "Mineral coverage review", status: "in_progress", sortOrder: 1, targetDate: null },
  { operationId: 4, id: 7, label: "Blueprint research complete", status: "done", sortOrder: 1, targetDate: "2026-06-01" },
  { operationId: 4, id: 8, label: "Final production run", status: "in_progress", sortOrder: 2, targetDate: "2026-07-20" },
  { operationId: 5, id: 9, label: "P3 input restock", status: "in_progress", sortOrder: 1, targetDate: "2026-07-05" },
  { operationId: 6, id: 10, label: "Component stockpile survey", status: "pending", sortOrder: 1, targetDate: null },
];

/** Mirrors OperationRepository's IS_BLOCKED_EXPR: own status, or any incomplete dependency. */
function isBlocked(op: RawOperation): boolean {
  if (op.status === "blocked") return true;
  return DEPENDENCIES.filter((d) => d.operationId === op.operationId).some((d) => {
    const dep = OPERATIONS.find((o) => o.operationId === d.dependsOnOperationId);
    return dep ? dep.status !== "completed" : false;
  });
}

function toSummary(op: RawOperation): OperationSummary {
  return {
    operationId: op.operationId,
    goal: op.goal,
    targetTypeName: op.targetTypeName,
    priority: op.priority,
    status: op.status,
    progress: op.progress,
    deadline: op.deadline,
    isBlocked: isBlocked(op),
    isDemo: true,
  };
}

export function getMockOperationsDashboard(): OperationsDashboard {
  const summaries = [...OPERATIONS].sort((a, b) => a.priority - b.priority || a.operationId - b.operationId).map(toSummary);

  const total = OPERATIONS.length;
  const active = OPERATIONS.filter((o) => o.status === "active").length;
  const blockedCount = summaries.filter((s) => s.isBlocked).length;
  const health: OperationHealth = {
    totalOperations: total,
    activeOperations: active,
    blockedOperations: blockedCount,
    healthyFraction: total > 0 ? (total - blockedCount) / total : 0,
  };

  const priorityQueue = summaries.filter((s) => s.status !== "completed");
  const blocked = summaries.filter((s) => s.isBlocked);
  const upcomingCompletions = summaries
    .filter((s) => s.deadline !== null && s.status !== "completed")
    .sort((a, b) => (a.deadline! < b.deadline! ? -1 : 1))
    .slice(0, 5);

  return {
    health,
    currentOperations: summaries,
    priorityQueue,
    blocked,
    upcomingCompletions,
  };
}

export function getMockOperationDetail(operationId: number): OperationDetail {
  const op = OPERATIONS.find((o) => o.operationId === operationId);
  if (!op) throw new Error(`unknown operation: ${operationId}`);

  const dependencies: OperationDependency[] = DEPENDENCIES.filter((d) => d.operationId === operationId).map((d) => {
    const dep = OPERATIONS.find((o) => o.operationId === d.dependsOnOperationId)!;
    return {
      dependsOnOperationId: d.dependsOnOperationId,
      dependsOnGoal: dep.goal,
      dependsOnStatus: dep.status,
      reason: d.reason,
    };
  });

  const timeline = TIMELINE.filter((t) => t.operationId === operationId)
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map(({ id, label, status, sortOrder, targetDate }) => ({ id, label, status, sortOrder, targetDate }));

  return {
    operationId: op.operationId,
    goal: op.goal,
    targetTypeName: op.targetTypeName,
    priority: op.priority,
    status: op.status,
    progress: op.progress,
    notes: op.notes,
    deadline: op.deadline,
    isBlocked: isBlocked(op),
    isDemo: true,
    dependencies,
    timeline,
  };
}

export function listMockOperationSummaries(): OperationSummary[] {
  return [...OPERATIONS].sort((a, b) => a.priority - b.priority || a.operationId - b.operationId).map(toSummary);
}

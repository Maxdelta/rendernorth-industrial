// Sprint 006 demo dataset — browser-mode fallback for the Blueprint
// Engine. Mirrors migration 0006's seed and the derivation logic in
// src-tauri/src/blueprint/repository.rs (missing-report, readiness), so
// browser mode and the Tauri shell show the same shape.
import type {
  BlueprintDetail,
  BlueprintReadiness,
  BlueprintRecord,
  BlueprintRequirement,
  BlueprintSummary,
  MissingBlueprintReport,
} from "../lib/backend";

interface RawBlueprint {
  blueprintId: number;
  typeName: string;
  isCopy: boolean;
  meLevel: number;
  teLevel: number;
  runsRemaining: number | null;
  ownerName: string;
  locationName: string;
  status: string;
}

interface RawRequirement {
  operationId: number;
  operationGoal: string;
  typeName: string;
  reason: string;
}

const BLUEPRINTS: RawBlueprint[] = [
  { blueprintId: 1, typeName: "Avatar Blueprint Copy", isCopy: true, meLevel: 10, teLevel: 20, runsRemaining: 1, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "idle" },
  { blueprintId: 2, typeName: "Capital Construction Parts Blueprint", isCopy: false, meLevel: 8, teLevel: 12, runsRemaining: null, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "idle" },
  { blueprintId: 3, typeName: "Capital Armor Plates Blueprint", isCopy: false, meLevel: 6, teLevel: 8, runsRemaining: null, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "researching" },
  { blueprintId: 4, typeName: "Broadcast Node Blueprint Copy", isCopy: true, meLevel: 0, teLevel: 0, runsRemaining: 10, ownerName: "Demo Hauler", locationName: "Nakugard - Home POS Silo", status: "idle" },
  { blueprintId: 5, typeName: "Broadcast Node Blueprint Copy", isCopy: true, meLevel: 2, teLevel: 4, runsRemaining: 5, ownerName: "Demo Hauler", locationName: "Nakugard - Home POS Silo", status: "idle" },
  { blueprintId: 6, typeName: "Navy Revelation Blueprint", isCopy: false, meLevel: 4, teLevel: 6, runsRemaining: null, ownerName: "Demo Forgemaster", locationName: "Amarr VIII (Oris) - Emperor Family Academy", status: "idle" },
  { blueprintId: 7, typeName: "Apostle Blueprint Copy", isCopy: true, meLevel: 0, teLevel: 0, runsRemaining: 3, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "copying" },
  { blueprintId: 8, typeName: "Capital Capacitor Batteries Blueprint", isCopy: false, meLevel: 10, teLevel: 10, runsRemaining: null, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "idle" },
  { blueprintId: 9, typeName: "Auto-Integrity Preservation Seal Blueprint", isCopy: false, meLevel: 5, teLevel: 5, runsRemaining: null, ownerName: "Demo Forgemaster", locationName: "Egghelende - RenderNorth Forge (Keepstar)", status: "idle" },
  { blueprintId: 10, typeName: "Capital Shield Extender II Blueprint", isCopy: false, meLevel: 3, teLevel: 3, runsRemaining: null, ownerName: "Demo Hauler", locationName: "1DQ1-A - Sotiyo (corp staging)", status: "idle" },
];

const REQUIREMENTS: RawRequirement[] = [
  { operationId: 1, operationGoal: "Build Avatar", typeName: "Avatar Blueprint Copy", reason: "Primary hull blueprint for the Avatar build" },
  { operationId: 1, operationGoal: "Build Avatar", typeName: "Capital Construction Parts Blueprint", reason: "Feeds the capital component line for Avatar" },
  { operationId: 1, operationGoal: "Build Avatar", typeName: "Capital Jump Drive Blueprint", reason: "Jump drive component not yet blueprinted" },
  { operationId: 2, operationGoal: "Build Navy Revelation", typeName: "Navy Revelation Blueprint", reason: "Primary hull blueprint for the Navy Revelation build" },
  { operationId: 2, operationGoal: "Build Navy Revelation", typeName: "Capital Armor Plates Blueprint", reason: "Faction armor tier component supply" },
  { operationId: 3, operationGoal: "Build Apostle", typeName: "Apostle Blueprint Copy", reason: "Primary hull blueprint for the Apostle build" },
  { operationId: 3, operationGoal: "Build Apostle", typeName: "Capital Capacitor Batteries Blueprint", reason: "Capacitor component supply for Apostle" },
  { operationId: 4, operationGoal: "Manufacture Capital Construction Parts", typeName: "Capital Construction Parts Blueprint", reason: "Direct production blueprint for this run" },
  { operationId: 5, operationGoal: "Manufacture Broadcast Nodes", typeName: "Broadcast Node Blueprint Copy", reason: "PI commodity blueprint for this run" },
  { operationId: 6, operationGoal: "Prepare Titan Components", typeName: "Capital Construction Parts Blueprint", reason: "Shares the Capital Construction Parts supply with Avatar" },
  { operationId: 6, operationGoal: "Prepare Titan Components", typeName: "Capital Jump Drive Blueprint", reason: "Shared Titan-class component staging requirement" },
];

function ownedTypeNames(): Set<string> {
  return new Set(BLUEPRINTS.map((b) => b.typeName));
}

function linkedOperationFor(typeName: string): string | null {
  const match = REQUIREMENTS.find((r) => r.typeName === typeName);
  return match ? match.operationGoal : null;
}

function toRecord(b: RawBlueprint): BlueprintRecord {
  return {
    blueprintId: b.blueprintId,
    typeName: b.typeName,
    isCopy: b.isCopy,
    meLevel: b.meLevel,
    teLevel: b.teLevel,
    runsRemaining: b.runsRemaining,
    ownerName: b.ownerName,
    locationName: b.locationName,
    status: b.status,
    linkedOperation: linkedOperationFor(b.typeName),
  };
}

export function listMockBlueprints(): BlueprintRecord[] {
  return [...BLUEPRINTS].sort((a, b) => a.blueprintId - b.blueprintId).map(toRecord);
}

export function getMockBlueprintDetail(blueprintId: number): BlueprintDetail {
  const b = BLUEPRINTS.find((x) => x.blueprintId === blueprintId);
  if (!b) throw new Error(`unknown blueprint: ${blueprintId}`);
  const requiredBy = [...new Set(REQUIREMENTS.filter((r) => r.typeName === b.typeName).map((r) => r.operationGoal))];
  return { record: toRecord(b), requiredBy };
}

export function getMockBlueprintSummary(): BlueprintSummary {
  const total = BLUEPRINTS.length;
  const bpo = BLUEPRINTS.filter((b) => !b.isCopy).length;
  const bpc = BLUEPRINTS.filter((b) => b.isCopy).length;
  const researchComplete = BLUEPRINTS.filter((b) => b.status !== "researching").length;
  const copies = BLUEPRINTS.filter((b) => b.isCopy).reduce((sum, b) => sum + (b.runsRemaining ?? 0), 0);
  const owned = ownedTypeNames();
  const missingForOperations = new Set(
    REQUIREMENTS.filter((r) => !owned.has(r.typeName)).map((r) => r.typeName),
  ).size;

  return {
    totalBlueprints: total,
    bpoCount: bpo,
    bpcCount: bpc,
    researchComplete,
    copies,
    missingForOperations,
  };
}

/** Detect only — mirrors BlueprintRepository::missing_report. */
export function getMockMissingBlueprintReport(): MissingBlueprintReport[] {
  const owned = ownedTypeNames();
  const missingTypes = [...new Set(REQUIREMENTS.filter((r) => !owned.has(r.typeName)).map((r) => r.typeName))];
  return missingTypes.map((typeName) => ({
    typeName,
    requiredByOperations: [
      ...new Set(REQUIREMENTS.filter((r) => r.typeName === typeName).map((r) => r.operationGoal)),
    ],
  }));
}

function readinessFor(operationId: number): BlueprintReadiness {
  const reqs = REQUIREMENTS.filter((r) => r.operationId === operationId);
  const owned = ownedTypeNames();
  const required: BlueprintRequirement[] = reqs.map((r) => {
    const isOwned = owned.has(r.typeName);
    const warningBlueprint = BLUEPRINTS.find(
      (b) => b.typeName === r.typeName && (b.status === "researching" || b.status === "copying"),
    );
    return {
      typeName: r.typeName,
      reason: r.reason,
      isOwned,
      warningStatus: warningBlueprint ? warningBlueprint.status : null,
    };
  });

  return {
    operationId,
    operationGoal: reqs[0]?.operationGoal ?? "",
    required,
    ownedCount: required.filter((r) => r.isOwned).length,
    missingCount: required.filter((r) => !r.isOwned).length,
    warningCount: required.filter((r) => r.warningStatus !== null).length,
  };
}

export function getMockBlueprintReadinessForOperation(operationId: number): BlueprintReadiness {
  return readinessFor(operationId);
}

export function getMockBlueprintReadinessAll(): BlueprintReadiness[] {
  const operationIds = [...new Set(REQUIREMENTS.map((r) => r.operationId))].sort((a, b) => a - b);
  return operationIds.map((id) => readinessFor(id));
}

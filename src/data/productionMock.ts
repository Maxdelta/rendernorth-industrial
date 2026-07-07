// Sprint 007 demo dataset — browser-mode fallback for the Production
// Requirement Engine. Mirrors migration 0007's seed and the derivation
// logic in src-tauri/src/production/repository.rs (coverage, shortage,
// critical bottlenecks), so browser mode and the Tauri shell show the
// same shape.
import type {
  CategoryCoverage,
  CriticalBottleneck,
  OperationRequirementBreakdown,
  RequirementCategory,
  RequirementDetail,
  RequirementLine,
  RequirementShortage,
  RequirementSummary,
} from "../lib/backend";

interface RawRequirement {
  requirementId: number;
  operationId: number;
  operationGoal: string;
  categoryKey: string;
  categoryLabel: string;
  typeName: string;
  requiredQuantity: number;
  ownedQuantity: number; // mirrors inventory_items total for this type_name
  reservedForOperation: number; // mirrors this operation's active reservation on this type_name
  note: string;
}

const CATEGORY_LABELS: Record<string, string> = {
  minerals: "Minerals",
  capital_components: "Capital Components",
  advanced_components: "Advanced Components",
  pi: "PI",
  reaction_materials: "Reaction Materials",
  fuel: "Fuel",
};

const REQUIREMENTS: RawRequirement[] = [
  { requirementId: 1, operationId: 1, operationGoal: "Build Avatar", categoryKey: "minerals", categoryLabel: "Minerals", typeName: "Nocxium", requiredQuantity: 2_000_000, ownedQuantity: 1_620_000, reservedForOperation: 0, note: "Avatar advanced component and hull plating mineral demand" },
  { requirementId: 2, operationId: 1, operationGoal: "Build Avatar", categoryKey: "capital_components", categoryLabel: "Capital Components", typeName: "Capital Construction Parts", requiredQuantity: 250, ownedQuantity: 210, reservedForOperation: 90, note: "Avatar capital hull assembly requirement" },
  { requirementId: 3, operationId: 1, operationGoal: "Build Avatar", categoryKey: "capital_components", categoryLabel: "Capital Components", typeName: "Capital Armor Plates", requiredQuantity: 70, ownedQuantity: 68, reservedForOperation: 0, note: "Avatar armor tier requirement" },
  { requirementId: 4, operationId: 1, operationGoal: "Build Avatar", categoryKey: "advanced_components", categoryLabel: "Advanced Components", typeName: "Auto-Integrity Preservation Seal", requiredQuantity: 60, ownedQuantity: 57, reservedForOperation: 0, note: "Avatar advanced component line requirement" },
  { requirementId: 5, operationId: 1, operationGoal: "Build Avatar", categoryKey: "pi", categoryLabel: "PI", typeName: "Broadcast Node", requiredQuantity: 60, ownedQuantity: 48, reservedForOperation: 18, note: "Avatar PI-fed advanced component requirement" },
  { requirementId: 6, operationId: 2, operationGoal: "Build Navy Revelation", categoryKey: "minerals", categoryLabel: "Minerals", typeName: "Megacyte", requiredQuantity: 150_000, ownedQuantity: 288_000, reservedForOperation: 0, note: "Navy Revelation mineral demand" },
  { requirementId: 7, operationId: 2, operationGoal: "Build Navy Revelation", categoryKey: "capital_components", categoryLabel: "Capital Components", typeName: "Capital Armor Plates", requiredQuantity: 90, ownedQuantity: 68, reservedForOperation: 0, note: "Navy Revelation faction armor tier requirement" },
  { requirementId: 8, operationId: 3, operationGoal: "Build Apostle", categoryKey: "minerals", categoryLabel: "Minerals", typeName: "Megacyte", requiredQuantity: 320_000, ownedQuantity: 288_000, reservedForOperation: 0, note: "Apostle mineral demand" },
  { requirementId: 9, operationId: 3, operationGoal: "Build Apostle", categoryKey: "capital_components", categoryLabel: "Capital Components", typeName: "Capital Capacitor Batteries", requiredQuantity: 50, ownedQuantity: 42, reservedForOperation: 0, note: "Apostle capacitor component requirement" },
  { requirementId: 10, operationId: 4, operationGoal: "Manufacture Capital Construction Parts", categoryKey: "minerals", categoryLabel: "Minerals", typeName: "Tritanium", requiredQuantity: 900_000_000, ownedQuantity: 842_000_000, reservedForOperation: 0, note: "Capital Construction Parts production run mineral demand" },
  { requirementId: 11, operationId: 4, operationGoal: "Manufacture Capital Construction Parts", categoryKey: "reaction_materials", categoryLabel: "Reaction Materials", typeName: "Mechanical Parts", requiredQuantity: 120_000, ownedQuantity: 96_000, reservedForOperation: 0, note: "Capital Construction Parts reaction material demand" },
  { requirementId: 12, operationId: 4, operationGoal: "Manufacture Capital Construction Parts", categoryKey: "fuel", categoryLabel: "Fuel", typeName: "Nitrogen Fuel Block", requiredQuantity: 110_000, ownedQuantity: 96_000, reservedForOperation: 0, note: "Capital Construction Parts run fuel demand" },
  { requirementId: 13, operationId: 5, operationGoal: "Manufacture Broadcast Nodes", categoryKey: "pi", categoryLabel: "PI", typeName: "Ukomi Superconductors", requiredQuantity: 320, ownedQuantity: 280, reservedForOperation: 0, note: "Broadcast Node P3 input demand" },
  { requirementId: 14, operationId: 5, operationGoal: "Manufacture Broadcast Nodes", categoryKey: "pi", categoryLabel: "PI", typeName: "Condensates", requiredQuantity: 300, ownedQuantity: 260, reservedForOperation: 0, note: "Broadcast Node P3 input demand" },
  { requirementId: 15, operationId: 5, operationGoal: "Manufacture Broadcast Nodes", categoryKey: "pi", categoryLabel: "PI", typeName: "High-Tech Transmitters", requiredQuantity: 280, ownedQuantity: 240, reservedForOperation: 0, note: "Broadcast Node P3 input demand" },
  { requirementId: 16, operationId: 6, operationGoal: "Prepare Titan Components", categoryKey: "capital_components", categoryLabel: "Capital Components", typeName: "Capital Construction Parts", requiredQuantity: 180, ownedQuantity: 210, reservedForOperation: 50, note: "Titan component staging demand, shares supply with Avatar" },
];

function toLine(r: RawRequirement): RequirementLine {
  const shortage = Math.max(r.requiredQuantity - r.ownedQuantity, 0);
  const coverageFraction = r.requiredQuantity > 0 ? Math.min(r.ownedQuantity / r.requiredQuantity, 1) : 1;
  return {
    requirementId: r.requirementId,
    operationId: r.operationId,
    operationGoal: r.operationGoal,
    categoryKey: r.categoryKey,
    categoryLabel: r.categoryLabel,
    typeName: r.typeName,
    requiredQuantity: r.requiredQuantity,
    ownedQuantity: r.ownedQuantity,
    reservedForOperation: r.reservedForOperation,
    shortage,
    coverageFraction,
    isSatisfied: r.ownedQuantity >= r.requiredQuantity,
  };
}

export function listMockRequirementLines(operationId?: number, categoryKey?: string): RequirementLine[] {
  return REQUIREMENTS.filter(
    (r) => (operationId === undefined || r.operationId === operationId) && (categoryKey === undefined || r.categoryKey === categoryKey),
  ).map(toLine);
}

export function getMockRequirementDetail(requirementId: number): RequirementDetail {
  const r = REQUIREMENTS.find((x) => x.requirementId === requirementId);
  if (!r) throw new Error(`unknown requirement: ${requirementId}`);
  return {
    line: toLine(r),
    sources: [{ sourceKind: "demo_seed", contributedQuantity: r.requiredQuantity, note: r.note }],
  };
}

export function listMockRequirementCategories(): RequirementCategory[] {
  const order = ["minerals", "capital_components", "advanced_components", "pi", "reaction_materials", "fuel"];
  const present = new Set(REQUIREMENTS.map((r) => r.categoryKey));
  return order
    .filter((k) => present.has(k))
    .map((k, i) => ({ categoryKey: k, categoryLabel: CATEGORY_LABELS[k] ?? k, sortOrder: i }));
}

export function getMockRequirementSummary(): RequirementSummary {
  const lines = REQUIREMENTS.map(toLine);
  const totalRequirements = lines.length;
  const satisfied = lines.filter((l) => l.isSatisfied).length;
  const missing = totalRequirements - satisfied;
  const totalRequired = lines.reduce((sum, l) => sum + l.requiredQuantity, 0);
  const totalCovered = lines.reduce((sum, l) => sum + Math.min(l.ownedQuantity, l.requiredQuantity), 0);
  const coveragePercent = totalRequired > 0 ? (100 * totalCovered) / totalRequired : 0;

  return {
    totalRequirements,
    satisfied,
    missing,
    coveragePercent,
    criticalBottleneckCount: getMockCriticalBottlenecks().length,
  };
}

export function getMockRequirementShortages(): RequirementShortage[] {
  return REQUIREMENTS.map(toLine)
    .filter((l) => !l.isSatisfied)
    .map((l) => ({
      typeName: l.typeName,
      operationGoal: l.operationGoal,
      requiredQuantity: l.requiredQuantity,
      ownedQuantity: l.ownedQuantity,
      shortage: l.shortage,
    }));
}

/** Mirrors ProductionRepository::critical_bottlenecks_from. */
export function getMockCriticalBottlenecks(): CriticalBottleneck[] {
  const lines = REQUIREMENTS.map(toLine);
  const byType = new Map<string, RequirementLine[]>();
  for (const l of lines) {
    byType.set(l.typeName, [...(byType.get(l.typeName) ?? []), l]);
  }
  const out: CriticalBottleneck[] = [];
  for (const [typeName, group] of byType) {
    const distinctOps = new Set(group.map((l) => l.operationId));
    const anyUnmet = group.some((l) => !l.isSatisfied);
    if (distinctOps.size > 1 && anyUnmet) {
      out.push({
        typeName,
        operationsRequiring: group.map((l) => l.operationGoal),
        ownedQuantity: group[0].ownedQuantity,
      });
    }
  }
  return out.sort((a, b) => a.typeName.localeCompare(b.typeName));
}

export function getMockOperationRequirementBreakdown(operationId: number): OperationRequirementBreakdown {
  const lines = listMockRequirementLines(operationId);
  const operationGoal = lines[0]?.operationGoal ?? "";

  const byCategory = new Map<string, { label: string; required: number; covered: number; missing: number }>();
  for (const l of lines) {
    const entry = byCategory.get(l.categoryKey) ?? { label: l.categoryLabel, required: 0, covered: 0, missing: 0 };
    entry.required += l.requiredQuantity;
    entry.covered += Math.min(l.ownedQuantity, l.requiredQuantity);
    if (!l.isSatisfied) entry.missing += 1;
    byCategory.set(l.categoryKey, entry);
  }

  const categories: CategoryCoverage[] = [...byCategory.entries()]
    .map(([categoryKey, v]) => ({
      categoryKey,
      categoryLabel: v.label,
      requiredTotal: v.required,
      ownedTotal: v.covered,
      coverageFraction: v.required > 0 ? v.covered / v.required : 1,
      missingCount: v.missing,
    }))
    .sort((a, b) => a.categoryKey.localeCompare(b.categoryKey));

  const totalRequired = lines.reduce((sum, l) => sum + l.requiredQuantity, 0);
  const totalCovered = lines.reduce((sum, l) => sum + Math.min(l.ownedQuantity, l.requiredQuantity), 0);

  return {
    operationId,
    operationGoal,
    categories,
    lines,
    totalRequirements: lines.length,
    missingCount: lines.filter((l) => !l.isSatisfied).length,
    coveragePercent: totalRequired > 0 ? (100 * totalCovered) / totalRequired : 0,
  };
}

import type { LeafTotal, ProductionPlan } from "../lib/backend";

/** "Shopping List — Prices Not Included." No prices, no market sourcing, ever. */
const TITLE = "Material List — Prices Not Included";

function rows(plan: ProductionPlan): LeafTotal[] {
  return plan.leafTotals;
}

function csvEscape(value: string): string {
  if (value.includes(",") || value.includes('"')) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

export function toCsv(plan: ProductionPlan): string {
  const header = ["Type ID", "Type Name", "Category/Group", "Unit m3", "Required", "Required m3", "Owned", "Owned m3", "Owned Source", "Reserved", "Available", "Missing", "Purchase m3", "Operation"];
  const lines = [header.join(",")];
  for (const r of rows(plan)) {
    const categoryGroup = [r.categoryName, r.groupName].filter(Boolean).join(" / ");
    lines.push(
      [
        r.typeId,
        csvEscape(r.typeName),
        csvEscape(categoryGroup || "—"),
        r.unitVolumeM3 ?? "",
        r.requiredQuantity,
        r.requiredVolumeM3 ?? "",
        r.ownedQuantity,
        r.ownedVolumeM3 ?? "",
        csvEscape(r.ownedQuantity > 0 ? r.ownedSource : ""),
        r.reservedQuantity,
        r.availableQuantity,
        r.missingQuantity,
        r.missingVolumeM3 ?? "",
        csvEscape(plan.operationGoal),
      ].join(","),
    );
  }
  return lines.join("\n");
}

export function toMarkdown(plan: ProductionPlan): string {
  const lines = [
    `# ${TITLE}`,
    "",
    `Operation: ${plan.operationGoal}`,
    `Build target: ${plan.buildTargetName} × ${plan.requestedQuantity} (${plan.totalRuns} runs, ${plan.blueprintMode})`,
    `Inventory scope: ${plan.inventoryScope}`,
    `Total required volume: ${plan.totalRequiredVolumeM3 ?? "unknown"} m³`,
    `Total owned volume: ${plan.totalOwnedVolumeM3 ?? "unknown"} m³`,
    `Total purchase volume: ${plan.totalMissingVolumeM3 ?? "unknown"} m³`,
    "",
    "| Type | Category/Group | Unit m³ | Required | Required m³ | Owned | Owned m³ | Owned Source | Reserved | Available | Missing | Purchase m³ |",
    "|---|---|---|---|---|---|---|---|---|---|---|---|",
  ];
  for (const r of rows(plan)) {
    const categoryGroup = [r.categoryName, r.groupName].filter(Boolean).join(" / ") || "—";
    const source = r.ownedQuantity > 0 ? r.ownedSource : "—";
    lines.push(
      `| ${r.typeName} | ${categoryGroup} | ${r.unitVolumeM3 ?? "unknown"} | ${r.requiredQuantity} | ${r.requiredVolumeM3 ?? "unknown"} | ${r.ownedQuantity} | ${r.ownedVolumeM3 ?? "unknown"} | ${source} | ${r.reservedQuantity} | ${r.availableQuantity} | ${r.missingQuantity} | ${r.missingVolumeM3 ?? "unknown"} |`,
    );
  }
  return lines.join("\n");
}

export function toPlainText(plan: ProductionPlan): string {
  const lines = [TITLE, `Operation: ${plan.operationGoal}`, `Build target: ${plan.buildTargetName} × ${plan.requestedQuantity}`, `Total purchase volume: ${plan.totalMissingVolumeM3 ?? "unknown"} m³`, ""];
  for (const r of rows(plan)) {
    lines.push(`${r.typeName}: need ${r.requiredQuantity} (${r.requiredVolumeM3 ?? "unknown"} m³), have ${r.ownedQuantity} (${r.ownedVolumeM3 ?? "unknown"} m³; ${r.ownedQuantity > 0 ? r.ownedSource : "none"}), missing ${r.missingQuantity} (${r.missingVolumeM3 ?? "unknown"} m³)`);
  }
  return lines.join("\n");
}

export async function copyToClipboard(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

export function downloadFile(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

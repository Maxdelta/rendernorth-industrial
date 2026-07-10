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
  const header = ["Type ID", "Type Name", "Category/Group", "Required", "Owned", "Reserved", "Available", "Missing", "Operation"];
  const lines = [header.join(",")];
  for (const r of rows(plan)) {
    const categoryGroup = [r.categoryName, r.groupName].filter(Boolean).join(" / ");
    lines.push(
      [
        r.typeId,
        csvEscape(r.typeName),
        csvEscape(categoryGroup || "—"),
        r.requiredQuantity,
        r.ownedQuantity,
        r.reservedQuantity,
        r.availableQuantity,
        r.missingQuantity,
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
    "",
    "| Type | Category/Group | Required | Owned | Reserved | Available | Missing |",
    "|---|---|---|---|---|---|---|",
  ];
  for (const r of rows(plan)) {
    const categoryGroup = [r.categoryName, r.groupName].filter(Boolean).join(" / ") || "—";
    lines.push(
      `| ${r.typeName} | ${categoryGroup} | ${r.requiredQuantity} | ${r.ownedQuantity} | ${r.reservedQuantity} | ${r.availableQuantity} | ${r.missingQuantity} |`,
    );
  }
  return lines.join("\n");
}

export function toPlainText(plan: ProductionPlan): string {
  const lines = [TITLE, `Operation: ${plan.operationGoal}`, `Build target: ${plan.buildTargetName} × ${plan.requestedQuantity}`, ""];
  for (const r of rows(plan)) {
    lines.push(`${r.typeName}: need ${r.requiredQuantity}, have ${r.ownedQuantity}, missing ${r.missingQuantity}`);
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

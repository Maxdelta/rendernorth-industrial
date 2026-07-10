/** Sprint 008.2 — tolerant "Name<sep>Quantity" line parsing for Paste Inventory. */

export interface ParsedInventoryRow {
  rawLine: string;
  name: string | null;
  quantity: number | null;
  problem: "invalid_format" | "invalid_quantity" | null;
}

/**
 * Splits one line into [name, quantityRaw]. One general rule handles tabs,
 * multiple spaces, a single space, and comma-separated columns uniformly:
 * find the shortest possible name prefix such that everything after it is
 * one-or-more separator characters (tab/space/comma) followed by a
 * trailing run of digits-and-commas (a thousands-grouped number) to the
 * end of the line.
 */
function splitLine(line: string): [string, string] | null {
  const tabParts = line.split("\t").map((s) => s.trim()).filter(Boolean);
  if (tabParts.length === 2) return [tabParts[0], tabParts[1]];

  const match = line.trim().match(/^(.*?)[\t ,]+([\d,]+)$/);
  if (!match) return null;
  const name = match[1].trim();
  const quantityRaw = match[2];
  if (name.length === 0) return null;
  return [name, quantityRaw];
}

function parseQuantity(raw: string): number | null {
  const cleaned = raw.replace(/,/g, "").trim();
  if (!/^\d+$/.test(cleaned)) return null;
  const n = Number(cleaned);
  return Number.isFinite(n) && n > 0 ? n : null;
}

function parseOneLine(line: string): ParsedInventoryRow {
  const split = splitLine(line);
  if (!split) return { rawLine: line, name: null, quantity: null, problem: "invalid_format" };
  const [namePart, quantityPart] = split;
  const quantity = parseQuantity(quantityPart);
  if (quantity === null) return { rawLine: line, name: namePart, quantity: null, problem: "invalid_quantity" };
  return { rawLine: line, name: namePart, quantity, problem: null };
}

export function parsePastedInventory(text: string): ParsedInventoryRow[] {
  const lines = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0);

  if (lines.length === 0) return [];

  // An optional header row is only ever skipped when there's at least one
  // more line to justify it, AND the first line itself fails to parse as
  // valid data — a single bad line pasted alone is always reported, never
  // silently swallowed as "probably a header."
  const first = parseOneLine(lines[0]);
  const firstLineIsHeader = lines.length > 1 && first.problem !== null;

  const dataLines = firstLineIsHeader ? lines.slice(1) : lines;
  return dataLines.map(parseOneLine);
}

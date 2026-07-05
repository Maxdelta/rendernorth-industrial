// Sprint 003 demo dataset — browser-mode fallback for the Inventory Engine.
// Mirrors the seed rows in migrations 0003 and the derivation logic in
// src-tauri/src/inventory/repository.rs, so browser mode and the Tauri
// shell show the same shape of data. Nothing here knows what a Titan is —
// every row is plain data flowing through the same category/location/owner
// fields.
import type { InventoryCategory, InventoryItem, InventorySummary } from "../lib/backend";

interface RawItem {
  itemId: number;
  typeName: string;
  categoryKey: string;
  quantity: number;
  locationName: string;
  ownerName: string;
  unitValue: number;
  state?: string; // defaults to "available"
  reservedQuantity?: number;
  reservedOperation?: string | null;
  allocatedOperation?: string | null;
}

const CATEGORIES: { key: string; label: string; sortOrder: number }[] = [
  { key: "ships", label: "Ships", sortOrder: 1 },
  { key: "blueprints", label: "Blueprints", sortOrder: 2 },
  { key: "minerals", label: "Minerals", sortOrder: 3 },
  { key: "ore", label: "Ore", sortOrder: 4 },
  { key: "compressed_ore", label: "Compressed Ore", sortOrder: 5 },
  { key: "ice", label: "Ice", sortOrder: 6 },
  { key: "ice_products", label: "Ice Products", sortOrder: 7 },
  { key: "pi", label: "PI", sortOrder: 8 },
  { key: "reaction_materials", label: "Reaction Materials", sortOrder: 9 },
  { key: "components", label: "Components", sortOrder: 10 },
  { key: "capital_components", label: "Capital Components", sortOrder: 11 },
  { key: "advanced_components", label: "Advanced Components", sortOrder: 12 },
  { key: "modules", label: "Modules", sortOrder: 13 },
  { key: "charges", label: "Charges", sortOrder: 14 },
  { key: "fuel", label: "Fuel", sortOrder: 15 },
  { key: "structures", label: "Structures", sortOrder: 16 },
  { key: "deployables", label: "Deployables", sortOrder: 17 },
];

const STATE_LABELS: Record<string, string> = {
  available: "Available",
  reserved: "Reserved",
  allocated: "Allocated",
  manufacturing: "Manufacturing",
  research: "Research",
  reaction: "Reaction",
  in_transit: "In Transit",
  asset_safety: "Asset Safety",
  contract: "Contract",
  delivery: "Delivery",
  destroyed: "Destroyed",
};

const ITEMS: RawItem[] = [
  { itemId: 1, typeName: "Providence", categoryKey: "ships", quantity: 1, locationName: "Jita IV - Moon 4 - Caldari Navy Assembly Plant", ownerName: "Demo Forgemaster", unitValue: 980_000_000 },
  { itemId: 2, typeName: "Orca", categoryKey: "ships", quantity: 2, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 1_450_000_000 },
  { itemId: 3, typeName: "Rorqual", categoryKey: "ships", quantity: 1, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 12_800_000_000 },
  { itemId: 4, typeName: "Avatar Blueprint Copy", categoryKey: "blueprints", quantity: 1, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 0 },
  { itemId: 5, typeName: "Capital Construction Parts Blueprint", categoryKey: "blueprints", quantity: 1, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 3_500_000_000 },
  { itemId: 6, typeName: "Capital Armor Plates Blueprint", categoryKey: "blueprints", quantity: 1, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 4_200_000_000 },
  { itemId: 7, typeName: "Broadcast Node Blueprint Copy", categoryKey: "blueprints", quantity: 3, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 0 },
  { itemId: 8, typeName: "Tritanium", categoryKey: "minerals", quantity: 842_000_000, locationName: "Jita IV - Moon 4 - Caldari Navy Assembly Plant", ownerName: "Demo Forgemaster", unitValue: 4.2 },
  { itemId: 9, typeName: "Pyerite", categoryKey: "minerals", quantity: 210_000_000, locationName: "Jita IV - Moon 4 - Caldari Navy Assembly Plant", ownerName: "Demo Forgemaster", unitValue: 9.8 },
  { itemId: 10, typeName: "Mexallon", categoryKey: "minerals", quantity: 38_500_000, locationName: "Jita IV - Moon 4 - Caldari Navy Assembly Plant", ownerName: "Demo Forgemaster", unitValue: 65 },
  { itemId: 11, typeName: "Isogen", categoryKey: "minerals", quantity: 12_100_000, locationName: "Jita IV - Moon 4 - Caldari Navy Assembly Plant", ownerName: "Demo Forgemaster", unitValue: 145 },
  { itemId: 12, typeName: "Nocxium", categoryKey: "minerals", quantity: 1_620_000, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 850, reservedQuantity: 900_000, reservedOperation: "Navy Revelation" },
  { itemId: 13, typeName: "Zydrine", categoryKey: "minerals", quantity: 410_000, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 1450 },
  { itemId: 14, typeName: "Megacyte", categoryKey: "minerals", quantity: 288_000, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 2100, allocatedOperation: "Apostle" },
  { itemId: 15, typeName: "Morphite", categoryKey: "minerals", quantity: 62_000, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 8200 },
  { itemId: 16, typeName: "Bistot", categoryKey: "ore", quantity: 340_000, locationName: "Rens VI - Moon 8 - Brutor Tribe Treasury", ownerName: "Demo Hauler", unitValue: 950 },
  { itemId: 17, typeName: "Arkonor", categoryKey: "ore", quantity: 210_000, locationName: "Rens VI - Moon 8 - Brutor Tribe Treasury", ownerName: "Demo Hauler", unitValue: 1150 },
  { itemId: 18, typeName: "Mercoxit", categoryKey: "ore", quantity: 48_000, locationName: "Rens VI - Moon 8 - Brutor Tribe Treasury", ownerName: "Demo Hauler", unitValue: 3200 },
  { itemId: 19, typeName: "Compressed Bistot", categoryKey: "compressed_ore", quantity: 12_400, locationName: "Rens VI - Moon 8 - Brutor Tribe Treasury", ownerName: "Demo Hauler", unitValue: 9500 },
  { itemId: 20, typeName: "Compressed Arkonor", categoryKey: "compressed_ore", quantity: 8_600, locationName: "Rens VI - Moon 8 - Brutor Tribe Treasury", ownerName: "Demo Hauler", unitValue: 11_500 },
  { itemId: 21, typeName: "Glacial Mass", categoryKey: "ice", quantity: 96_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 550 },
  { itemId: 22, typeName: "Krystallos", categoryKey: "ice", quantity: 41_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 780 },
  { itemId: 23, typeName: "Heavy Water", categoryKey: "ice_products", quantity: 620_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 320 },
  { itemId: 24, typeName: "Liquid Ozone", categoryKey: "ice_products", quantity: 480_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 410 },
  { itemId: 25, typeName: "Helium Isotopes", categoryKey: "ice_products", quantity: 510_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 260 },
  { itemId: 26, typeName: "Broadcast Node", categoryKey: "pi", quantity: 48, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 1_450_000, reservedQuantity: 18, reservedOperation: "Avatar", allocatedOperation: "Avatar" },
  { itemId: 27, typeName: "Ukomi Superconductors", categoryKey: "pi", quantity: 280, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 68_000 },
  { itemId: 28, typeName: "Condensates", categoryKey: "pi", quantity: 260, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 71_000 },
  { itemId: 29, typeName: "High-Tech Transmitters", categoryKey: "pi", quantity: 240, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 69_500 },
  { itemId: 30, typeName: "Mechanical Parts", categoryKey: "reaction_materials", quantity: 96_000, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 145 },
  { itemId: 31, typeName: "Hypersynaptic Fibers", categoryKey: "reaction_materials", quantity: 8_200, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 1650 },
  { itemId: 32, typeName: "Construction Blocks", categoryKey: "components", quantity: 14_200, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 3200 },
  { itemId: 33, typeName: "Nanite Compound", categoryKey: "components", quantity: 6_100, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 5100 },
  { itemId: 34, typeName: "Capital Construction Parts", categoryKey: "capital_components", quantity: 210, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 1_180_000, reservedQuantity: 90, reservedOperation: "Avatar", allocatedOperation: "Avatar" },
  { itemId: 35, typeName: "Capital Armor Plates", categoryKey: "capital_components", quantity: 68, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 2_450_000, allocatedOperation: "Avatar" },
  { itemId: 36, typeName: "Capital Capacitor Batteries", categoryKey: "capital_components", quantity: 42, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 2_980_000 },
  { itemId: 37, typeName: "Capital Jump Drive", categoryKey: "capital_components", quantity: 6, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 8_900_000 },
  { itemId: 38, typeName: "Auto-Integrity Preservation Seal", categoryKey: "advanced_components", quantity: 57, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 3_650_000 },
  { itemId: 39, typeName: "Life Support Backup Unit", categoryKey: "advanced_components", quantity: 34, locationName: "Egghelende - RenderNorth Forge (Keepstar)", ownerName: "Demo Forgemaster", unitValue: 2_100_000 },
  { itemId: 40, typeName: "Capital Shield Extender II", categoryKey: "modules", quantity: 4, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 42_000_000 },
  { itemId: 41, typeName: "Large Armor Repairer II", categoryKey: "modules", quantity: 18, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 18_500_000 },
  { itemId: 42, typeName: "Antimatter Charge L", categoryKey: "charges", quantity: 240_000, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 480 },
  { itemId: 43, typeName: "Void L", categoryKey: "charges", quantity: 180_000, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 520 },
  { itemId: 44, typeName: "Nitrogen Fuel Block", categoryKey: "fuel", quantity: 96_000, locationName: "Nakugard - Home POS Silo", ownerName: "Demo Hauler", unitValue: 1180 },
  { itemId: 45, typeName: "Oxygen Isotopes", categoryKey: "fuel", quantity: 620_000, locationName: "Saatuban - Cyno Beacon Storage (Fortizar)", ownerName: "Demo Hauler", unitValue: 210 },
  { itemId: 46, typeName: "Mobile Depot", categoryKey: "deployables", quantity: 11, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 1_650_000 },
  { itemId: 47, typeName: "Mobile Tractor Unit", categoryKey: "deployables", quantity: 7, locationName: "1DQ1-A - Sotiyo (corp staging)", ownerName: "Demo Hauler", unitValue: 2_950_000 },
];

const KNOWN_LOCATIONS = 10; // mirrors inventory_locations row count in migration 0003

function toInventoryItem(raw: RawItem): InventoryItem {
  const state = raw.state ?? "available";
  const reservedQuantity = raw.reservedQuantity ?? 0;
  const stateLabel = STATE_LABELS[state] ?? state;

  let status: string;
  if (state !== "available") {
    status = stateLabel;
  } else if (reservedQuantity >= raw.quantity && raw.quantity > 0) {
    status = "Reserved";
  } else if (reservedQuantity > 0) {
    status = "Partially Reserved";
  } else if (raw.allocatedOperation) {
    status = "Allocated";
  } else {
    status = "Available";
  }

  return {
    itemId: raw.itemId,
    typeName: raw.typeName,
    categoryKey: raw.categoryKey,
    categoryLabel: CATEGORIES.find((c) => c.key === raw.categoryKey)?.label ?? raw.categoryKey,
    quantity: raw.quantity,
    locationName: raw.locationName,
    ownerName: raw.ownerName,
    unitValue: raw.unitValue,
    totalValue: raw.unitValue * raw.quantity,
    reservedQuantity,
    availableQuantity: Math.max(0, raw.quantity - reservedQuantity),
    allocatedOperation: raw.allocatedOperation ?? null,
    reservedOperation: raw.reservedOperation ?? null,
    state,
    stateLabel,
    status,
  };
}

export function listMockInventoryItems(categoryKey?: string): InventoryItem[] {
  const filtered = categoryKey ? ITEMS.filter((i) => i.categoryKey === categoryKey) : ITEMS;
  return filtered.map(toInventoryItem).sort((a, b) => b.totalValue - a.totalValue);
}

export function listMockInventoryCategories(): InventoryCategory[] {
  return CATEGORIES.map((c) => ({
    ...c,
    itemCount: ITEMS.filter((i) => i.categoryKey === c.key).reduce((sum, i) => sum + i.quantity, 0),
  }));
}

export function getMockInventorySummary(): InventorySummary {
  const totalAssets = ITEMS.reduce((sum, i) => sum + i.quantity, 0);
  const estimatedValue = ITEMS.reduce((sum, i) => sum + i.quantity * i.unitValue, 0);
  const uniqueItemTypes = new Set(ITEMS.map((i) => i.typeName)).size;
  const reservedValue = ITEMS.reduce((sum, i) => sum + (i.reservedQuantity ?? 0) * i.unitValue, 0);

  return {
    totalAssets,
    estimatedValue,
    uniqueItemTypes,
    locations: KNOWN_LOCATIONS,
    reservedValue,
    availableValue: estimatedValue - reservedValue,
  };
}

/** Same placeholder formula as InventoryRepository::coverage_for_operation: average of a target's requirement tiers. */
export function coverageFromTiers(tiers: { coverage: number }[]): number {
  if (tiers.length === 0) return 0;
  return tiers.reduce((sum, t) => sum + t.coverage, 0) / tiers.length;
}

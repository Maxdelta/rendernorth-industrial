// Sprint 005 demo dataset — browser-mode fallback for the Reservation
// Engine. Mirrors migration 0005's seed and the derivation logic in
// src-tauri/src/reservation/repository.rs (conflicts, inventory
// commitment), so browser mode and the Tauri shell show the same shape.
import type {
  InventoryCommitment,
  OperationReservations,
  ReservationConflict,
  ReservationDetail,
  ReservationEvent,
  ReservationRecord,
  ReservationSummary,
} from "../lib/backend";

interface RawReservation {
  reservationId: number;
  itemId: number;
  itemName: string;
  itemQuantity: number; // on-hand stock for this item, mirrors inventory_items.quantity
  operationId: number | null;
  operationGoal: string | null;
  quantity: number;
  reason: string;
  createdAt: string;
  releasedAt: string | null;
}

const RESERVATIONS: RawReservation[] = [
  { reservationId: 1, itemId: 26, itemName: "Broadcast Node", itemQuantity: 48, operationId: 1, operationGoal: "Build Avatar", quantity: 18, reason: "Reserved for Avatar PI requirement", createdAt: "2026-04-20T00:00:00Z", releasedAt: null },
  { reservationId: 2, itemId: 34, itemName: "Capital Construction Parts", itemQuantity: 210, operationId: 1, operationGoal: "Build Avatar", quantity: 90, reason: "Reserved for Avatar capital component requirement", createdAt: "2026-04-22T00:00:00Z", releasedAt: null },
  { reservationId: 3, itemId: 12, itemName: "Nocxium", itemQuantity: 1_620_000, operationId: 2, operationGoal: "Build Navy Revelation", quantity: 900_000, reason: "Reserved for Navy Revelation mineral requirement", createdAt: "2026-04-25T00:00:00Z", releasedAt: null },
  { reservationId: 4, itemId: 34, itemName: "Capital Construction Parts", itemQuantity: 210, operationId: 6, operationGoal: "Prepare Titan Components", quantity: 50, reason: "Shared Capital Construction Parts stockpile for Titan component staging", createdAt: "2026-06-20T00:00:00Z", releasedAt: null },
  { reservationId: 5, itemId: 38, itemName: "Auto-Integrity Preservation Seal", itemQuantity: 57, operationId: 3, operationGoal: "Build Apostle", quantity: 70, reason: "Apostle advanced component reservation exceeds current stock — flagged for review", createdAt: "2026-06-21T00:00:00Z", releasedAt: null },
  { reservationId: 6, itemId: 44, itemName: "Nitrogen Fuel Block", itemQuantity: 96_000, operationId: 2, operationGoal: "Build Navy Revelation", quantity: 5000, reason: "Temporary Navy Revelation fuel staging (returned to general stock)", createdAt: "2026-05-01T00:00:00Z", releasedAt: "2026-05-14T00:00:00Z" },
  { reservationId: 7, itemId: 27, itemName: "Ukomi Superconductors", itemQuantity: 280, operationId: 5, operationGoal: "Manufacture Broadcast Nodes", quantity: 40, reason: "Broadcast Node PI staging hold (expired without confirmation)", createdAt: "2026-06-01T00:00:00Z", releasedAt: "2026-06-08T00:00:00Z" },
];

const EVENTS: (ReservationEvent & { reservationId: number })[] = [
  { id: 1, reservationId: 1, eventType: "reserved", quantity: 18, fromOperationId: null, toOperationId: 1, reason: "Reserved for Avatar PI requirement", createdAt: "2026-04-20T00:00:00Z" },
  { id: 2, reservationId: 2, eventType: "reserved", quantity: 90, fromOperationId: null, toOperationId: 1, reason: "Reserved for Avatar capital component requirement", createdAt: "2026-04-22T00:00:00Z" },
  { id: 3, reservationId: 3, eventType: "reserved", quantity: 900_000, fromOperationId: null, toOperationId: 2, reason: "Reserved for Navy Revelation mineral requirement", createdAt: "2026-04-25T00:00:00Z" },
  { id: 4, reservationId: 4, eventType: "reserved", quantity: 50, fromOperationId: null, toOperationId: 6, reason: "Shared Capital Construction Parts stockpile for Titan component staging", createdAt: "2026-06-20T00:00:00Z" },
  { id: 5, reservationId: 5, eventType: "reserved", quantity: 70, fromOperationId: null, toOperationId: 3, reason: "Apostle advanced component reservation exceeds current stock", createdAt: "2026-06-21T00:00:00Z" },
  { id: 6, reservationId: 6, eventType: "reserved", quantity: 5000, fromOperationId: null, toOperationId: 2, reason: "Temporary Navy Revelation fuel staging", createdAt: "2026-05-01T00:00:00Z" },
  { id: 7, reservationId: 6, eventType: "released", quantity: 5000, fromOperationId: 2, toOperationId: null, reason: "Fuel staging complete; unused portion returned to general stock", createdAt: "2026-05-14T00:00:00Z" },
  { id: 8, reservationId: 2, eventType: "transferred", quantity: 90, fromOperationId: 4, toOperationId: 1, reason: "Capital Construction Parts output reassigned directly to Avatar hull assembly", createdAt: "2026-04-23T00:00:00Z" },
  { id: 9, reservationId: 7, eventType: "reserved", quantity: 40, fromOperationId: null, toOperationId: 5, reason: "Broadcast Node PI staging hold", createdAt: "2026-06-01T00:00:00Z" },
  { id: 10, reservationId: 7, eventType: "expired", quantity: 40, fromOperationId: 5, toOperationId: null, reason: "Hold window elapsed without confirmation", createdAt: "2026-06-08T00:00:00Z" },
];

function toRecord(r: RawReservation): ReservationRecord {
  return {
    reservationId: r.reservationId,
    itemId: r.itemId,
    itemName: r.itemName,
    operationId: r.operationId,
    operationGoal: r.operationGoal,
    quantity: r.quantity,
    reason: r.reason,
    createdAt: r.createdAt,
    releasedAt: r.releasedAt,
    isActive: r.releasedAt === null,
  };
}

export function listMockActiveReservations(): ReservationRecord[] {
  return RESERVATIONS.filter((r) => r.releasedAt === null).map(toRecord);
}

export function getMockReservationDetail(reservationId: number): ReservationDetail {
  const r = RESERVATIONS.find((x) => x.reservationId === reservationId);
  if (!r) throw new Error(`unknown reservation: ${reservationId}`);
  const history = EVENTS.filter((e) => e.reservationId === reservationId)
    .slice()
    .sort((a, b) => (a.createdAt < b.createdAt ? -1 : 1))
    .map(({ id, reservationId, eventType, quantity, fromOperationId, toOperationId, reason, createdAt }) => ({
      id,
      reservationId,
      eventType,
      quantity,
      fromOperationId,
      toOperationId,
      reason,
      createdAt,
    }));
  return { record: toRecord(r), history };
}

export function getMockReservationSummary(): ReservationSummary {
  const active = RESERVATIONS.filter((r) => r.releasedAt === null);
  return {
    activeReservationCount: active.length,
    totalReservedQuantity: active.reduce((sum, r) => sum + r.quantity, 0),
    itemsWithReservations: new Set(active.map((r) => r.itemId)).size,
    conflictCount: getMockReservationConflicts().length,
  };
}

/** Detect only — mirrors the three checks in ReservationRepository::conflicts. */
export function getMockReservationConflicts(): ReservationConflict[] {
  const active = RESERVATIONS.filter((r) => r.releasedAt === null);
  const byItem = new Map<number, RawReservation[]>();
  for (const r of active) {
    byItem.set(r.itemId, [...(byItem.get(r.itemId) ?? []), r]);
  }

  const conflicts: ReservationConflict[] = [];
  for (const [itemId, rs] of byItem) {
    const itemName = rs[0].itemName;
    const onHand = rs[0].itemQuantity;
    const totalReserved = rs.reduce((sum, r) => sum + r.quantity, 0);
    const distinctOps = new Set(rs.map((r) => r.operationId)).size;

    if (distinctOps > 1) {
      conflicts.push({
        itemId,
        itemName,
        conflictType: "overlapping_operations",
        description: `${itemName} held by ${distinctOps} operations — ${totalReserved} of ${onHand} on hand`,
      });
    }
    if (totalReserved > onHand) {
      conflicts.push({
        itemId,
        itemName,
        conflictType: "exceeds_stock",
        description: `${itemName} reserved ${totalReserved} against ${onHand} on hand — ${totalReserved - onHand} units over-committed`,
      });
    }
    if (onHand === 0) {
      conflicts.push({
        itemId,
        itemName,
        conflictType: "missing_inventory",
        description: `${itemName} has an active reservation but zero on-hand stock`,
      });
    }
  }
  return conflicts;
}

export function getMockInventoryCommitment(): InventoryCommitment {
  // Mirrors the real query's per-item clamp math. Total is the sum of all
  // seeded item quantities in src/data/inventoryMock.ts, kept as a literal
  // here to avoid a circular import between the two mock modules.
  const totalInventory = 1_108_607_795;
  const active = RESERVATIONS.filter((r) => r.releasedAt === null);
  const reservedByItem = new Map<number, number>();
  for (const r of active) {
    reservedByItem.set(r.itemId, (reservedByItem.get(r.itemId) ?? 0) + r.quantity);
  }
  const reserved = active.reduce((sum, r) => sum + r.quantity, 0);
  let blocked = 0;
  for (const [itemId, qty] of reservedByItem) {
    const onHand = active.find((r) => r.itemId === itemId)?.itemQuantity ?? 0;
    blocked += Math.max(qty - onHand, 0);
  }
  const available = totalInventory - reserved + blocked;
  // Unallocated: total minus everything touched by an active reservation.
  // A browser-mode approximation — the real query also nets out soft
  // allocations per item; this fallback only has reservation data handy.
  const unallocated = totalInventory - reserved;
  return { totalInventory, reserved, available, blocked, unallocated };
}

export function getMockOperationReservations(operationId: number): OperationReservations {
  const active = RESERVATIONS.filter((r) => r.releasedAt === null && r.operationId === operationId);
  const categoryOf: Record<number, "minerals" | "components" | "pi"> = {
    12: "minerals",
    26: "pi",
    27: "pi",
    34: "components",
    35: "components",
    38: "components",
  };
  const sumFor = (cat: "minerals" | "components" | "pi") =>
    active.filter((r) => categoryOf[r.itemId] === cat).reduce((sum, r) => sum + r.quantity, 0);

  // Only meaningful for operations 1–5 (they share id space with
  // build_projects / missing_materials); standalone operations like 6 have
  // none to compare against.
  const missingByOperation: Record<number, number> = { 1: 2, 2: 1, 3: 1, 4: 0, 5: 2 };

  return {
    reservedMinerals: sumFor("minerals"),
    reservedComponents: sumFor("components"),
    reservedPi: sumFor("pi"),
    missingReservations: missingByOperation[operationId] ?? 0,
  };
}

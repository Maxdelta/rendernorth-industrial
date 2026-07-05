//! Reservation domain DTOs. Mirror `src/lib/backend.ts` — change both
//! together. The Reservation Engine owns who owns inventory, reservation
//! quantities, reservation history, and reservation conflicts — nothing
//! here performs production math or resolves anything it detects.

use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReservationRecord {
    pub reservation_id: i64,
    pub item_id: i64,
    pub item_name: String,
    pub operation_id: Option<i64>,
    pub operation_goal: Option<String>,
    pub quantity: i64,
    pub reason: String,
    pub created_at: String,
    pub released_at: Option<String>,
    pub is_active: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReservationEvent {
    pub id: i64,
    pub reservation_id: i64,
    pub event_type: String, // reserved / released / transferred / expired
    pub quantity: i64,
    pub from_operation_id: Option<i64>,
    pub to_operation_id: Option<i64>,
    pub reason: String,
    pub created_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReservationDetail {
    pub record: ReservationRecord,
    pub history: Vec<ReservationEvent>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReservationConflict {
    pub item_id: i64,
    pub item_name: String,
    /// overlapping_operations / exceeds_stock / missing_inventory
    pub conflict_type: String,
    pub description: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReservationSummary {
    pub active_reservation_count: i64,
    pub total_reserved_quantity: i64,
    pub items_with_reservations: i64,
    pub conflict_count: i64,
}

/// Mission Control's "Inventory Commitment" panel. Every field is derived
/// live from current reservation + inventory state, never a stored
/// literal — same precedent as Operation Engine's `is_blocked` and
/// Inventory Engine's `coverage_for_operation`.
///
/// Definitions (see docs/architecture for the full writeup):
/// - `total_inventory`: sum of all on-hand quantity.
/// - `reserved`: raw sum of active reservation quantity (can exceed a
///   single item's stock — that excess is exactly what `blocked` reports).
/// - `available`: sum, per item, of `max(quantity - reserved, 0)`.
/// - `blocked`: sum, per item, of `max(reserved - quantity, 0)` — the
///   over-committed slice that can't actually be honored.
/// - `unallocated`: on-hand quantity for items with neither an active
///   reservation nor a soft allocation — completely untouched stock.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCommitment {
    pub total_inventory: i64,
    pub reserved: i64,
    pub available: i64,
    pub blocked: i64,
    pub unallocated: i64,
}

/// The Operations Workspace's Reservations section, scoped to one
/// operation. `missing_reservations` counts entries in that operation's
/// `missing_materials` (migration 0002) with no matching active
/// reservation yet — only meaningful for operations that share id space
/// with a `build_projects` row; always 0 for a standalone operation like
/// "Prepare Titan Components".
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationReservations {
    pub reserved_minerals: i64,
    pub reserved_components: i64,
    pub reserved_pi: i64,
    pub missing_reservations: i64,
}

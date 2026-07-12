//! IPC-facing inventory DTOs. Mirror `src/lib/backend.ts` — change both
//! together. Nothing here is specific to any ship, structure, or category;
//! `category_key` and `state` are always data, never a Rust type variant per
//! item kind.

use serde::{Deserialize, Serialize};

/// Real, user-entered inventory (Sprint 008) — kept in its own table
/// (`manual_inventory_entries`) rather than mixed into the demo-seeded
/// `inventory_items`, so the two can never be confused. Always tied to an
/// imported `eve_types` row, since a manual entry only makes sense once
/// static data names the type.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ManualInventoryEntry {
    pub id: i64,
    pub type_id: i64,
    pub type_name: String,
    pub quantity: i64,
    pub location_name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncedAsset {
    pub character_id: i64,
    pub character_owner: String,
    pub type_id: i64,
    pub type_name: String,
    pub quantity: i64,
    pub item_id: i64,
    pub location_id: i64,
    pub location_type: String,
    pub location_flag: String,
    pub singleton: bool,
    pub source: String,
    pub last_synced: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewManualInventoryEntry {
    pub type_id: i64,
    pub quantity: i64,
    pub location_name: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryCategory {
    pub key: String,
    pub label: String,
    pub sort_order: i64,
    pub item_count: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLocation {
    pub location_id: i64,
    pub name: String,
    pub kind: String,
    pub system_name: Option<String>,
    pub region_name: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    pub item_id: i64,
    pub type_name: String,
    pub category_key: String,
    pub category_label: String,
    pub quantity: i64,
    pub location_name: String,
    pub owner_name: String,
    pub unit_value: f64,
    pub total_value: f64,
    pub reserved_quantity: i64,
    pub available_quantity: i64,
    /// Nets out soft allocations too, not just hard reservations — "has
    /// nothing at all claimed against it," a stricter bar than
    /// `available_quantity`. See the computation site in
    /// `InventoryRepository::items` for the current caveat.
    pub free_quantity: i64,
    pub allocated_operation: Option<String>,
    pub reserved_operation: Option<String>,
    /// Lifecycle state key from `inventory_states` (available, reserved,
    /// manufacturing, research, reaction, in_transit, asset_safety,
    /// contract, delivery, destroyed…). Display label is derived, not
    /// hardcoded per category.
    pub state: String,
    pub state_label: String,
    /// Human-facing status combining state + reservation coverage —
    /// "Available" / "Partially Reserved" / "Reserved" / a lifecycle label.
    pub status: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventorySummary {
    pub total_assets: i64,
    pub estimated_value: f64,
    pub unique_item_types: i64,
    pub locations: i64,
    pub reserved_value: f64,
    pub available_value: f64,
}

/// Soft, plan-level earmark of inventory against an operation. Does not
/// reduce availability — the Reservation Engine does that. Read-only in
/// this sprint; nothing creates or removes rows from the app yet.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAllocation {
    pub id: i64,
    pub item_id: i64,
    pub project_id: i64,
    pub quantity: i64,
}

/// Hard hold against an operation, reducing available quantity while
/// `released_at` is unset. Read-only in this sprint.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventoryReservation {
    pub id: i64,
    pub item_id: i64,
    pub project_id: Option<i64>,
    pub quantity: i64,
    pub reason: String,
    pub released_at: Option<String>,
}

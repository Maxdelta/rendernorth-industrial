//! `ReservationEngine` — the layer the refined architecture places between
//! Operations and the Inventory Engine:
//!
//!   Mission Control -> Operations -> Reservation Engine -> Inventory Engine -> SQLite
//!
//! Operations never own inventory outright and never "move" items into
//! themselves. An operation asks this engine to hold a quantity against a
//! project; the Inventory Engine reflects that hold as reduced availability.
//!
//! Sprint 003 scope is architecture only: the trait below documents the
//! shape this layer will take, and `inventory_reservations` (migration
//! 0003) already has the rows a real implementation would read and write.
//! Nothing here is called by any Tauri command yet — Operations cannot
//! request a reservation through the UI this sprint. That lands with the
//! Production Engine once real requirement math exists to reserve against.

use super::models::InventoryReservation;

/// What an Operation is allowed to ask the inventory layer to do with a
/// quantity of a specific item. Implementations are expected to be
/// transactional and to never let `reserved_quantity` exceed `quantity` on
/// the underlying item.
#[allow(dead_code)]
pub trait ReservationEngine {
    /// Hold `quantity` of `item_id` against `project_id`. Returns the new
    /// reservation row on success.
    fn reserve(
        &self,
        item_id: i64,
        project_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<InventoryReservation, String>;

    /// Release a previously created reservation, returning its quantity to
    /// available.
    fn release(&self, reservation_id: i64) -> Result<(), String>;

    /// Total quantity currently held (across all operations) for one item.
    fn reserved_quantity(&self, item_id: i64) -> Result<i64, String>;
}

/// Placeholder implementation. Compiles and documents the seam; every
/// method returns an explicit "not yet" error rather than silently
/// succeeding, so nothing can accidentally depend on reservation writes
/// that don't really happen yet.
#[allow(dead_code)]
pub struct NotYetImplementedReservationEngine;

#[allow(unused_variables)]
impl ReservationEngine for NotYetImplementedReservationEngine {
    fn reserve(
        &self,
        item_id: i64,
        project_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<InventoryReservation, String> {
        Err("Reservation Engine is architecture-only in Sprint 003".into())
    }

    fn release(&self, reservation_id: i64) -> Result<(), String> {
        Err("Reservation Engine is architecture-only in Sprint 003".into())
    }

    fn reserved_quantity(&self, item_id: i64) -> Result<i64, String> {
        Err("Reservation Engine is architecture-only in Sprint 003".into())
    }
}

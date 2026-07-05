//! `ReservationEngine` — what Mission Control, the Operations Workspace,
//! and the Inventory page ask about who owns what. Mirrors
//! `inventory::engine::InventoryEngine` and
//! `operation::engine::OperationEngine`.
//!
//! Reads (summary, detail, conflicts, inventory commitment, per-operation
//! reservations) are real this sprint, backed by migration 0005. Mutations
//! (reserve/release/transfer) are declared here because the Reservation
//! Engine will own them — "owns who owns inventory, reservation
//! quantities, reservation history, reservation conflicts" — but each
//! returns an explicit "not yet" error. No business logic runs: nothing
//! decides whether a reservation *should* happen (that's the Decision
//! Engine's eventual job) and nothing here writes a reservation into
//! existence.

use super::models::{
    InventoryCommitment, OperationReservations, ReservationConflict, ReservationDetail,
    ReservationRecord, ReservationSummary,
};
use super::provider::{MockReservationProvider, ReservationProvider};
use rusqlite::Connection;

pub struct ReservationEngine<P: ReservationProvider> {
    provider: P,
}

impl<P: ReservationProvider> ReservationEngine<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    // ---- reads: live this sprint ----

    pub fn list_active(&self) -> Result<Vec<ReservationRecord>, String> {
        self.provider.list_active()
    }

    pub fn get_detail(&self, reservation_id: i64) -> Result<ReservationDetail, String> {
        self.provider.get_detail(reservation_id)
    }

    pub fn summary(&self) -> Result<ReservationSummary, String> {
        self.provider.summary()
    }

    /// Detect only. Never resolves a conflict — surfacing is the entire
    /// job. See `ReservationRepository::conflicts` for the three checks.
    pub fn conflicts(&self) -> Result<Vec<ReservationConflict>, String> {
        self.provider.conflicts()
    }

    pub fn inventory_commitment(&self) -> Result<InventoryCommitment, String> {
        self.provider.inventory_commitment()
    }

    pub fn operation_reservations(&self, operation_id: i64) -> Result<OperationReservations, String> {
        self.provider.operation_reservations(operation_id)
    }

    // ---- mutations: architecture-only this sprint ----
    // Declared now so the eventual implementation is a body swap, not an
    // API redesign. Every one of these is exactly the "business logic"
    // this sprint is told not to build — all return the same refusal.
    // #[allow(dead_code)] marks them as a deliberate reminder, not an
    // oversight — no command calls them yet.

    #[allow(dead_code, unused_variables)]
    pub fn reserve(
        &self,
        item_id: i64,
        operation_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<ReservationRecord, String> {
        Err("Reservation Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn release(&self, reservation_id: i64, reason: &str) -> Result<(), String> {
        Err("Reservation Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn transfer(
        &self,
        reservation_id: i64,
        to_operation_id: i64,
        reason: &str,
    ) -> Result<ReservationRecord, String> {
        Err("Reservation Engine mutations are architecture-only this sprint".into())
    }
}

/// Convenience constructor used by every command: today's engine is the
/// local demo-seeded provider over the shared SQLite connection.
pub fn engine_for(conn: &Connection) -> ReservationEngine<MockReservationProvider<'_>> {
    ReservationEngine::new(MockReservationProvider::new(conn))
}

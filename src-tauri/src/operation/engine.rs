//! `OperationEngine` — what Mission Control, the Operations Workspace, and
//! (later) the Decision Engine ask about Operations. Mirrors
//! `inventory::engine::InventoryEngine`.
//!
//! Read methods (list/queue/blocked/upcoming/health/detail) are real this
//! sprint, backed by migration 0004's seeded data. Mutation methods
//! (create/reprioritize/reschedule/annotate, and requesting a reservation)
//! are declared here because the Operation Engine will own them, but each
//! returns an explicit "not yet" error — no lifecycle mutation, reservation
//! request, or production logic is implemented this sprint. That is next
//! sprint's work, once the Reservation Engine is real.

use super::models::{OperationDetail, OperationHealth, OperationSummary};
use super::provider::{MockOperationProvider, OperationProvider};
use crate::inventory::models::InventoryReservation;
use rusqlite::Connection;

pub struct OperationEngine<P: OperationProvider> {
    provider: P,
}

impl<P: OperationProvider> OperationEngine<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    // ---- reads: live this sprint ----

    pub fn list_summaries(&self) -> Result<Vec<OperationSummary>, String> {
        self.provider.list_summaries()
    }

    pub fn priority_queue(&self) -> Result<Vec<OperationSummary>, String> {
        self.provider.priority_queue()
    }

    pub fn blocked(&self) -> Result<Vec<OperationSummary>, String> {
        self.provider.blocked()
    }

    pub fn upcoming_completions(&self, limit: i64) -> Result<Vec<OperationSummary>, String> {
        self.provider.upcoming_completions(limit)
    }

    pub fn health(&self) -> Result<OperationHealth, String> {
        self.provider.health()
    }

    pub fn get_detail(&self, operation_id: i64) -> Result<OperationDetail, String> {
        self.provider.get_detail(operation_id)
    }

    // ---- mutations: architecture-only this sprint ----
    // Declared on the engine now so the eventual implementation is a body
    // swap, not an API redesign. Every one of these is exactly the kind of
    // "production math / reservation logic" this sprint is told not to
    // build, so all return the same explicit refusal. #[allow(dead_code)]
    // marks them as a deliberate reminder, not an oversight — no command
    // calls them yet.

    #[allow(dead_code, unused_variables)]
    pub fn create_operation(
        &self,
        goal: &str,
        target_type_id: Option<i64>,
    ) -> Result<i64, String> {
        Err("Operation Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn set_priority(&self, operation_id: i64, priority: i64) -> Result<(), String> {
        Err("Operation Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn set_deadline(&self, operation_id: i64, deadline: Option<&str>) -> Result<(), String> {
        Err("Operation Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn set_notes(&self, operation_id: i64, notes: &str) -> Result<(), String> {
        Err("Operation Engine mutations are architecture-only this sprint".into())
    }

    /// The only way an Operation is ever meant to touch inventory: a
    /// request passed to the Reservation Engine. Implementations must not
    /// read or write inventory tables directly.
    #[allow(dead_code, unused_variables)]
    pub fn request_reservation(
        &self,
        operation_id: i64,
        item_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<InventoryReservation, String> {
        Err("Operation Engine mutations are architecture-only this sprint".into())
    }
}

/// Convenience constructor used by every command: today's engine is the
/// local demo-seeded provider over the shared SQLite connection.
pub fn engine_for(conn: &Connection) -> OperationEngine<MockOperationProvider<'_>> {
    OperationEngine::new(MockOperationProvider::new(conn))
}

//! `BlueprintEngine` — what Mission Control, the Blueprints page, and the
//! Operations Workspace ask about industrial capability. Mirrors
//! `inventory::engine::InventoryEngine`, `operation::engine::OperationEngine`,
//! and `reservation::engine::ReservationEngine`.
//!
//! Reads (list, detail, summary, missing report, readiness) are real this
//! sprint, backed by migration 0006. Mutations (research, copy, acquire)
//! are declared here because the Blueprint Engine will own them, but each
//! returns an explicit "not yet" error — no research/copy job logic,
//! production math, or acquisition runs through this engine yet.

use super::models::{
    BlueprintDetail, BlueprintReadiness, BlueprintRecord, BlueprintSummary, MissingBlueprintReport,
};
use super::provider::{BlueprintProvider, MockBlueprintProvider};
use rusqlite::Connection;

pub struct BlueprintEngine<P: BlueprintProvider> {
    provider: P,
}

impl<P: BlueprintProvider> BlueprintEngine<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    // ---- reads: live this sprint ----

    pub fn list(&self) -> Result<Vec<BlueprintRecord>, String> {
        self.provider.list()
    }

    pub fn get_detail(&self, blueprint_id: i64) -> Result<BlueprintDetail, String> {
        self.provider.get_detail(blueprint_id)
    }

    pub fn summary(&self) -> Result<BlueprintSummary, String> {
        self.provider.summary()
    }

    /// Detect only. Never acquires or researches anything — surfacing what's
    /// missing is the entire job.
    pub fn missing_report(&self) -> Result<Vec<MissingBlueprintReport>, String> {
        self.provider.missing_report()
    }

    pub fn readiness_for_operation(&self, operation_id: i64) -> Result<BlueprintReadiness, String> {
        self.provider.readiness_for_operation(operation_id)
    }

    pub fn readiness_all(&self) -> Result<Vec<BlueprintReadiness>, String> {
        self.provider.readiness_all()
    }

    // ---- mutations: architecture-only this sprint ----
    // Declared now so the eventual implementation is a body swap, not an
    // API redesign. Every one of these is exactly the "research/copy
    // mutation" and "automation" territory this sprint is told not to
    // build, so all return the same refusal. #[allow(dead_code)] marks
    // them as a deliberate reminder, not an oversight — no command calls
    // them yet.

    #[allow(dead_code, unused_variables)]
    pub fn start_research(&self, blueprint_id: i64, target_level: i64) -> Result<(), String> {
        Err("Blueprint Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn start_copy(&self, blueprint_id: i64, runs: i64) -> Result<(), String> {
        Err("Blueprint Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn acquire(&self, type_name: &str, is_copy: bool) -> Result<BlueprintRecord, String> {
        Err("Blueprint Engine mutations are architecture-only this sprint".into())
    }
}

/// Convenience constructor used by every command: today's engine is the
/// local demo-seeded provider over the shared SQLite connection.
pub fn engine_for(conn: &Connection) -> BlueprintEngine<MockBlueprintProvider<'_>> {
    BlueprintEngine::new(MockBlueprintProvider::new(conn))
}

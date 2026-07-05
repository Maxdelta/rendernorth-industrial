//! Operation domain — the object every other system in RenderNorth
//! Industrial revolves around. An Operation is industrial intent ("Build
//! Avatar", "Manufacture 500 Capital Construction Parts", "Stockpile
//! Broadcast Nodes"); it owns its lifecycle, priority, status, timeline,
//! notes, target, progress, and dependencies, and it never owns inventory.
//!
//! Architecture (see docs/SYSTEM_ARCHITECTURE.md):
//!
//!   Mission Control -> Operation Engine -> Reservation Engine -> Inventory Engine -> SQLite
//!
//! `models`/`repository`/`provider`/`engine` mirror the Inventory module's
//! shape exactly, per the Sprint 004 architectural-discipline instruction
//! to keep new domains consistent with existing ones.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;

pub use engine::{engine_for, OperationEngine};

// Re-exported for external callers that need the type name directly;
// nothing does yet, hence the allow.
#[allow(unused_imports)]
pub use models::OperationSummary;

/// The narrow, read-only shape the Decision Engine (`crate::decision`)
/// depends on. Kept intentionally smaller than the full `OperationEngine`
/// API: a decision rule may read an operation's state, never mutate it —
/// mutation stays behind `OperationEngine`'s own methods (architecture-only
/// this sprint) and, eventually, the Reservation Engine.
#[allow(dead_code)]
pub trait OperationQuery {
    fn profile(&self, operation_id: i64) -> Result<OperationProfile, String>;
    fn is_active(&self, operation_id: i64) -> Result<bool, String>;
}

/// The subset of an Operation's detail a decision rule is likely to need.
/// Deliberately smaller than `OperationDetail` — no notes, no timeline.
#[allow(dead_code)]
pub struct OperationProfile {
    pub operation_id: i64,
    pub goal: String,
    pub priority: i64,
    pub status: String,
    pub progress: f64,
    pub is_blocked: bool,
}

impl<P: provider::OperationProvider> OperationQuery for OperationEngine<P> {
    fn profile(&self, operation_id: i64) -> Result<OperationProfile, String> {
        let detail = self.get_detail(operation_id)?;
        Ok(OperationProfile {
            operation_id: detail.operation_id,
            goal: detail.goal,
            priority: detail.priority,
            status: detail.status,
            progress: detail.progress,
            is_blocked: detail.is_blocked,
        })
    }

    fn is_active(&self, operation_id: i64) -> Result<bool, String> {
        Ok(self.get_detail(operation_id)?.status == "active")
    }
}

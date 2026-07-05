//! Operation Engine — interface only, introduced this sprint as a
//! first-class architecture concept per the refined hierarchy:
//!
//!   Mission Control -> Operation Engine -> Reservation Engine -> Inventory Engine -> SQLite
//!
//! An Operation owns its own metadata — goal, deadline, priority, notes,
//! selected build target, timeline, production plan — and nothing else. It
//! never owns inventory and never touches inventory tables directly; it
//! asks the Reservation Engine to hold quantity on its behalf, and the
//! Inventory Engine remains the only source of truth for what exists and
//! what's available.
//!
//! Nothing in this file is wired to a command yet. `build_projects`
//! (migrations 0001–0002) remains the live table backing today's Mission
//! Control / Build Targets pages; it currently carries only a subset of
//! what `OperationProfile` below describes (name, target label, status,
//! coverage). Extending that table — deadlines, priority, notes, a real
//! timeline and production plan — is deferred to when this engine is
//! actually implemented, so this sprint adds no migration.

use crate::inventory::models::InventoryReservation;

/// The full shape an Operation will eventually carry. This is a forward
/// sketch, not a live DTO: only `project_id` and a name/target concept
/// exist as real columns today (see DATABASE_SCHEMA.md, `build_projects`).
#[allow(dead_code)]
pub struct OperationProfile {
    pub project_id: i64,
    pub goal: String,
    pub deadline: Option<String>,
    pub priority: i64,
    pub notes: String,
    pub selected_build_target_id: Option<i64>,
    /// Ordered milestones; shape TBD alongside the Production Engine.
    pub timeline: Vec<String>,
}

/// What anything above this layer (Mission Control, the Decision Engine)
/// is allowed to ask about an Operation. Read-only by design — mutating an
/// operation goes through `OperationEngine`, not this trait.
#[allow(dead_code)]
pub trait OperationQuery {
    fn profile(&self, project_id: i64) -> Result<OperationProfile, String>;
    fn is_active(&self, project_id: i64) -> Result<bool, String>;
}

/// What an Operation is allowed to do. It manages its own metadata and
/// requests reservations through the Reservation Engine — it never calls
/// into `InventoryEngine` or touches inventory tables itself. This is the
/// enforcement point for "Operations do not own inventory."
#[allow(dead_code)]
pub trait OperationEngine {
    fn create_operation(&self, goal: &str, target_type_id: Option<i64>) -> Result<i64, String>;
    fn set_priority(&self, project_id: i64, priority: i64) -> Result<(), String>;
    fn set_deadline(&self, project_id: i64, deadline: Option<&str>) -> Result<(), String>;
    fn set_notes(&self, project_id: i64, notes: &str) -> Result<(), String>;

    /// The only way an Operation touches inventory: a request passed
    /// through to the Reservation Engine. Implementations must not read or
    /// write inventory tables directly — they delegate.
    fn request_reservation(
        &self,
        project_id: i64,
        item_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<InventoryReservation, String>;
}

/// Placeholder implementation. Every method returns an explicit "not yet"
/// error so nothing can accidentally depend on operation writes that don't
/// really happen — same pattern as `inventory::reservation`.
#[allow(dead_code)]
pub struct NotYetImplementedOperationEngine;

#[allow(unused_variables)]
impl OperationEngine for NotYetImplementedOperationEngine {
    fn create_operation(&self, goal: &str, target_type_id: Option<i64>) -> Result<i64, String> {
        Err("Operation Engine is architecture-only".into())
    }

    fn set_priority(&self, project_id: i64, priority: i64) -> Result<(), String> {
        Err("Operation Engine is architecture-only".into())
    }

    fn set_deadline(&self, project_id: i64, deadline: Option<&str>) -> Result<(), String> {
        Err("Operation Engine is architecture-only".into())
    }

    fn set_notes(&self, project_id: i64, notes: &str) -> Result<(), String> {
        Err("Operation Engine is architecture-only".into())
    }

    fn request_reservation(
        &self,
        project_id: i64,
        item_id: i64,
        quantity: i64,
        reason: &str,
    ) -> Result<InventoryReservation, String> {
        Err("Operation Engine is architecture-only".into())
    }
}

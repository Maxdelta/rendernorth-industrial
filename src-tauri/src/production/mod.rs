//! Production Requirement Engine — a sibling to Inventory, Operation,
//! Reservation, and Blueprint. Answers "what does this operation actually
//! require to complete?" It owns required inputs — never inventory,
//! reservation state, operation lifecycle, or blueprint capability, all of
//! which stay the job of their own engines.
//!
//! Preserved hierarchy (see docs/SYSTEM_ARCHITECTURE.md):
//!
//!   Inventory Engine owns what exists.
//!   Reservation Engine owns who owns inventory.
//!   Operation Engine owns work.
//!   Blueprint Engine owns industrial capability.
//!   Production Requirement Engine owns required inputs.
//!   Decision Engine remains deterministic interface-only.
//!
//! `models`/`repository`/`provider`/`engine` mirror the other four
//! modules' shape exactly, per this sprint's instruction to extend the
//! existing system rather than redesign it. Coverage and shortage are
//! always derived live from `production_requirements` joined against
//! `inventory_items` — never a stored, editable flag.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;

// Re-exported for external callers that need the type name directly;
// nothing does yet, hence the allow — same pattern as InventoryEngine,
// OperationEngine, ReservationEngine, and BlueprintEngine.
#[allow(unused_imports)]
pub use engine::{engine_for, ProductionEngine};

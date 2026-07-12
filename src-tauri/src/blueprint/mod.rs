//! Blueprint Engine — a sibling to Inventory, Operation, and Reservation.
//! Owns industrial capability: what blueprints exist, their BPO/BPC
//! nature, ME/TE level, runs remaining, research/copy status, and whether
//! an operation's required blueprints are actually on hand. It never
//! touches inventory rows, reservation rows, or operation lifecycle —
//! those stay the job of their own engines.
//!
//! Preserved hierarchy (see docs/SYSTEM_ARCHITECTURE.md):
//!
//!   Inventory Engine owns what exists.
//!   Reservation Engine owns who owns inventory.
//!   Operation Engine owns work.
//!   Blueprint Engine owns industrial capability.
//!   Decision Engine remains deterministic interface-only.
//!
//! `models`/`repository`/`provider`/`engine` mirror the other three
//! modules' shape exactly, per this sprint's instruction to extend the
//! existing system rather than redesign it.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;
pub mod sync;

// Re-exported for external callers that need the type name directly;
// nothing does yet, hence the allow — same pattern as InventoryEngine,
// OperationEngine, and ReservationEngine.
#[allow(unused_imports)]
pub use engine::{engine_for, BlueprintEngine};

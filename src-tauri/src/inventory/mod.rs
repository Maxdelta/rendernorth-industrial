//! Inventory Engine — the single data core every other system queries.
//!
//! Architecture (see docs/architecture/SYSTEM_ARCHITECTURE.md):
//!
//!   Mission Control -> Operation Engine -> Reservation Engine -> Inventory Engine -> SQLite
//!
//! Operations never own inventory; they request reservations against it
//! through `crate::reservation` (the Reservation Engine, a sibling module,
//! not nested here). The Inventory Engine is the only thing that reads/
//! writes inventory rows directly. Nothing in this module knows what a
//! Titan, a Revelation, or an Avatar is — every query is generic over
//! category, location, and owner.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;

// `InventoryEngine` is re-exported for external callers that need the type
// name directly (e.g. a future stateful Tauri-managed engine instance);
// nothing does yet, hence the allow.
#[allow(unused_imports)]
pub use engine::{engine_for, InventoryEngine};

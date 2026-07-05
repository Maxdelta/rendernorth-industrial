//! Inventory Engine — the single data core every other system queries.
//!
//! Architecture (see docs/architecture/SYSTEM_ARCHITECTURE.md):
//!
//!   Mission Control -> Operations -> Reservation Engine -> Inventory Engine -> SQLite
//!
//! Operations never own inventory; they request reservations against it.
//! The Inventory Engine is the only thing that reads/writes inventory rows.
//! Nothing in this module knows what a Titan, a Revelation, or an Avatar is
//! — every query is generic over category, location, and owner.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;
pub mod reservation;

pub use engine::{engine_for, InventoryEngine};

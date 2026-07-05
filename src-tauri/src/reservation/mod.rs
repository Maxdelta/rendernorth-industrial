//! Reservation Engine — sits between the Inventory Engine and the
//! Operation Engine:
//!
//!   Inventory Engine -> Reservation Engine -> Operation Engine -> Decision Engine
//!
//! Inventory Engine owns what exists. Reservation Engine owns who owns
//! inventory, reservation quantities, reservation history, and reservation
//! conflicts. Operation Engine owns work and references reservations —
//! it never touches inventory tables directly (see `operation::engine`'s
//! `request_reservation` stub, which is exactly the call this engine will
//! answer once its own mutations are implemented). Decision Engine remains
//! interface-only.
//!
//! `models`/`repository`/`provider`/`engine` mirror the Inventory and
//! Operation modules' shape exactly, per this sprint's instruction to
//! extend the existing architecture rather than redesign it.

pub mod engine;
pub mod models;
pub mod provider;
pub mod repository;

pub use engine::{engine_for, ReservationEngine};

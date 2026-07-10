//! Static Data Import — a sibling to Inventory, Operation, Reservation,
//! Blueprint, and Production. Owns official reference-data ingestion:
//! EVE types, groups, categories, and blueprint activity/product/material
//! data, loaded from a locally-selected file. No network access, ever.
//!
//! Scope this sprint: the CSV-derivative SDE format (see
//! `repository.rs` doc comment), manufacturing activity only. See
//! docs/REAL_PRODUCTION_PLANNER.md for the full format specification and
//! known limitations.

pub mod importer;
pub mod jsonl;
pub mod models;
pub mod repository;

pub use importer::{import_from_directory, import_official_sde, latest_import, search_types};

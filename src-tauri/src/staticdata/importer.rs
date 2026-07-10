//! Static Data Import — owns official reference-data ingestion only. It
//! never touches operations, inventory, reservations, or settings.
//!
//! Unlike Inventory/Operation/Reservation/Blueprint/Production, this
//! module has no provider/engine trait indirection: there is exactly one
//! data source (a local file the user selects) and no alternate source to
//! swap in later, so the extra abstraction layer would be ceremony
//! without purpose. `importer.rs` is the thin entry point every command
//! calls through, mirroring the spirit of the other modules'
//! `engine_for()` convenience constructors without the unused trait.

use super::models::{ImportSummary, TypeSearchResult};
use super::repository::StaticDataRepository;
use rusqlite::Connection;

pub fn import_from_directory(conn: &Connection, dir_path: &str) -> Result<ImportSummary, String> {
    StaticDataRepository::new(conn).import_from_directory(dir_path)
}

/// Official CCP JSON Lines SDE import (Sprint 008.2) — see
/// `jsonl.rs` for the full field-mapping disclosure and known
/// limitations. Separate entry point from `import_from_directory`; the
/// sample CSV fixture path is untouched by this addition.
pub fn import_official_sde(conn: &Connection, dir_path: &str) -> Result<ImportSummary, String> {
    super::jsonl::import_official_sde(&StaticDataRepository::new(conn), dir_path)
}

pub fn latest_import(conn: &Connection) -> Result<ImportSummary, String> {
    StaticDataRepository::new(conn).latest_import()
}

pub fn search_types(conn: &Connection, query: &str, limit: i64) -> Result<Vec<TypeSearchResult>, String> {
    StaticDataRepository::new(conn).search_types(query, limit)
}

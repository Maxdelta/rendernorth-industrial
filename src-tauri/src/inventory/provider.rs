//! `InventoryProvider` — the seam between the Inventory Engine and wherever
//! inventory data actually comes from. Sprint 003 ships one real
//! implementation (`MockInventoryProvider`, backed by the demo-seeded
//! SQLite rows from migration 0003) plus a stub for the next one. Adding a
//! source later means adding a struct here, not touching the engine or any
//! command.

use super::models::{InventoryCategory, InventoryItem, InventoryLocation, InventorySummary};
use super::repository::InventoryRepository;
use rusqlite::Connection;

/// A source of inventory truth. Every method is generic over category and
/// location — no implementation may special-case a ship, structure, or
/// item type.
pub trait InventoryProvider {
    fn summary(&self) -> Result<InventorySummary, String>;
    fn categories(&self) -> Result<Vec<InventoryCategory>, String>;
    fn items(&self, category_key: Option<&str>) -> Result<Vec<InventoryItem>, String>;
    /// Read path for a future Locations breakdown view; no command surfaces
    /// it yet, hence the allow — this is a reminder, not a mistake.
    #[allow(dead_code)]
    fn locations(&self) -> Result<Vec<InventoryLocation>, String>;
    fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String>;
}

/// Demo data, seeded straight into SQLite by migration 0003. Despite the
/// name this is the live provider for Sprint 003 — "mock" describes the
/// data's origin (hand-seeded, not synced), not a separate code path from
/// what a real provider would do. A future `EsiInventoryProvider` fulfills
/// the exact same trait against live ESI-synced rows in the same tables.
pub struct MockInventoryProvider<'a> {
    repo: InventoryRepository<'a>,
}

impl<'a> MockInventoryProvider<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: InventoryRepository::new(conn),
        }
    }
}

impl<'a> InventoryProvider for MockInventoryProvider<'a> {
    fn summary(&self) -> Result<InventorySummary, String> {
        self.repo.summary()
    }

    fn categories(&self) -> Result<Vec<InventoryCategory>, String> {
        self.repo.categories()
    }

    fn items(&self, category_key: Option<&str>) -> Result<Vec<InventoryItem>, String> {
        self.repo.items(category_key)
    }

    fn locations(&self) -> Result<Vec<InventoryLocation>, String> {
        self.repo.locations()
    }

    fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String> {
        self.repo.coverage_for_operation(project_id)
    }
}

/// Not implemented this sprint. Exists so `InventoryEngine` can be built
/// against the official CCP ESI API later by swapping the provider, with
/// zero changes to the engine, repository shape, or any command signature.
/// Per the Product Constitution, this will only ever call the official ESI
/// endpoints under the OAuth PKCE flow documented in ESI_INTEGRATION.md —
/// never the EVE client, never a gameplay automation path.
#[allow(dead_code)]
pub struct EsiInventoryProvider;

#[allow(unused_variables)]
impl InventoryProvider for EsiInventoryProvider {
    fn summary(&self) -> Result<InventorySummary, String> {
        Err("ESI inventory sync is not implemented yet".into())
    }
    fn categories(&self) -> Result<Vec<InventoryCategory>, String> {
        Err("ESI inventory sync is not implemented yet".into())
    }
    fn items(&self, category_key: Option<&str>) -> Result<Vec<InventoryItem>, String> {
        Err("ESI inventory sync is not implemented yet".into())
    }
    fn locations(&self) -> Result<Vec<InventoryLocation>, String> {
        Err("ESI inventory sync is not implemented yet".into())
    }
    fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String> {
        Err("ESI inventory sync is not implemented yet".into())
    }
}

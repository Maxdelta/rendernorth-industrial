//! `InventoryEngine` — the one thing Mission Control, Operations, Production,
//! Industry, Logistics, Planning, and the Recommendation Engine ever ask
//! about inventory. It holds a provider and exposes generic questions;
//! nothing above this layer touches SQL, and nothing here knows a Titan
//! from a mineral.

use super::models::{InventoryCategory, InventoryItem, InventoryLocation, InventorySummary};
use super::provider::{InventoryProvider, MockInventoryProvider};
use rusqlite::Connection;

pub struct InventoryEngine<P: InventoryProvider> {
    provider: P,
}

impl<P: InventoryProvider> InventoryEngine<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// What do I own, in aggregate: total quantity, estimated value, unique
    /// types, known locations, and the reserved/available value split.
    pub fn summary(&self) -> Result<InventorySummary, String> {
        self.provider.summary()
    }

    /// The category rail: every category is a view, never a separate store.
    pub fn categories(&self) -> Result<Vec<InventoryCategory>, String> {
        self.provider.categories()
    }

    /// Where is it, who owns it, is it reserved or allocated — for one
    /// category, or `None` for the "Everything" view.
    pub fn items(&self, category_key: Option<&str>) -> Result<Vec<InventoryItem>, String> {
        self.provider.items(category_key)
    }

    /// Read path for a future Locations breakdown view; no command
    /// surfaces it yet, hence the allow — this is a reminder, not a mistake.
    #[allow(dead_code)]
    pub fn locations(&self) -> Result<Vec<InventoryLocation>, String> {
        self.provider.locations()
    }

    /// "Can this operation start? How covered is it?" — the question
    /// Operations ask instead of reading a hardcoded percentage. See
    /// `InventoryRepository::coverage_for_operation` for the current
    /// (placeholder, tier-average) formula and its Sprint 004 successor.
    pub fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String> {
        self.provider.coverage_for_operation(project_id)
    }
}

/// Convenience constructor used by every command: today's engine is the
/// mock/demo provider over the shared SQLite connection. Swapping to a live
/// ESI-backed engine later is a one-line change here, not a rewrite of every
/// call site.
pub fn engine_for(conn: &Connection) -> InventoryEngine<MockInventoryProvider<'_>> {
    InventoryEngine::new(MockInventoryProvider::new(conn))
}

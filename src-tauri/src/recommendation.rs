//! Recommendation Engine — interface only.
//!
//! Sprint 003 scope is explicit: build the seam the Recommendation Engine
//! will use to ask the Inventory Engine questions, not the rule logic
//! itself. Real rules (REC-xxx) still live as seeded, deterministic rows in
//! `recommendations` (see migrations 0001–0002) and are read verbatim by
//! `commands::mission_control`. Nothing in this file runs yet.
//!
//! When the Recommendation Engine is built (Sprint 005 per
//! SYSTEM_ARCHITECTURE.md), it will depend on this trait rather than on
//! `InventoryEngine` directly, so recommendation rules stay testable
//! against a fake inventory without touching SQLite.

use crate::inventory::models::InventoryItem;

/// The subset of Inventory Engine questions a recommendation rule is
/// allowed to ask. Deliberately narrow — recommendation rules read
/// inventory state, they never write it (that stays the Reservation
/// Engine's job, once it exists).
#[allow(dead_code)]
pub trait InventoryQuery {
    /// Every item backing a category, for rules that reason over a whole
    /// class of material (e.g. "all capital components below target").
    fn items_in_category(&self, category_key: &str) -> Result<Vec<InventoryItem>, String>;

    /// How covered a build target is right now, per
    /// `InventoryEngine::coverage_for_operation`.
    fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String>;
}

/// A single deterministic recommendation, shaped exactly like the rows
/// already stored in `recommendations` (title / rule id / summary /
/// inputs). Rule implementations will eventually produce these from an
/// `InventoryQuery`; none do yet.
#[allow(dead_code)]
pub struct DeterministicRecommendation {
    pub rule_id: String,
    pub title: String,
    pub summary: String,
    pub inputs: serde_json::Value,
}

/// A recommendation rule: pure function of whatever an `InventoryQuery`
/// returns, for one build target. No AI, no nondeterminism — a rule that
/// cannot be described in one paragraph does not belong here (Product
/// Constitution, Determinism Doctrine).
#[allow(dead_code)]
pub trait RecommendationRule {
    fn rule_id(&self) -> &'static str;
    fn evaluate(
        &self,
        inventory: &dyn InventoryQuery,
        project_id: i64,
    ) -> Result<Option<DeterministicRecommendation>, String>;
}

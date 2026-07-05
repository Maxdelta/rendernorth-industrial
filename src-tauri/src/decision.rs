//! Decision Engine — interface only. Renamed from "Recommendation Engine"
//! to reflect its actual job: answering build/buy/mine/sell/research/copy/
//! react decisions, not just surfacing a single "next thing to do" line.
//!
//! Scope is still explicit: build the seam the Decision Engine will use to
//! ask the Inventory Engine and Operation Engine questions, not the rule
//! logic itself. Real decisions today still live as seeded, deterministic
//! rows in the `recommendations` table (migrations 0001–0002) and are read
//! verbatim by `commands::mission_control` — that live feature is untouched
//! this sprint; it's the working ancestor this engine will eventually
//! replace, not something to regenerate early. Nothing in this file runs.
//!
//! Determinism Doctrine still applies without exception: a `DecisionRule`
//! is a pure function of its `DecisionContext`. LLMs may explain a decision
//! after the fact — never produce one.

use crate::inventory::models::InventoryItem;
use crate::operation::OperationQuery;
use serde::Serialize;

/// The subset of Inventory Engine questions a decision rule is allowed to
/// ask. Deliberately narrow — rules read inventory state, they never write
/// it (that stays the Reservation Engine's job).
#[allow(dead_code)]
pub trait InventoryQuery {
    /// Every item backing a category, for rules that reason over a whole
    /// class of material (e.g. "all capital components below target").
    fn items_in_category(&self, category_key: &str) -> Result<Vec<InventoryItem>, String>;

    /// How covered a build target is right now, per
    /// `InventoryEngine::coverage_for_operation`.
    fn coverage_for_operation(&self, project_id: i64) -> Result<f64, String>;
}

/// Everything a `DecisionRule` gets to see: inventory state plus the
/// requesting Operation's own metadata. Bundled so a rule takes one
/// argument instead of two, and so adding a third query source later
/// doesn't change every rule's signature.
#[allow(dead_code)]
pub struct DecisionContext<'a> {
    pub inventory: &'a dyn InventoryQuery,
    pub operations: &'a dyn OperationQuery,
}

/// The action family a decision falls into. Matches the operator questions
/// in the Product Constitution: what should I build, buy, mine, sell,
/// research, copy, or react.
#[allow(dead_code)]
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Build,
    Buy,
    Mine,
    Sell,
    Research,
    Copy,
    React,
}

/// A single deterministic decision, shaped like the rows already stored in
/// `recommendations` (title / rule id / summary / inputs) plus the action
/// kind. Rule implementations will eventually produce these from a
/// `DecisionContext`; none do yet.
#[allow(dead_code)]
pub struct Decision {
    pub rule_id: String,
    pub kind: DecisionKind,
    pub title: String,
    pub summary: String,
    pub inputs: serde_json::Value,
}

/// A decision rule: pure function of a `DecisionContext`, for one
/// operation. No AI, no nondeterminism — a rule that cannot be described in
/// one paragraph does not belong here (Product Constitution, Determinism
/// Doctrine). An LLM may narrate `summary` in friendlier prose later; it
/// may never decide `kind`, `title`, or whether the rule fires.
#[allow(dead_code)]
pub trait DecisionRule {
    fn rule_id(&self) -> &'static str;
    fn evaluate(
        &self,
        ctx: &DecisionContext,
        project_id: i64,
    ) -> Result<Option<Decision>, String>;
}

/// Runs the registered `DecisionRule`s for one operation and returns
/// whatever fires, highest priority first. No implementation exists yet —
/// this is the shape callers (Mission Control, later Planning/Intelligence
/// pages) will depend on once rules are written.
#[allow(dead_code)]
pub trait DecisionEngine {
    fn decide(&self, project_id: i64) -> Result<Vec<Decision>, String>;
}

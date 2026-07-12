//! Production Requirement domain DTOs. Mirror `src/lib/backend.ts` —
//! change both together. The Production Requirement Engine owns required
//! inputs: what an operation actually needs to complete. It never owns
//! inventory, reservations, operation lifecycle, or blueprints — coverage
//! and shortage are always computed live against `inventory_items`, never
//! a stored flag.

use serde::Serialize;

/// One node of a real, blueprint-derived requirement tree (Sprint 008).
/// Distinct from `RequirementLine` (Sprint 007's flat, demo-seeded
/// ledger) — this is computed recursively from imported static data, not
/// read from a stored `production_requirements` row.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementTreeNode {
    pub type_id: i64,
    pub type_name: String,
    pub is_leaf: bool,
    pub runs: i64,
    pub produced_quantity: i64,
    pub needed_quantity: i64,
    pub per_run_quantity: i64,
    pub owned_quantity: i64,
    /// Which inventory source contributed `owned_quantity` — "Manual
    /// Inventory" today; a future ESI-backed source would extend this
    /// label set, not replace it. Demo inventory never contributes to a
    /// real operation's plan, so it's never one of these values.
    pub owned_source: String,
    pub reserved_quantity: i64,
    pub available_quantity: i64,
    pub missing_quantity: i64,
    pub coverage_fraction: f64,
    pub is_satisfied: bool,
    pub me_applied: i64,
    /// "owned" (from a Blueprint Engine record) or "assumed" (ME0 default
    /// for sub-components with no owned blueprint, or the operation's own
    /// assumption at the root).
    pub me_source: String,
    /// Sprint 009 Validation Mode — the blueprint that produces this
    /// node, if it isn't a leaf. `None` for leaves (nothing manufactures
    /// a raw material). Distinct from `type_id`: in real EVE a
    /// blueprint's own type id can differ from what it produces.
    pub blueprint_type_id: Option<i64>,
    /// Fixed at "manufacturing" this sprint — the only activity this
    /// engine calculates. Present as its own field so a future reaction/
    /// invention activity is additive, not a breaking rename.
    pub activity: String,
    /// The ancestor chain that led to this node, e.g. "Apostle > Capital
    /// Armor Plates > Mechanical Parts" — engineering-verification detail,
    /// not shown outside Validation Mode.
    pub calculation_path: String,
    pub children: Vec<RequirementTreeNode>,
}

/// A leaf material's totals, summed across every path that requires it
/// (shared subcomponents included) — the flattened shape the shortage
/// export and the Production page's leaf-material list read from.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LeafTotal {
    pub type_id: i64,
    pub type_name: String,
    /// Added Sprint 008.2 for export — looked up separately from
    /// `eve_groups`/`eve_categories` after the recursive expansion
    /// finishes; never touches the expansion/ME/run-rounding logic itself.
    pub group_name: Option<String>,
    pub category_name: Option<String>,
    /// Sprint 009 Validation Mode — every distinct ancestor chain that
    /// contributed to this leaf's total. A shared subcomponent (like
    /// Mineral A required both directly and three levels deep) has more
    /// than one entry here; the required_quantity below is their sum.
    pub calculation_paths: Vec<String>,
    pub required_quantity: i64,
    pub owned_quantity: i64,
    pub owned_source: String,
    pub reserved_quantity: i64,
    pub available_quantity: i64,
    pub missing_quantity: i64,
    pub coverage_fraction: f64,
    pub is_satisfied: bool,
}

/// A fully calculated production plan for one operation's real build
/// target. Always recomputed live — never read back from
/// `production_plan_snapshots`, which exists purely for audit history.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductionPlan {
    pub operation_id: i64,
    pub operation_goal: String,
    pub build_target_type_id: i64,
    pub build_target_name: String,
    pub requested_quantity: i64,
    pub blueprint_mode: String,
    pub me: i64,
    pub te: i64,
    pub total_runs: i64,
    pub produced_quantity: i64,
    /// Captured but not yet enforced — see `operation_build_targets.inventory_scope`.
    /// The plan is always calculated against all manual inventory
    /// regardless of the chosen scope until location-aware filtering
    /// (which needs real location data, not yet available without ESI)
    /// is implemented.
    pub inventory_scope: String,
    pub tree: RequirementTreeNode,
    pub leaf_totals: Vec<LeafTotal>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementLine {
    pub requirement_id: i64,
    pub operation_id: i64,
    pub operation_goal: String,
    pub category_key: String,
    pub category_label: String,
    pub type_name: String,
    pub required_quantity: i64,
    /// Total owned across all locations/characters — a global figure, the
    /// same for every requirement line referencing this type name.
    pub owned_quantity: i64,
    /// Quantity specifically reserved for this operation and this
    /// material (via the Reservation Engine), where applicable.
    pub reserved_for_operation: i64,
    pub shortage: i64,
    /// 0.0–1.0, `owned_quantity / required_quantity` clamped at 1.0.
    pub coverage_fraction: f64,
    pub is_satisfied: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementSource {
    pub source_kind: String,
    pub contributed_quantity: i64,
    pub note: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementDetail {
    pub line: RequirementLine,
    pub sources: Vec<RequirementSource>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementCategory {
    pub category_key: String,
    pub category_label: String,
    pub sort_order: i64,
}

/// One row of the global shortage report: a requirement line that isn't
/// currently satisfied, anywhere.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementShortage {
    pub type_name: String,
    pub operation_goal: String,
    pub required_quantity: i64,
    pub owned_quantity: i64,
    pub shortage: i64,
}

/// A material required by more than one operation, unmet in at least one
/// of them — a genuine cross-operation bottleneck, always computed live.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CriticalBottleneck {
    pub type_name: String,
    pub operations_requiring: Vec<String>,
    pub owned_quantity: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RequirementSummary {
    pub total_requirements: i64,
    pub satisfied: i64,
    pub missing: i64,
    /// Weighted: sum(min(owned, required)) / sum(required) * 100, 0 when
    /// there are no requirements.
    pub coverage_percent: f64,
    pub critical_bottleneck_count: i64,
}

/// One category's rollup within an operation — the Operations Workspace's
/// Minerals/Components/Advanced Components/PI/Reaction Materials buckets.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCoverage {
    pub category_key: String,
    pub category_label: String,
    pub required_total: i64,
    pub owned_total: i64,
    pub coverage_fraction: f64,
    pub missing_count: i64,
}

/// The Operations Workspace's Production Requirements section, and the
/// Production page's per-target breakdown (same shape — see
/// `engine::requirements_for_build_target`).
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationRequirementBreakdown {
    pub operation_id: i64,
    pub operation_goal: String,
    pub categories: Vec<CategoryCoverage>,
    pub lines: Vec<RequirementLine>,
    pub total_requirements: i64,
    pub missing_count: i64,
    pub coverage_percent: f64,
}

//! Production Requirement domain DTOs. Mirror `src/lib/backend.ts` —
//! change both together. The Production Requirement Engine owns required
//! inputs: what an operation actually needs to complete. It never owns
//! inventory, reservations, operation lifecycle, or blueprints — coverage
//! and shortage are always computed live against `inventory_items`, never
//! a stored flag.

use serde::Serialize;

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

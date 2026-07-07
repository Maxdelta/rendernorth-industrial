//! Blueprint domain DTOs. Mirror `src/lib/backend.ts` — change both
//! together. The Blueprint Engine owns industrial capability: what
//! blueprints exist, their research/copy state, and whether an operation's
//! required blueprints are actually on hand. It never touches inventory,
//! reservation, or operation-lifecycle tables.

use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintRecord {
    pub blueprint_id: i64,
    pub type_name: String,
    pub is_copy: bool,
    pub me_level: i64,
    pub te_level: i64,
    pub runs_remaining: Option<i64>,
    pub owner_name: String,
    pub location_name: String,
    pub status: String, // idle / researching / copying / in_use
    pub linked_operation: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintDetail {
    pub record: BlueprintRecord,
    /// Every operation that lists this blueprint's type name as a
    /// requirement, whether or not this specific copy satisfies it.
    pub required_by: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintSummary {
    pub total_blueprints: i64,
    pub bpo_count: i64,
    pub bpc_count: i64,
    /// Blueprints not currently mid-research (status != "researching").
    pub research_complete: i64,
    /// Total remaining production runs summed across every BPC.
    pub copies: i64,
    /// Distinct blueprint type names required by some operation but not
    /// owned by any blueprint record, across all operations.
    pub missing_for_operations: i64,
}

/// One blueprint requirement row for an operation, joined against whether
/// it's actually owned right now. Never a stored flag — always computed
/// live from `operation_blueprint_requirements` + `blueprints`.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintRequirement {
    pub type_name: String,
    pub reason: String,
    pub is_owned: bool,
    /// Set when owned but currently researching/copying — a softer signal
    /// than "missing": the blueprint exists but isn't usable yet.
    pub warning_status: Option<String>,
}

/// One row of the global missing-blueprint report: a required type name
/// with no owned blueprint anywhere, and which operations need it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MissingBlueprintReport {
    pub type_name: String,
    pub required_by_operations: Vec<String>,
}

/// The Operations Workspace's Blueprints section, and one row of Mission
/// Control's Blueprint Readiness panel, scoped to one operation.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintReadiness {
    pub operation_id: i64,
    pub operation_goal: String,
    pub required: Vec<BlueprintRequirement>,
    pub owned_count: i64,
    pub missing_count: i64,
    pub warning_count: i64,
}

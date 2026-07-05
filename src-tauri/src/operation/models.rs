//! Operation domain DTOs. Mirror `src/lib/backend.ts` — change both
//! together. An Operation represents industrial intent; nothing here
//! assumes it maps to exactly one ship or structure.

use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationSummary {
    pub operation_id: i64,
    pub goal: String,
    pub target_type_name: Option<String>,
    pub priority: i64,
    pub status: String,
    pub progress: f64,
    pub deadline: Option<String>,
    /// Derived from `operation_dependencies`, never a hand-set literal:
    /// true if the operation's own status is "blocked", or if anything it
    /// depends on isn't complete yet.
    pub is_blocked: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationTimelineEntry {
    pub id: i64,
    pub label: String,
    pub status: String,
    pub sort_order: i64,
    pub target_date: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationDependency {
    pub depends_on_operation_id: i64,
    pub depends_on_goal: String,
    pub depends_on_status: String,
    pub reason: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationDetail {
    pub operation_id: i64,
    pub goal: String,
    pub target_type_name: Option<String>,
    pub priority: i64,
    pub status: String,
    pub progress: f64,
    pub notes: String,
    pub deadline: Option<String>,
    pub is_blocked: bool,
    pub dependencies: Vec<OperationDependency>,
    /// Owned by the Operation Engine, not rendered by the Workspace UI yet
    /// — Timeline is a reserved placeholder section this sprint.
    pub timeline: Vec<OperationTimelineEntry>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationHealth {
    pub total_operations: i64,
    pub active_operations: i64,
    pub blocked_operations: i64,
    /// (total - blocked) / total, 0 when there are no operations.
    pub healthy_fraction: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OperationsDashboard {
    pub health: OperationHealth,
    pub current_operations: Vec<OperationSummary>,
    pub priority_queue: Vec<OperationSummary>,
    pub blocked: Vec<OperationSummary>,
    pub upcoming_completions: Vec<OperationSummary>,
}

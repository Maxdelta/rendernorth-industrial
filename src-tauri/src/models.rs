//! IPC DTOs. These are the contract with `src/lib/backend.ts` — change both together.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbHealth {
    pub ok: bool,
    pub schema_version: i64,
    pub db_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierCoverage {
    pub label: String,
    pub coverage: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingMaterial {
    pub name: String,
    pub quantity: i64,
    pub category: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub title: String,
    pub reason: String,
    pub rule_id: String,
    /// Machine-readable inputs the rule fired on (Determinism Doctrine).
    pub inputs: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedTarget {
    pub project_id: i64,
    pub name: String,
    pub class_name: String,
    pub overall_progress: f64,
    pub tiers: Vec<TierCoverage>, // labels come from data — never hardcoded per hull
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildTargetSummary {
    pub project_id: i64,
    pub name: String,
    pub class_name: String,
    pub status: String,
    pub overall_progress: f64,
    pub is_selected: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryHealth {
    pub health: f64, // 0..1
    pub status: String, // derived deterministically: Blocked / Degraded / Operational
    pub idle_slots: i64,
    pub blocked_jobs: i64,
    pub missing_inputs: i64, // count of missing-input lines for the selected target
    pub isk_locked_in_jobs: f64,
    pub projected_finish_days: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionControl {
    pub source: String,
    pub as_of: String,
    pub running_jobs: i64,
    pub idle_characters: i64,
    pub idle_bpos: i64,
    pub wallet_isk: f64,
    pub factory_health: FactoryHealth,
    pub selected_target: SelectedTarget,
    pub missing_materials: Vec<MissingMaterial>,
    pub recommendation: Recommendation,
}

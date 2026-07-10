//! `ProductionEngine` — what Mission Control, the Production page, and the
//! Operations Workspace ask about required inputs. Mirrors
//! `inventory::engine`, `operation::engine`, `reservation::engine`, and
//! `blueprint::engine`.
//!
//! Reads (lines, detail, categories, summary, shortages, bottlenecks,
//! per-operation breakdown) are real this sprint, backed by migration
//! 0007. Mutations (declare a requirement, adjust a required quantity) are
//! declared here because the Production Requirement Engine will own them,
//! but each returns an explicit "not yet" error — no manufacturing job
//! logic, scheduling, or shopping runs through this engine.

use super::models::{
    CriticalBottleneck, OperationRequirementBreakdown, ProductionPlan, RequirementCategory,
    RequirementDetail, RequirementLine, RequirementShortage, RequirementSummary,
};
use super::provider::{MockProductionProvider, ProductionProvider};
use rusqlite::Connection;

pub struct ProductionEngine<P: ProductionProvider> {
    provider: P,
}

impl<P: ProductionProvider> ProductionEngine<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    // ---- reads: live this sprint ----

    pub fn lines(&self, operation_id: Option<i64>, category_key: Option<&str>) -> Result<Vec<RequirementLine>, String> {
        self.provider.lines(operation_id, category_key)
    }

    pub fn get_detail(&self, requirement_id: i64) -> Result<RequirementDetail, String> {
        self.provider.get_detail(requirement_id)
    }

    pub fn categories(&self) -> Result<Vec<RequirementCategory>, String> {
        self.provider.categories()
    }

    pub fn summary(&self) -> Result<RequirementSummary, String> {
        self.provider.summary()
    }

    /// Detect only. Nothing here acquires, manufactures, or schedules
    /// anything — surfacing what's short is the entire job.
    pub fn shortages(&self) -> Result<Vec<RequirementShortage>, String> {
        self.provider.shortages()
    }

    pub fn critical_bottlenecks(&self) -> Result<Vec<CriticalBottleneck>, String> {
        self.provider.critical_bottlenecks()
    }

    pub fn breakdown_for_operation(&self, operation_id: i64) -> Result<OperationRequirementBreakdown, String> {
        self.provider.breakdown_for_operation(operation_id)
    }

    /// The real, blueprint-derived plan for one operation's build target
    /// (Sprint 008). Deterministic; always recomputed from current
    /// imported data, owned/assumed blueprint state, and inventory.
    pub fn calculate_plan(&self, operation_id: i64) -> Result<ProductionPlan, String> {
        self.provider.calculate_plan(operation_id)
    }

    /// Same underlying data as `breakdown_for_operation`, exposed under a
    /// build-target-flavored name for the Build Targets flow. Operations
    /// 1–5 intentionally share id space with `build_projects.project_id`
    /// (see migration 0004's coexistence note), so this forwards rather
    /// than duplicating the query.
    pub fn requirements_for_build_target(
        &self,
        project_id: i64,
    ) -> Result<OperationRequirementBreakdown, String> {
        self.breakdown_for_operation(project_id)
    }

    // ---- mutations: architecture-only this sprint ----
    // Declared now so the eventual implementation is a body swap, not an
    // API redesign. Every one of these is exactly the "manufacturing job
    // logic" and "shopping logic" this sprint is told not to build, so
    // all return the same refusal. #[allow(dead_code)] marks them as a
    // deliberate reminder, not an oversight — no command calls them yet.

    #[allow(dead_code, unused_variables)]
    pub fn declare_requirement(
        &self,
        operation_id: i64,
        category_key: &str,
        type_name: &str,
        required_quantity: i64,
    ) -> Result<RequirementLine, String> {
        Err("Production Requirement Engine mutations are architecture-only this sprint".into())
    }

    #[allow(dead_code, unused_variables)]
    pub fn adjust_required_quantity(
        &self,
        requirement_id: i64,
        new_quantity: i64,
    ) -> Result<(), String> {
        Err("Production Requirement Engine mutations are architecture-only this sprint".into())
    }
}

/// Convenience constructor used by every command: today's engine is the
/// local demo-seeded provider over the shared SQLite connection.
pub fn engine_for(conn: &Connection) -> ProductionEngine<MockProductionProvider<'_>> {
    ProductionEngine::new(MockProductionProvider::new(conn))
}

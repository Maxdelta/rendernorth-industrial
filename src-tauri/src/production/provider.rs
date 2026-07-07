//! `ProductionProvider` — the seam between the Production Requirement
//! Engine and wherever requirement data actually lives. Mirrors
//! `inventory::provider`, `operation::provider`, `reservation::provider`,
//! and `blueprint::provider`. A future `EsiProductionProvider` (fulfilling
//! this same trait once real blueprint material trees exist from the SDE)
//! is the intended extension point — not implemented this sprint.

use super::models::{
    CriticalBottleneck, OperationRequirementBreakdown, RequirementCategory, RequirementDetail,
    RequirementLine, RequirementShortage, RequirementSummary,
};
use super::repository::ProductionRepository;
use rusqlite::Connection;

pub trait ProductionProvider {
    fn lines(&self, operation_id: Option<i64>, category_key: Option<&str>) -> Result<Vec<RequirementLine>, String>;
    fn get_detail(&self, requirement_id: i64) -> Result<RequirementDetail, String>;
    fn categories(&self) -> Result<Vec<RequirementCategory>, String>;
    fn summary(&self) -> Result<RequirementSummary, String>;
    fn shortages(&self) -> Result<Vec<RequirementShortage>, String>;
    fn critical_bottlenecks(&self) -> Result<Vec<CriticalBottleneck>, String>;
    fn breakdown_for_operation(&self, operation_id: i64) -> Result<OperationRequirementBreakdown, String>;
}

/// The live provider, backed by the demo-seeded rows from migration 0007.
/// "Mock" describes the data's origin — same convention as Inventory,
/// Operation, Reservation, and Blueprint.
pub struct MockProductionProvider<'a> {
    repo: ProductionRepository<'a>,
}

impl<'a> MockProductionProvider<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: ProductionRepository::new(conn),
        }
    }
}

impl<'a> ProductionProvider for MockProductionProvider<'a> {
    fn lines(&self, operation_id: Option<i64>, category_key: Option<&str>) -> Result<Vec<RequirementLine>, String> {
        self.repo.lines(operation_id, category_key)
    }

    fn get_detail(&self, requirement_id: i64) -> Result<RequirementDetail, String> {
        self.repo.get_detail(requirement_id)
    }

    fn categories(&self) -> Result<Vec<RequirementCategory>, String> {
        self.repo.categories()
    }

    fn summary(&self) -> Result<RequirementSummary, String> {
        self.repo.summary()
    }

    fn shortages(&self) -> Result<Vec<RequirementShortage>, String> {
        self.repo.shortages()
    }

    fn critical_bottlenecks(&self) -> Result<Vec<CriticalBottleneck>, String> {
        self.repo.critical_bottlenecks()
    }

    fn breakdown_for_operation(&self, operation_id: i64) -> Result<OperationRequirementBreakdown, String> {
        self.repo.breakdown_for_operation(operation_id)
    }
}

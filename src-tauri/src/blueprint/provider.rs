//! `BlueprintProvider` — the seam between the Blueprint Engine and
//! wherever blueprint data actually lives. Mirrors
//! `inventory::provider::InventoryProvider`,
//! `operation::provider::OperationProvider`, and
//! `reservation::provider::ReservationProvider`. A future
//! `EsiBlueprintProvider` (fulfilling this same trait against ESI-synced
//! blueprint rows) is the intended extension point — see
//! ESI_INTEGRATION.md — but is not implemented this sprint.

use super::models::{
    BlueprintDetail, BlueprintReadiness, BlueprintRecord, BlueprintSummary, MissingBlueprintReport,
    OwnedBlueprintCandidate,
};
use super::repository::BlueprintRepository;
use rusqlite::Connection;

pub trait BlueprintProvider {
    fn list(&self) -> Result<Vec<BlueprintRecord>, String>;
    fn get_detail(&self, blueprint_id: i64) -> Result<BlueprintDetail, String>;
    fn summary(&self) -> Result<BlueprintSummary, String>;
    fn missing_report(&self) -> Result<Vec<MissingBlueprintReport>, String>;
    fn readiness_for_operation(&self, operation_id: i64) -> Result<BlueprintReadiness, String>;
    fn readiness_all(&self) -> Result<Vec<BlueprintReadiness>, String>;
    fn owned_candidates(&self, product_type_id:i64, requested_quantity:i64) -> Result<Vec<OwnedBlueprintCandidate>,String>;
}

/// The live provider, backed by the demo-seeded rows from migration 0006.
/// "Mock" describes the data's origin — same convention as Inventory,
/// Operation, and Reservation.
pub struct MockBlueprintProvider<'a> {
    repo: BlueprintRepository<'a>,
}

impl<'a> MockBlueprintProvider<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: BlueprintRepository::new(conn),
        }
    }
}

impl<'a> BlueprintProvider for MockBlueprintProvider<'a> {
    fn list(&self) -> Result<Vec<BlueprintRecord>, String> {
        self.repo.list()
    }

    fn get_detail(&self, blueprint_id: i64) -> Result<BlueprintDetail, String> {
        self.repo.get_detail(blueprint_id)
    }

    fn summary(&self) -> Result<BlueprintSummary, String> {
        self.repo.summary()
    }

    fn missing_report(&self) -> Result<Vec<MissingBlueprintReport>, String> {
        self.repo.missing_report()
    }

    fn readiness_for_operation(&self, operation_id: i64) -> Result<BlueprintReadiness, String> {
        self.repo.readiness_for_operation(operation_id)
    }

    fn readiness_all(&self) -> Result<Vec<BlueprintReadiness>, String> {
        self.repo.readiness_all()
    }

    fn owned_candidates(&self, product_type_id:i64, requested_quantity:i64) -> Result<Vec<OwnedBlueprintCandidate>,String> {
        self.repo.owned_candidates(product_type_id,requested_quantity)
    }
}

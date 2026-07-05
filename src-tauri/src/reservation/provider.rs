//! `ReservationProvider` — the seam between the Reservation Engine and
//! wherever reservation data actually lives. Mirrors
//! `inventory::provider::InventoryProvider` and
//! `operation::provider::OperationProvider`. Like Operations, reservations
//! are a RenderNorth-only concept with no ESI equivalent — the seam exists
//! for consistency and future flexibility, not because a sync backend is
//! coming.

use super::models::{
    InventoryCommitment, OperationReservations, ReservationConflict, ReservationDetail,
    ReservationRecord, ReservationSummary,
};
use super::repository::ReservationRepository;
use rusqlite::Connection;

pub trait ReservationProvider {
    fn list_active(&self) -> Result<Vec<ReservationRecord>, String>;
    fn get_detail(&self, reservation_id: i64) -> Result<ReservationDetail, String>;
    fn summary(&self) -> Result<ReservationSummary, String>;
    fn conflicts(&self) -> Result<Vec<ReservationConflict>, String>;
    fn inventory_commitment(&self) -> Result<InventoryCommitment, String>;
    fn operation_reservations(&self, operation_id: i64) -> Result<OperationReservations, String>;
}

/// The live provider, backed by the demo-seeded rows from migration 0005
/// (extending 0003's original reservation seed). "Mock" describes the
/// data's origin — same convention as the Inventory and Operation modules.
pub struct MockReservationProvider<'a> {
    repo: ReservationRepository<'a>,
}

impl<'a> MockReservationProvider<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: ReservationRepository::new(conn),
        }
    }
}

impl<'a> ReservationProvider for MockReservationProvider<'a> {
    fn list_active(&self) -> Result<Vec<ReservationRecord>, String> {
        self.repo.list_active()
    }

    fn get_detail(&self, reservation_id: i64) -> Result<ReservationDetail, String> {
        self.repo.get_detail(reservation_id)
    }

    fn summary(&self) -> Result<ReservationSummary, String> {
        self.repo.summary()
    }

    fn conflicts(&self) -> Result<Vec<ReservationConflict>, String> {
        self.repo.conflicts()
    }

    fn inventory_commitment(&self) -> Result<InventoryCommitment, String> {
        self.repo.inventory_commitment()
    }

    fn operation_reservations(&self, operation_id: i64) -> Result<OperationReservations, String> {
        self.repo.operation_reservations(operation_id)
    }
}

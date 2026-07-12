//! `OperationProvider` — the seam between the Operation Engine and wherever
//! operation data actually lives. Unlike Inventory (which will eventually
//! sync from CCP ESI), Operations are a RenderNorth-only concept — there is
//! no external authority for "what operations exist," so there is no
//! ESI-equivalent provider to stub here. The seam still exists for
//! consistency with the Inventory architecture and to leave room for a
//! future sync backend (e.g. multi-device) without touching the engine or
//! any command.

use super::models::{NewOperationInput, OperationDetail, OperationHealth, OperationSummary};
use super::repository::OperationRepository;
use rusqlite::Connection;

pub trait OperationProvider {
    fn list_summaries(&self) -> Result<Vec<OperationSummary>, String>;
    fn priority_queue(&self) -> Result<Vec<OperationSummary>, String>;
    fn blocked(&self) -> Result<Vec<OperationSummary>, String>;
    fn upcoming_completions(&self, limit: i64) -> Result<Vec<OperationSummary>, String>;
    fn health(&self) -> Result<OperationHealth, String>;
    fn get_detail(&self, operation_id: i64) -> Result<OperationDetail, String>;
    fn create(&self, input: &NewOperationInput) -> Result<i64, String>;
    fn delete(&self, operation_id: i64) -> Result<(), String>;
}

/// The live provider, backed by the demo-seeded rows from migration 0004.
/// "Mock" describes the data's origin (hand-seeded, not synced) — same
/// convention as `inventory::provider::MockInventoryProvider`.
pub struct MockOperationProvider<'a> {
    repo: OperationRepository<'a>,
}

impl<'a> MockOperationProvider<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self {
            repo: OperationRepository::new(conn),
        }
    }
}

impl<'a> OperationProvider for MockOperationProvider<'a> {
    fn list_summaries(&self) -> Result<Vec<OperationSummary>, String> {
        self.repo.list_summaries()
    }

    fn priority_queue(&self) -> Result<Vec<OperationSummary>, String> {
        self.repo.priority_queue()
    }

    fn blocked(&self) -> Result<Vec<OperationSummary>, String> {
        self.repo.blocked()
    }

    fn upcoming_completions(&self, limit: i64) -> Result<Vec<OperationSummary>, String> {
        self.repo.upcoming_completions(limit)
    }

    fn health(&self) -> Result<OperationHealth, String> {
        self.repo.health()
    }

    fn get_detail(&self, operation_id: i64) -> Result<OperationDetail, String> {
        self.repo.get_detail(operation_id)
    }

    fn create(&self, input: &NewOperationInput) -> Result<i64, String> {
        self.repo.create(input)
    }

    fn delete(&self, operation_id: i64) -> Result<(), String> {
        self.repo.delete(operation_id)
    }
}

//! `ProductionRepository` — the only place that writes SQL against the
//! production requirement tables. Mirrors `inventory::repository`,
//! `operation::repository`, `reservation::repository`, and
//! `blueprint::repository`.
//!
//! Coverage, shortage, and bottleneck figures are computed in Rust over
//! the rows this repository fetches, rather than in increasingly complex
//! nested SQL — the dataset per operation is small and this keeps every
//! aggregate easy to verify by hand against the underlying lines.

use super::models::{
    CategoryCoverage, CriticalBottleneck, OperationRequirementBreakdown, RequirementCategory,
    RequirementLine, RequirementShortage, RequirementSource, RequirementSummary,
};
use rusqlite::Connection;
use std::collections::HashMap;

pub struct ProductionRepository<'a> {
    conn: &'a Connection,
}

impl<'a> ProductionRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    const LINE_SELECT: &'static str = "
        SELECT r.id, r.operation_id, o.goal, r.category_key, c.label,
               r.type_name, r.required_quantity,
               COALESCE(owned.total, 0) AS owned_quantity,
               COALESCE(resv.reserved, 0) AS reserved_for_operation
        FROM production_requirements r
        JOIN operations o ON o.operation_id = r.operation_id
        JOIN inventory_categories c ON c.key = r.category_key
        LEFT JOIN (
            SELECT type_name, SUM(quantity) AS total FROM inventory_items GROUP BY type_name
        ) owned ON owned.type_name = r.type_name
        LEFT JOIN (
            SELECT res.operation_id, i.type_name, SUM(res.quantity) AS reserved
            FROM inventory_reservations res
            JOIN inventory_items i ON i.item_id = res.item_id
            WHERE res.released_at IS NULL
            GROUP BY res.operation_id, i.type_name
        ) resv ON resv.operation_id = r.operation_id AND resv.type_name = r.type_name
    ";

    fn map_line(row: &rusqlite::Row) -> rusqlite::Result<RequirementLine> {
        let required_quantity: i64 = row.get(6)?;
        let owned_quantity: i64 = row.get(7)?;
        let shortage = (required_quantity - owned_quantity).max(0);
        let coverage_fraction = if required_quantity > 0 {
            (owned_quantity as f64 / required_quantity as f64).min(1.0)
        } else {
            1.0
        };
        Ok(RequirementLine {
            requirement_id: row.get(0)?,
            operation_id: row.get(1)?,
            operation_goal: row.get(2)?,
            category_key: row.get(3)?,
            category_label: row.get(4)?,
            type_name: row.get(5)?,
            required_quantity,
            owned_quantity,
            reserved_for_operation: row.get(8)?,
            shortage,
            coverage_fraction,
            is_satisfied: owned_quantity >= required_quantity,
        })
    }

    /// `operation_id = None` returns every operation's lines; `category_key
    /// = None` returns every category. Both filters compose (used by the
    /// Production page's filterable requirement tree).
    pub fn lines(
        &self,
        operation_id: Option<i64>,
        category_key: Option<&str>,
    ) -> Result<Vec<RequirementLine>, String> {
        let sql = format!(
            "{} WHERE (?1 IS NULL OR r.operation_id = ?1) AND (?2 IS NULL OR r.category_key = ?2)
             ORDER BY r.operation_id, c.sort_order, r.id",
            Self::LINE_SELECT
        );
        let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map((operation_id, category_key), Self::map_line)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_detail(&self, requirement_id: i64) -> Result<super::models::RequirementDetail, String> {
        let sql = format!("{} WHERE r.id = ?1", Self::LINE_SELECT);
        let line = self
            .conn
            .query_row(&sql, [requirement_id], Self::map_line)
            .map_err(|e| format!("requirement {requirement_id} not found: {e}"))?;

        let mut sources = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT source_kind, contributed_quantity, note
                     FROM production_requirement_sources WHERE requirement_id = ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([requirement_id], |row| {
                    Ok(RequirementSource {
                        source_kind: row.get(0)?,
                        contributed_quantity: row.get(1)?,
                        note: row.get(2)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                sources.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(super::models::RequirementDetail { line, sources })
    }

    /// Every category with a declared scope somewhere (structural, from
    /// `production_requirement_groups` — never the source of coverage
    /// numbers), ordered the same way Inventory orders its category rail.
    pub fn categories(&self) -> Result<Vec<RequirementCategory>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT c.key, c.label, c.sort_order
                 FROM production_requirement_groups g
                 JOIN inventory_categories c ON c.key = g.category_key
                 ORDER BY c.sort_order",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(RequirementCategory {
                    category_key: row.get(0)?,
                    category_label: row.get(1)?,
                    sort_order: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn summary(&self) -> Result<RequirementSummary, String> {
        let lines = self.lines(None, None)?;
        let total_requirements = lines.len() as i64;
        let satisfied = lines.iter().filter(|l| l.is_satisfied).count() as i64;
        let missing = total_requirements - satisfied;

        let total_required: i64 = lines.iter().map(|l| l.required_quantity).sum();
        let total_covered: i64 = lines
            .iter()
            .map(|l| l.owned_quantity.min(l.required_quantity))
            .sum();
        let coverage_percent = if total_required > 0 {
            100.0 * total_covered as f64 / total_required as f64
        } else {
            0.0
        };

        let critical_bottleneck_count = self.critical_bottlenecks_from(&lines).len() as i64;

        Ok(RequirementSummary {
            total_requirements,
            satisfied,
            missing,
            coverage_percent,
            critical_bottleneck_count,
        })
    }

    pub fn shortages(&self) -> Result<Vec<RequirementShortage>, String> {
        let lines = self.lines(None, None)?;
        Ok(lines
            .into_iter()
            .filter(|l| !l.is_satisfied)
            .map(|l| RequirementShortage {
                type_name: l.type_name,
                operation_goal: l.operation_goal,
                required_quantity: l.required_quantity,
                owned_quantity: l.owned_quantity,
                shortage: l.shortage,
            })
            .collect())
    }

    /// A material required by 2+ operations, unmet in at least one of
    /// them. Always computed fresh from current lines — never a stored
    /// flag.
    pub fn critical_bottlenecks(&self) -> Result<Vec<CriticalBottleneck>, String> {
        let lines = self.lines(None, None)?;
        Ok(self.critical_bottlenecks_from(&lines))
    }

    fn critical_bottlenecks_from(&self, lines: &[RequirementLine]) -> Vec<CriticalBottleneck> {
        let mut by_type: HashMap<&str, Vec<&RequirementLine>> = HashMap::new();
        for l in lines {
            by_type.entry(&l.type_name).or_default().push(l);
        }

        let mut out = Vec::new();
        for (type_name, group) in by_type {
            let distinct_ops: std::collections::HashSet<i64> =
                group.iter().map(|l| l.operation_id).collect();
            let any_unmet = group.iter().any(|l| !l.is_satisfied);
            if distinct_ops.len() > 1 && any_unmet {
                out.push(CriticalBottleneck {
                    type_name: type_name.to_string(),
                    operations_requiring: group.iter().map(|l| l.operation_goal.clone()).collect(),
                    owned_quantity: group[0].owned_quantity,
                });
            }
        }
        out.sort_by(|a, b| a.type_name.cmp(&b.type_name));
        out
    }

    /// Scoped to one operation: category rollups + every line + totals.
    /// Backs both the Operations Workspace's Production Requirements
    /// section and the Production page's per-target breakdown (see
    /// `production::engine::requirements_for_build_target`, which forwards
    /// here rather than duplicating this query).
    pub fn breakdown_for_operation(
        &self,
        operation_id: i64,
    ) -> Result<OperationRequirementBreakdown, String> {
        let operation_goal: String = self
            .conn
            .query_row(
                "SELECT goal FROM operations WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("operation {operation_id} not found: {e}"))?;

        let lines = self.lines(Some(operation_id), None)?;

        let mut by_category: HashMap<&str, (String, i64, i64, i64)> = HashMap::new();
        for l in &lines {
            let entry = by_category
                .entry(&l.category_key)
                .or_insert((l.category_label.clone(), 0, 0, 0));
            entry.1 += l.required_quantity;
            entry.2 += l.owned_quantity.min(l.required_quantity);
            if !l.is_satisfied {
                entry.3 += 1;
            }
        }

        let mut categories: Vec<CategoryCoverage> = by_category
            .into_iter()
            .map(|(key, (label, required_total, covered_total, missing_count))| {
                let coverage_fraction = if required_total > 0 {
                    covered_total as f64 / required_total as f64
                } else {
                    1.0
                };
                CategoryCoverage {
                    category_key: key.to_string(),
                    category_label: label,
                    required_total,
                    owned_total: covered_total,
                    coverage_fraction,
                    missing_count,
                }
            })
            .collect();
        categories.sort_by(|a, b| a.category_key.cmp(&b.category_key));

        let total_requirements = lines.len() as i64;
        let missing_count = lines.iter().filter(|l| !l.is_satisfied).count() as i64;
        let total_required: i64 = lines.iter().map(|l| l.required_quantity).sum();
        let total_covered: i64 = lines
            .iter()
            .map(|l| l.owned_quantity.min(l.required_quantity))
            .sum();
        let coverage_percent = if total_required > 0 {
            100.0 * total_covered as f64 / total_required as f64
        } else {
            0.0
        };

        Ok(OperationRequirementBreakdown {
            operation_id,
            operation_goal,
            categories,
            lines,
            total_requirements,
            missing_count,
            coverage_percent,
        })
    }
}

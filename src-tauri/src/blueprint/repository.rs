//! `BlueprintRepository` — the only place that writes SQL against the
//! blueprint tables. Mirrors `inventory::repository::InventoryRepository`,
//! `operation::repository::OperationRepository`, and
//! `reservation::repository::ReservationRepository`.

use super::models::{
    BlueprintDetail, BlueprintReadiness, BlueprintRecord, BlueprintRequirement, BlueprintSummary,
    MissingBlueprintReport, OwnedBlueprintCandidate,
};
use rusqlite::Connection;

pub struct BlueprintRepository<'a> {
    conn: &'a Connection,
}

#[cfg(test)]
mod error_tests {
    use super::*;
    #[test]
    fn backend_query_failure_is_returned_as_error_not_empty_blueprints() {
        let conn=Connection::open_in_memory().unwrap();
        let result=BlueprintRepository::new(&conn).list();
        assert!(result.is_err(),"missing schema must surface an error, never an empty successful list");
    }
}

impl<'a> BlueprintRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    const RECORD_SELECT: &'static str = "
        SELECT b.blueprint_id, b.type_name, b.is_copy, b.me_level, b.te_level,
               b.runs_remaining, COALESCE(ch.name, 'Unassigned') AS owner_name,
               COALESCE(loc.name, 'Unknown location') AS location_name, b.status,
               op.goal AS linked_operation
        FROM blueprints b
        LEFT JOIN characters ch ON ch.character_id = b.character_id
        LEFT JOIN inventory_locations loc ON loc.location_id = b.location_id
        LEFT JOIN (
            SELECT r.type_name, MAX(o.goal) AS goal
            FROM operation_blueprint_requirements r
            JOIN operations o ON o.operation_id = r.operation_id
            GROUP BY r.type_name
        ) op ON op.type_name = b.type_name
        WHERE b.is_demo = 0
    ";

    fn map_record(row: &rusqlite::Row) -> rusqlite::Result<BlueprintRecord> {
        let is_copy: i64 = row.get(2)?;
        Ok(BlueprintRecord {
            blueprint_id: row.get(0)?,
            type_id: None,
            type_name: row.get(1)?,
            is_copy: is_copy != 0,
            me_level: row.get(3)?,
            te_level: row.get(4)?,
            runs_remaining: row.get(5)?,
            owner_name: row.get(6)?,
            location_name: row.get(7)?,
            status: row.get(8)?,
            linked_operation: row.get(9)?,
            location_id: None,
            location_flag: None,
            quantity: 1,
            source: "Manual Ownership".into(),
            last_synced: None,
            is_owned: true,
            resolved_location: None,
        })
    }

    pub fn list(&self) -> Result<Vec<BlueprintRecord>, String> {
        let mut out = {
            let sql = format!("{} ORDER BY b.blueprint_id", Self::RECORD_SELECT);
            let mut stmt = self.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let collected=stmt.query_map([], Self::map_record).map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            collected
        };
        let mut stmt = self.conn.prepare(
            "SELECT cb.item_id, cb.type_id, COALESCE(t.name,'Unknown Blueprint ' || cb.type_id),
                    cb.runs != -1, cb.material_efficiency, cb.time_efficiency,
                    CASE WHEN cb.runs=-1 THEN NULL ELSE cb.runs END,
                    c.name, cb.location_id, cb.location_flag, cb.quantity, cb.source, cb.synced_at,
                    (SELECT MAX(o.goal) FROM operation_build_targets bt JOIN operations o ON o.operation_id=bt.operation_id WHERE bt.type_id IN (SELECT product_type_id FROM blueprint_products WHERE blueprint_type_id=cb.type_id)), cb.character_id
             FROM character_blueprints cb JOIN characters c ON c.character_id=cb.character_id
             LEFT JOIN eve_types t ON t.type_id=cb.type_id
             ORDER BY c.name,t.name,cb.item_id"
        ).map_err(|e|format!("synchronized blueprint query failed: {e}"))?;
        let esi=stmt.query_map([],|r|{let character_id:i64=r.get(14)?;let location_id:i64=r.get(8)?;Ok(BlueprintRecord{
            blueprint_id:r.get(0)?,type_id:Some(r.get(1)?),type_name:r.get(2)?,is_copy:r.get::<_,i64>(3)?!=0,
            me_level:r.get(4)?,te_level:r.get(5)?,runs_remaining:r.get(6)?,owner_name:r.get(7)?,
            location_name:location_id.to_string(),status:"available".into(),linked_operation:r.get(13)?,
            location_id:Some(location_id),location_flag:Some(r.get(9)?),quantity:r.get(10)?,source:r.get(11)?,last_synced:Some(r.get(12)?),is_owned:true,
            resolved_location:Some(crate::location::resolve(self.conn,character_id,location_id).map_err(|e|rusqlite::Error::ToSqlConversionFailure(e.into()))?),
        })}).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        out.extend(esi);
        let mut refs=self.conn.prepare("SELECT DISTINCT bp.blueprint_type_id,COALESCE(t.name,'Unknown Blueprint ' || bp.blueprint_type_id) FROM blueprint_products bp LEFT JOIN eve_types t ON t.type_id=bp.blueprint_type_id ORDER BY 2").map_err(|e|e.to_string())?;
        let reference=refs.query_map([],|r|Ok(BlueprintRecord{blueprint_id:r.get(0)?,type_id:Some(r.get(0)?),type_name:r.get(1)?,is_copy:false,me_level:0,te_level:0,runs_remaining:None,owner_name:"CCP Static Data".into(),location_name:"—".into(),status:"reference".into(),linked_operation:None,location_id:None,location_flag:None,quantity:0,source:"CCP Reference Data".into(),last_synced:None,is_owned:false,resolved_location:None})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
        out.extend(reference); Ok(out)
    }

    pub fn owned_candidates(
        &self,
        product_type_id: i64,
        requested_quantity: i64,
    ) -> Result<Vec<OwnedBlueprintCandidate>, String> {
        if requested_quantity < 1 {
            return Err("requested output quantity must be at least 1".into());
        }
        let product_name: String = self.conn.query_row(
            "SELECT name FROM eve_types WHERE type_id=?1",
            [product_type_id],
            |row| row.get(0),
        ).map_err(|e| format!("product type {product_type_id} not found: {e}"))?;

        let mut out = Vec::new();
        let mut manual = self.conn.prepare(
            "SELECT b.blueprint_id,b.is_copy,b.me_level,b.te_level,b.runs_remaining,
                    COALESCE(c.name,'Unassigned'),
                    COALESCE(bp.blueprint_type_id,(SELECT blueprint_type_id FROM blueprint_products WHERE product_type_id=?1 ORDER BY blueprint_type_id LIMIT 1)),
                    COALESCE(bp.quantity,(SELECT quantity FROM blueprint_products WHERE product_type_id=?1 ORDER BY blueprint_type_id LIMIT 1))
             FROM blueprints b
             LEFT JOIN characters c ON c.character_id=b.character_id
             LEFT JOIN eve_types bt ON bt.name=b.type_name
             LEFT JOIN blueprint_products bp
               ON bp.product_type_id=?1 AND bp.blueprint_type_id=bt.type_id
             WHERE b.is_demo=0
               AND (b.type_name=?2 OR bp.blueprint_type_id IS NOT NULL)
             ORDER BY b.is_copy ASC,b.me_level DESC,b.te_level DESC,
                      COALESCE(c.name,'Unassigned'),b.blueprint_id"
        ).map_err(|e|e.to_string())?;
        let manual_rows = manual.query_map((product_type_id, &product_name), |r| {
            let is_copy = r.get::<_,i64>(1)? != 0;
            let available: Option<i64> = r.get(4)?;
            let output_per_run = r.get::<_,Option<i64>>(7)?.unwrap_or(1).max(1);
            let required_runs = (requested_quantity + output_per_run - 1) / output_per_run;
            Ok(OwnedBlueprintCandidate {
                source:"manual".into(),manual_blueprint_id:Some(r.get(0)?),
                character_id:None,item_id:None,blueprint_type_id:r.get(6)?,
                product_type_id,is_copy,me:r.get(2)?,te:r.get(3)?,
                runs_remaining:available,required_runs,owner_name:r.get(5)?,
                source_label:"Manual Ownership".into(),resolved_location:None,
            })
        }).map_err(|e|e.to_string())?;
        for row in manual_rows {
            let candidate=row.map_err(|e|e.to_string())?;
            if !candidate.is_copy || candidate.runs_remaining.unwrap_or(0) >= candidate.required_runs {
                out.push(candidate);
            }
        }

        let mut esi = self.conn.prepare(
            "SELECT cb.character_id,cb.item_id,cb.type_id,cb.runs != -1,
                    cb.material_efficiency,cb.time_efficiency,
                    CASE WHEN cb.runs=-1 THEN NULL ELSE cb.runs END,
                    c.name,cb.location_id,bp.quantity,cb.source
             FROM character_blueprints cb
             JOIN characters c ON c.character_id=cb.character_id
             JOIN blueprint_products bp ON bp.blueprint_type_id=cb.type_id
             WHERE c.enabled=1 AND c.is_demo=0 AND bp.product_type_id=?1
             ORDER BY (cb.runs != -1) ASC,cb.material_efficiency DESC,
                      cb.time_efficiency DESC,c.name,cb.character_id,cb.item_id"
        ).map_err(|e|e.to_string())?;
        let esi_rows=esi.query_map([product_type_id],|r|{
            let character_id:i64=r.get(0)?;
            let location_id:i64=r.get(8)?;
            let is_copy=r.get::<_,i64>(3)?!=0;
            let output_per_run=r.get::<_,i64>(9)?.max(1);
            let required_runs=(requested_quantity+output_per_run-1)/output_per_run;
            Ok(OwnedBlueprintCandidate{
                source:"esi_character".into(),manual_blueprint_id:None,
                character_id:Some(character_id),item_id:Some(r.get(1)?),
                blueprint_type_id:Some(r.get(2)?),product_type_id,is_copy,
                me:r.get(4)?,te:r.get(5)?,runs_remaining:r.get(6)?,
                required_runs,owner_name:r.get(7)?,source_label:r.get(10)?,
                resolved_location:Some(crate::location::resolve(self.conn,character_id,location_id)
                    .map_err(|e|rusqlite::Error::ToSqlConversionFailure(e.into()))?),
            })
        }).map_err(|e|e.to_string())?;
        for row in esi_rows {
            let candidate=row.map_err(|e|e.to_string())?;
            if !candidate.is_copy || candidate.runs_remaining.unwrap_or(0) >= candidate.required_runs {
                out.push(candidate);
            }
        }
        out.sort_by(|a,b|{
            a.is_copy.cmp(&b.is_copy)
                .then(b.me.cmp(&a.me))
                .then(b.te.cmp(&a.te))
                .then(a.owner_name.cmp(&b.owner_name))
                .then(a.source.cmp(&b.source))
                .then(a.character_id.cmp(&b.character_id))
                .then(a.item_id.cmp(&b.item_id))
                .then(a.manual_blueprint_id.cmp(&b.manual_blueprint_id))
        });
        Ok(out)
    }

    pub fn get_detail(&self, blueprint_id: i64) -> Result<BlueprintDetail, String> {
        let sql = format!("{} AND b.blueprint_id = ?1", Self::RECORD_SELECT);
        let record = self
            .conn
            .query_row(&sql, [blueprint_id], Self::map_record)
            .map_err(|e| format!("blueprint {blueprint_id} not found: {e}"))?;

        let mut required_by = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT DISTINCT o.goal
                     FROM operation_blueprint_requirements r
                     JOIN operations o ON o.operation_id = r.operation_id
                     WHERE r.type_name = ?1",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([&record.type_name], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            for row in rows {
                required_by.push(row.map_err(|e| e.to_string())?);
            }
        }

        Ok(BlueprintDetail { record, required_by })
    }

    pub fn summary(&self) -> Result<BlueprintSummary, String> {
        let (total_blueprints, bpo_count, bpc_count, research_complete, copies): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT
                    COUNT(*),
                    SUM(CASE WHEN is_copy = 0 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN is_copy = 1 THEN 1 ELSE 0 END),
                    SUM(CASE WHEN status != 'researching' THEN 1 ELSE 0 END),
                    COALESCE(SUM(CASE WHEN is_copy = 1 THEN runs_remaining ELSE 0 END), 0)
                 FROM (
                    SELECT is_copy,status,runs_remaining FROM blueprints WHERE is_demo=0
                    UNION ALL
                    SELECT runs != -1,'available',CASE WHEN runs=-1 THEN NULL ELSE runs END
                    FROM character_blueprints cb JOIN characters c ON c.character_id=cb.character_id WHERE c.enabled=1 AND c.is_demo=0
                 )",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                        row.get(4)?,
                    ))
                },
            )
            .map_err(|e| format!("blueprint summary query failed: {e}"))?;

        let missing_for_operations: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT r.type_name)
                 FROM operation_blueprint_requirements r
                 JOIN operations o ON o.operation_id = r.operation_id AND o.is_demo = 0
                 WHERE NOT EXISTS (SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name AND b.is_demo = 0)
                   AND NOT EXISTS (SELECT 1 FROM character_blueprints cb JOIN eve_types t ON t.type_id=cb.type_id JOIN characters c ON c.character_id=cb.character_id WHERE t.name=r.type_name AND c.enabled=1 AND c.is_demo=0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("missing-for-operations query failed: {e}"))?;

        Ok(BlueprintSummary {
            total_blueprints,
            bpo_count,
            bpc_count,
            research_complete,
            copies,
            missing_for_operations,
        })
    }

    /// Global report: every required blueprint type with no owned
    /// blueprint anywhere, and which operations need it. Detect only —
    /// nothing here acquires or researches a blueprint.
    pub fn missing_report(&self) -> Result<Vec<MissingBlueprintReport>, String> {
        let mut report = Vec::new();
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT r.type_name
                 FROM operation_blueprint_requirements r
                 JOIN operations o ON o.operation_id = r.operation_id AND o.is_demo = 0
                 WHERE NOT EXISTS (SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name AND b.is_demo = 0)
                   AND NOT EXISTS (SELECT 1 FROM character_blueprints cb JOIN eve_types t ON t.type_id=cb.type_id JOIN characters c ON c.character_id=cb.character_id WHERE t.name=r.type_name AND c.enabled=1 AND c.is_demo=0)",
            )
            .map_err(|e| e.to_string())?;
        let type_names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        for type_name in type_names {
            let mut op_stmt = self
                .conn
                .prepare(
                    "SELECT o.goal FROM operation_blueprint_requirements r
                     JOIN operations o ON o.operation_id = r.operation_id AND o.is_demo = 0
                     WHERE r.type_name = ?1",
                )
                .map_err(|e| e.to_string())?;
            let goals = op_stmt
                .query_map([&type_name], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            report.push(MissingBlueprintReport {
                type_name,
                required_by_operations: goals,
            });
        }

        Ok(report)
    }

    /// Blueprint readiness for one operation: every requirement, whether
    /// it's owned, and whether an owned copy is currently researching or
    /// copying (a warning, distinct from missing). Always computed live —
    /// same precedent as Operation Engine's `is_blocked`.
    pub fn readiness_for_operation(&self, operation_id: i64) -> Result<BlueprintReadiness, String> {
        let operation_goal: String = self
            .conn
            .query_row(
                "SELECT goal FROM operations WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("operation {operation_id} not found: {e}"))?;

        let mut required = Vec::new();
        {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT r.type_name, r.reason,
                            (EXISTS(SELECT 1 FROM blueprints b WHERE b.type_name = r.type_name)
                             OR EXISTS(SELECT 1 FROM character_blueprints cb JOIN eve_types t ON t.type_id=cb.type_id JOIN characters c ON c.character_id=cb.character_id WHERE t.name=r.type_name AND c.enabled=1 AND c.is_demo=0)) AS is_owned,
                            (SELECT b.status FROM blueprints b
                             WHERE b.type_name = r.type_name AND b.status IN ('researching', 'copying')
                             LIMIT 1) AS warning_status
                     FROM operation_blueprint_requirements r
                     WHERE r.operation_id = ?1
                     ORDER BY r.id",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([operation_id], |row| {
                    let is_owned: i64 = row.get(2)?;
                    Ok(BlueprintRequirement {
                        type_name: row.get(0)?,
                        reason: row.get(1)?,
                        is_owned: is_owned != 0,
                        warning_status: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?;
            for row in rows {
                required.push(row.map_err(|e| e.to_string())?);
            }
        }

        let owned_count = required.iter().filter(|r| r.is_owned).count() as i64;
        let missing_count = required.iter().filter(|r| !r.is_owned).count() as i64;
        let warning_count = required.iter().filter(|r| r.warning_status.is_some()).count() as i64;

        Ok(BlueprintReadiness {
            operation_id,
            operation_goal,
            required,
            owned_count,
            missing_count,
            warning_count,
        })
    }

    /// Readiness for every operation that has at least one blueprint
    /// requirement — backs Mission Control's Blueprint Readiness panel.
    pub fn readiness_all(&self) -> Result<Vec<BlueprintReadiness>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT r.operation_id
                 FROM operation_blueprint_requirements r
                 JOIN operations o ON o.operation_id = r.operation_id AND o.is_demo = 0
                 ORDER BY r.operation_id",
            )
            .map_err(|e| e.to_string())?;
        let operation_ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        for id in operation_ids {
            out.push(self.readiness_for_operation(id)?);
        }
        Ok(out)
    }
}

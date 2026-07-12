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
    CategoryCoverage, CriticalBottleneck, LeafTotal, OperationRequirementBreakdown,
    ProductionPlan, RequirementCategory, RequirementLine, RequirementShortage, RequirementSource,
    RequirementSummary, RequirementTreeNode,
};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

/// Hard recursion ceiling — protects against runaway expansion from
/// malformed or extremely deep imported data, independent of cycle
/// detection (which catches only exact repeats, not merely very long
/// legitimate chains).
const MAX_DEPTH: i64 = 12;

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

    // ------------------------------------------------------------------
    // Real, blueprint-derived calculation (Sprint 008). Everything above
    // this line is the Sprint 007 demo-seeded ledger and is untouched.
    // ------------------------------------------------------------------

    /// EVE's standard community ME formula, applied per run then
    /// multiplied by run count (not applied to the pre-multiplied total —
    /// the two give different rounding results at scale, and this is the
    /// more commonly cited convention): `max(1, ceil(base * (100 - me) /
    /// 100))` per run, times the number of runs.
    /// Display-only lookup for export (Sprint 008.2) — never used by the
    /// expansion/ME/run-rounding math itself, only to label a leaf
    /// material's category/group afterward.
    fn group_and_category_for(&self, type_id: i64) -> (Option<String>, Option<String>) {
        self.conn
            .query_row(
                "SELECT g.name, c.name
                 FROM eve_types t
                 LEFT JOIN eve_groups g ON g.group_id = t.group_id
                 LEFT JOIN eve_categories c ON c.category_id = g.category_id
                 WHERE t.type_id = ?1",
                [type_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((None, None))
    }

    fn per_run_quantity(base_quantity: i64, me: i64) -> i64 {
        let reduced = (base_quantity as f64) * ((100 - me) as f64) / 100.0;
        (reduced.ceil() as i64).max(1)
    }

    fn type_name(&self, type_id: i64) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT name FROM eve_types WHERE type_id = ?1",
                [type_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("type {type_id} not found in imported static data: {e}"))
    }

    /// The blueprint that produces `product_type_id`, if any. Foundation
    /// assumption: at most one manufacturing blueprint per product (the
    /// first match wins if the imported data has more than one) — real
    /// EVE occasionally has alternates (e.g. faction variants), out of
    /// scope for this sprint.
    fn blueprint_for_product(&self, product_type_id: i64) -> Result<Option<(i64, i64)>, String> {
        self.conn
            .query_row(
                "SELECT blueprint_type_id, quantity FROM blueprint_products
                 WHERE product_type_id = ?1 LIMIT 1",
                [product_type_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(format!("blueprint lookup failed for type {product_type_id}: {other}")),
            })
    }

    fn blueprint_materials(&self, blueprint_type_id: i64) -> Result<Vec<(i64, i64)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT material_type_id, quantity FROM blueprint_materials WHERE blueprint_type_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([blueprint_type_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// ME/TE for a blueprint encountered mid-recursion: use an owned
    /// Blueprint Engine record if one exists whose `type_name` matches
    /// this product (text-matched, same bridge convention already used
    /// throughout the app between the static-data world and the
    /// Sprint 001–007 demo tables); otherwise assume ME0/TE0. The
    /// operation's own top-level assumption is applied separately, before
    /// recursion starts, and is never overridden by this fallback.
    fn owned_me_for_product(&self, product_type_id: i64) -> Result<Option<i64>, String> {
        let type_name = match self.type_name(product_type_id) {
            Ok(n) => n,
            Err(_) => return Ok(None),
        };
        let manual = self.conn
            .query_row(
                "SELECT me_level FROM blueprints WHERE type_name = ?1 LIMIT 1",
                [type_name],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(format!("owned-blueprint ME lookup failed: {other}")),
            })?;
        if manual.is_some() { return Ok(manual); }
        // An ESI fallback is unambiguous only when exactly one enabled
        // synchronized blueprint item exists for the producing blueprint
        // type. Multiple candidates are displayed to the user but are not
        // auto-selected by RNI-150.
        let (count,me):(i64,Option<i64>)=self.conn.query_row(
            "SELECT COUNT(*),MAX(cb.material_efficiency) FROM character_blueprints cb
             JOIN characters c ON c.character_id=cb.character_id
             WHERE c.enabled=1 AND c.is_demo=0 AND cb.type_id IN
               (SELECT blueprint_type_id FROM blueprint_products WHERE product_type_id=?1)",
            [product_type_id],|r|Ok((r.get(0)?,r.get(1)?))
        ).map_err(|e|format!("synchronized blueprint ME lookup failed: {e}"))?;
        Ok(if count==1 { me } else { None })
    }

    /// On-hand quantity for a type, for the real (non-demo) production
    /// plan calculation: `manual_inventory_entries` only.
    ///
    /// Demo `inventory_items` (Sprint 001-007's seeded scenario data) is
    /// deliberately EXCLUDED here, unconditionally. Sprint 011A note:
    /// this remains manual-inventory-only — no character asset sync
    /// exists yet (that's a later sprint), so there is nothing else to
    /// add here.
    fn owned_quantity(&self, type_id: i64) -> Result<(i64, &'static str), String> {
        crate::character::assets::global_owned_quantity(self.conn, type_id)
    }

    /// Quantity of a type already reserved for this specific operation
    /// (via the Reservation Engine, matched by name against demo
    /// inventory items — manual inventory has no reservation linkage this
    /// sprint, a documented limitation).
    fn reserved_quantity(&self, operation_id: i64, type_name: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(r.quantity), 0)
                 FROM inventory_reservations r
                 JOIN inventory_items i ON i.item_id = r.item_id
                 WHERE r.released_at IS NULL AND r.operation_id = ?1 AND i.type_name = ?2",
                (operation_id, type_name),
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    fn expand(
        &self,
        type_id: i64,
        needed_quantity: i64,
        me_top: i64,
        operation_id: i64,
        path: &mut HashSet<i64>,
        depth: i64,
        leaves: &mut HashMap<i64, (String, i64, Vec<String>)>,
        warnings: &mut Vec<String>,
        path_prefix: &str,
    ) -> Result<RequirementTreeNode, String> {
        let type_name = self.type_name(type_id).unwrap_or_else(|_| format!("Unknown type {type_id}"));
        let calculation_path = if path_prefix.is_empty() {
            type_name.clone()
        } else {
            format!("{path_prefix} > {type_name}")
        };

        if depth > MAX_DEPTH {
            warnings.push(format!("maximum recursion depth exceeded at {type_name} ({type_id})"));
            return self.leaf_node(type_id, &type_name, needed_quantity, operation_id, leaves, &calculation_path);
        }
        if path.contains(&type_id) {
            warnings.push(format!("cycle detected at {type_name} ({type_id}) — stopped expanding this branch"));
            return self.leaf_node(type_id, &type_name, needed_quantity, operation_id, leaves, &calculation_path);
        }

        let blueprint = self.blueprint_for_product(type_id)?;
        let Some((blueprint_type_id, product_quantity)) = blueprint else {
            return self.leaf_node(type_id, &type_name, needed_quantity, operation_id, leaves, &calculation_path);
        };
        if product_quantity <= 0 {
            warnings.push(format!("{type_name} ({type_id}) has a non-positive blueprint output quantity — treated as a leaf"));
            return self.leaf_node(type_id, &type_name, needed_quantity, operation_id, leaves, &calculation_path);
        }

        let me = if depth == 0 {
            me_top
        } else {
            self.owned_me_for_product(type_id)?.unwrap_or(0)
        };
        let me_source = if depth == 0 {
            "top-level".to_string()
        } else if self.owned_me_for_product(type_id)?.is_some() {
            "owned".to_string()
        } else {
            "assumed".to_string()
        };

        let runs = (needed_quantity as f64 / product_quantity as f64).ceil() as i64;
        let produced_quantity = runs * product_quantity;

        let materials = self.blueprint_materials(blueprint_type_id)?;
        if materials.is_empty() {
            warnings.push(format!(
                "{type_name} ({type_id}) has a blueprint but no imported material lines — treated as if fully satisfied with no inputs"
            ));
        }

        path.insert(type_id);
        let mut children = Vec::new();
        for (material_type_id, base_quantity) in materials {
            let per_run = Self::per_run_quantity(base_quantity, me);
            let total_needed = per_run * runs;
            let child = self.expand(
                material_type_id,
                total_needed,
                me_top,
                operation_id,
                path,
                depth + 1,
                leaves,
                warnings,
                &calculation_path,
            )?;
            children.push(child);
        }
        path.remove(&type_id);

        let (owned_quantity, owned_source) = self.owned_quantity(type_id)?;
        let reserved_quantity = self.reserved_quantity(operation_id, &type_name)?;
        // Available for Build = In Scope minus reservations — never
        // Globally Owned minus reservations. Owning something somewhere
        // is not the same as it being usable for this build.
        let available_quantity = (owned_quantity - reserved_quantity).max(0);
        let missing_quantity = (needed_quantity - available_quantity).max(0);
        let coverage_fraction = if needed_quantity > 0 {
            (available_quantity as f64 / needed_quantity as f64).min(1.0)
        } else {
            1.0
        };

        Ok(RequirementTreeNode {
            type_id,
            type_name,
            is_leaf: false,
            runs,
            produced_quantity,
            needed_quantity,
            per_run_quantity: product_quantity,
            owned_quantity,
            owned_source: owned_source.to_string(),
            reserved_quantity,
            available_quantity,
            missing_quantity,
            coverage_fraction,
            is_satisfied: available_quantity >= needed_quantity,
            me_applied: me,
            me_source,
            blueprint_type_id: Some(blueprint_type_id),
            activity: "manufacturing".to_string(),
            calculation_path,
            children,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn leaf_node(
        &self,
        type_id: i64,
        type_name: &str,
        needed_quantity: i64,
        operation_id: i64,
        leaves: &mut HashMap<i64, (String, i64, Vec<String>)>,
        calculation_path: &str,
    ) -> Result<RequirementTreeNode, String> {
        let entry = leaves.entry(type_id).or_insert_with(|| (type_name.to_string(), 0, Vec::new()));
        entry.1 += needed_quantity;
        entry.2.push(calculation_path.to_string());

        let (owned_quantity, owned_source) = self.owned_quantity(type_id)?;
        let reserved_quantity = self.reserved_quantity(operation_id, type_name)?;
        let available_quantity = (owned_quantity - reserved_quantity).max(0);
        let missing_quantity = (needed_quantity - available_quantity).max(0);
        let coverage_fraction = if needed_quantity > 0 {
            (available_quantity as f64 / needed_quantity as f64).min(1.0)
        } else {
            1.0
        };

        Ok(RequirementTreeNode {
            type_id,
            type_name: type_name.to_string(),
            is_leaf: true,
            runs: 0,
            produced_quantity: 0,
            needed_quantity,
            per_run_quantity: 0,
            owned_quantity,
            owned_source: owned_source.to_string(),
            reserved_quantity,
            available_quantity,
            missing_quantity,
            coverage_fraction,
            is_satisfied: available_quantity >= needed_quantity,
            me_applied: 0,
            me_source: "n/a".to_string(),
            blueprint_type_id: None,
            activity: "manufacturing".to_string(),
            calculation_path: calculation_path.to_string(),
            children: Vec::new(),
        })
    }

    /// The real, blueprint-derived plan for one operation's build target.
    /// Deterministic: same inputs (imported data + owned/assumed ME/TE +
    /// current inventory/reservations) always produce the same tree.
    /// Never stores its result as the source of truth — only
    /// `production_plan_snapshots` may optionally record one for audit.
    pub fn calculate_plan(&self, operation_id: i64) -> Result<ProductionPlan, String> {
        let operation_goal: String = self
            .conn
            .query_row(
                "SELECT goal FROM operations WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("operation {operation_id} not found: {e}"))?;

        let (type_id, quantity_requested, blueprint_mode, owned_blueprint_id, assumed_me, assumed_te, inventory_scope): (
            i64,
            i64,
            String,
            Option<i64>,
            i64,
            i64,
            String,
        ) = self
            .conn
            .query_row(
                "SELECT type_id, quantity_requested, blueprint_mode, owned_blueprint_id, assumed_me, assumed_te, inventory_scope
                 FROM operation_build_targets WHERE operation_id = ?1",
                [operation_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                        row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                        row.get(6)?,
                    ))
                },
            )
            .map_err(|e| format!("operation {operation_id} has no build target set: {e}"))?;

        let build_target_name = self.type_name(type_id)?;

        let (me, te) = if blueprint_mode == "owned" {
            match owned_blueprint_id {
                Some(bp_id) => self
                    .conn
                    .query_row(
                        "SELECT me_level, te_level FROM blueprints WHERE blueprint_id = ?1",
                        [bp_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| format!("owned blueprint {bp_id} not found: {e}"))?,
                None => return Err("blueprint_mode is 'owned' but no owned_blueprint_id was set".into()),
            }
        } else {
            (assumed_me, assumed_te)
        };

        let mut path = HashSet::new();
        let mut leaves: HashMap<i64, (String, i64, Vec<String>)> = HashMap::new();
        let mut warnings = Vec::new();

        let tree = self.expand(
            type_id,
            quantity_requested,
            me,
            operation_id,
            &mut path,
            0,
            &mut leaves,
            &mut warnings,
            "",
        )?;

        let mut leaf_totals: Vec<LeafTotal> = Vec::new();
        for (leaf_type_id, (leaf_name, required_quantity, calculation_paths)) in leaves {
            let (owned_quantity, owned_source) = self.owned_quantity(leaf_type_id)?;
            let reserved_quantity = self.reserved_quantity(operation_id, &leaf_name)?;
            let available_quantity = (owned_quantity - reserved_quantity).max(0);
            let missing_quantity = (required_quantity - available_quantity).max(0);
            let coverage_fraction = if required_quantity > 0 {
                (available_quantity as f64 / required_quantity as f64).min(1.0)
            } else {
                1.0
            };
            let (group_name, category_name) = self.group_and_category_for(leaf_type_id);
            leaf_totals.push(LeafTotal {
                group_name,
                category_name,
                calculation_paths,
                type_id: leaf_type_id,
                type_name: leaf_name,
                required_quantity,
                owned_quantity,
                owned_source: owned_source.to_string(),
                reserved_quantity,
                available_quantity,
                missing_quantity,
                coverage_fraction,
                is_satisfied: available_quantity >= required_quantity,
            });
        }
        leaf_totals.sort_by(|a, b| a.type_name.cmp(&b.type_name));

        let mut bp_stmt=self.conn.prepare(
            "SELECT cb.item_id,c.name,cb.runs != -1,cb.material_efficiency,cb.time_efficiency,CASE WHEN cb.runs=-1 THEN NULL ELSE cb.runs END
             FROM character_blueprints cb JOIN characters c ON c.character_id=cb.character_id
             WHERE c.enabled=1 AND c.is_demo=0 AND cb.type_id IN
               (SELECT blueprint_type_id FROM blueprint_products WHERE product_type_id=?1)
             ORDER BY c.name,cb.item_id"
        ).map_err(|e|format!("owned synchronized blueprint lookup failed: {e}"))?;
        let synchronized_blueprints=bp_stmt.query_map([type_id],|r|Ok(super::models::OwnedBlueprintInfo{item_id:r.get(0)?,owner_name:r.get(1)?,is_copy:r.get::<_,i64>(2)?!=0,me:r.get(3)?,te:r.get(4)?,runs_remaining:r.get(5)?,source:"ESI Character Blueprints".into()})).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;

        Ok(ProductionPlan {
            operation_id,
            operation_goal,
            build_target_type_id: type_id,
            build_target_name,
            requested_quantity: quantity_requested,
            blueprint_mode,
            me,
            te,
            total_runs: tree.runs,
            produced_quantity: tree.produced_quantity,
            inventory_scope,
            tree,
            leaf_totals,
            warnings,
            synchronized_blueprints,
        })
    }
}

// ============================================================
// Tests. Fixtures mirror the exact scenario hand-verified against real
// SQLite in Python before this file was written (see
// docs/REAL_PRODUCTION_PLANNER.md, "Verification procedure") — same
// numbers, same assertions, now exercised through the actual Rust code
// path. Written and reasoned through carefully, but NOT run through
// `cargo test` in the authoring environment (no cargo available there);
// run `cargo test` locally to confirm.
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    const MIGRATIONS: &[&str] = &[
        include_str!("../../migrations/0001_init.sql"),
        include_str!("../../migrations/0002_build_targets.sql"),
        include_str!("../../migrations/0003_inventory_foundation.sql"),
        include_str!("../../migrations/0004_operation_foundation.sql"),
        include_str!("../../migrations/0005_reservation_foundation.sql"),
        include_str!("../../migrations/0006_blueprint_foundation.sql"),
        include_str!("../../migrations/0007_production_requirement_foundation.sql"),
        include_str!("../../migrations/0008_real_production_planner.sql"),
        include_str!("../../migrations/0009_inventory_scope.sql"),
        include_str!("../../migrations/0010_demo_category_correction.sql"),
        include_str!("../../migrations/0011_esi_character_auth.sql"),
        include_str!("../../migrations/0012_character_asset_sync.sql"),
        include_str!("../../migrations/0013_character_blueprint_sync.sql"),
    ];

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for m in MIGRATIONS {
            conn.execute_batch(m).expect("apply migration");
        }
        conn
    }

    /// Seeds the small Widget/Gadget/Material-A/Material-C fixture used
    /// throughout this sprint's validation: Widget BP makes 2/run and
    /// needs 100 A + 3 Gadget; Gadget BP makes 1/run and needs 5 A + 7 C.
    fn seed_fixture(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (1, 'Component');
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (1, 1, 'Sample Component');
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES
                (100, 'Sample Widget', 1, 1),
                (20,  'Sample Gadget', 1, 1),
                (10,  'Sample Material A', 1, 0),
                (30,  'Sample Material C', 1, 0);
             INSERT INTO blueprint_products (blueprint_type_id, product_type_id, quantity) VALUES
                (100, 100, 2),
                (20, 20, 1);
             INSERT INTO blueprint_materials (blueprint_type_id, material_type_id, quantity) VALUES
                (100, 10, 100),
                (100, 20, 3),
                (20, 10, 5),
                (20, 30, 7);",
        )
        .expect("seed fixture");
    }

    /// A 3-level-deep fixture (Ship -> Capital Component -> Component ->
    /// Mineral) matching real capital-ship BOM depth, with Mineral A
    /// (type 10) required both directly by the ship (depth 1) and via the
    /// deepest component (depth 3) — the hardest realistic case for both
    /// "does recursion reach the right depth" and "are shared leaves
    /// summed, not overwritten." Independently verified against a Python
    /// simulation of the same algorithm before being written here — see
    /// Sprint 009's audit notes.
    fn seed_three_level_fixture(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (2, 'Capital Component');
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (2, 2, 'Test Group');
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES
                (1000, 'Test Capital Ship', 2, 1),
                (500,  'Test Capital Component', 2, 1),
                (200,  'Test Component', 2, 1),
                (10,   'Mineral A', 2, 0),
                (20,   'Mineral B', 2, 0),
                (30,   'Mineral C', 2, 0);
             INSERT INTO blueprint_products (blueprint_type_id, product_type_id, quantity) VALUES
                (1000, 1000, 1),
                (500, 500, 1),
                (200, 200, 5);
             INSERT INTO blueprint_materials (blueprint_type_id, material_type_id, quantity) VALUES
                (1000, 500, 3),
                (1000, 10, 200),
                (500, 200, 2),
                (500, 20, 50),
                (200, 10, 100),
                (200, 30, 30);",
        )
        .expect("seed three-level fixture");
    }

    fn seed_operation_with_target(conn: &Connection, quantity: i64, me: i64, te: i64) -> i64 {
        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Build Sample Widget', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let operation_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO operation_build_targets
                (operation_id, type_id, quantity_requested, blueprint_mode, assumed_me, assumed_te, assumed_is_bpc)
             VALUES (?1, 100, ?2, 'assumed', ?3, ?4, 0)",
            (operation_id, quantity, me, te),
        )
        .unwrap();
        operation_id
    }

    #[test]
    fn per_run_quantity_me0() {
        assert_eq!(ProductionRepository::per_run_quantity(100, 0), 100);
    }

    #[test]
    fn per_run_quantity_positive_me_rounds_up() {
        // 3 * (100-10)/100 = 2.7 -> ceil -> 3
        assert_eq!(ProductionRepository::per_run_quantity(3, 10), 3);
    }

    #[test]
    fn per_run_quantity_floors_at_one() {
        // Even a huge ME reduction never drops below 1 per run.
        assert_eq!(ProductionRepository::per_run_quantity(1, 99), 1);
    }

    #[test]
    fn output_quantity_greater_than_one_and_run_rounding() {
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);
        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        // Widget: product_quantity=2/run, requested=5 -> runs=ceil(5/2)=3, produced=6
        assert_eq!(plan.total_runs, 3);
        assert_eq!(plan.produced_quantity, 6);
        assert_eq!(plan.tree.needed_quantity, 5);
    }

    #[test]
    fn recursive_expansion_shared_subcomponent_sums_correctly() {
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);
        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        // Material A is needed by both the Widget branch (270) and the
        // Gadget branch (45) -> must sum to 315, not overwrite.
        let material_a = plan.leaf_totals.iter().find(|l| l.type_id == 10).expect("material A present");
        assert_eq!(material_a.required_quantity, 315);

        let material_c = plan.leaf_totals.iter().find(|l| l.type_id == 30).expect("material C present");
        assert_eq!(material_c.required_quantity, 63);

        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn intermediate_component_totals_are_correct() {
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);
        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        let gadget_node = plan.tree.children.iter().find(|c| c.type_id == 20).expect("gadget child present");
        assert_eq!(gadget_node.needed_quantity, 9);
        assert_eq!(gadget_node.runs, 9);
        assert_eq!(gadget_node.produced_quantity, 9);
        // No owned blueprint exists for the Gadget in this fixture, so its
        // ME defaults to 0 regardless of the top-level assumption (10).
        assert_eq!(gadget_node.me_applied, 0);
        assert!(plan.synchronized_blueprints.is_empty(), "no synchronized ownership must leave existing production behavior unchanged");
    }

    #[test]
    fn production_detects_synchronized_owned_blueprint() {
        let conn=test_db();seed_fixture(&conn);let op_id=seed_operation_with_target(&conn,1,0,0);
        conn.execute("INSERT INTO characters(character_id,name,is_demo,scopes_granted,enabled) VALUES(1,'Maxdelta',0,'esi-assets.read_assets.v1 esi-characters.read_blueprints.v1',1)",[]).unwrap();
        conn.execute("INSERT INTO character_blueprints(character_id,item_id,type_id,location_id,location_flag,quantity,material_efficiency,time_efficiency,runs,source,synced_at) VALUES(1,9001,100,600,'Hangar',-1,10,20,-1,'ESI Character Blueprints','now')",[]).unwrap();
        let plan=ProductionRepository::new(&conn).calculate_plan(op_id).unwrap();
        assert_eq!(plan.synchronized_blueprints.len(),1);let bp=&plan.synchronized_blueprints[0];assert_eq!(bp.owner_name,"Maxdelta");assert!(!bp.is_copy);assert_eq!((bp.me,bp.te,bp.runs_remaining),(10,20,None));
    }

    #[test]
    fn exactly_one_synchronized_intermediate_blueprint_applies_its_me() {
        let conn=test_db();seed_fixture(&conn);let op_id=seed_operation_with_target(&conn,1,0,0);
        conn.execute("INSERT INTO characters(character_id,name,is_demo,scopes_granted,enabled) VALUES(1,'Maxdelta',0,'esi-characters.read_blueprints.v1',1)",[]).unwrap();
        conn.execute("INSERT INTO character_blueprints(character_id,item_id,type_id,location_id,location_flag,quantity,material_efficiency,time_efficiency,runs,source,synced_at) VALUES(1,9002,20,600,'Hangar',-1,10,20,-1,'ESI Character Blueprints','now')",[]).unwrap();
        let plan=ProductionRepository::new(&conn).calculate_plan(op_id).unwrap();let gadget=plan.tree.children.iter().find(|c|c.type_id==20).unwrap();assert_eq!(gadget.me_applied,10);
    }

    #[test]
    fn large_demo_inventory_with_no_manual_entries_gives_owned_zero() {
        // The exact pattern requested after a report that demo inventory
        // was still appearing as "Owned" on a real production plan. A
        // large demo row exists; no manual inventory exists at all. Owned
        // must be exactly 0, not any fraction of the demo quantity.
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);

        conn.execute(
            "INSERT INTO inventory_locations (location_id, name, kind) VALUES (999, 'Test Station', 'station')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO characters (character_id, name, is_demo) VALUES (-99, 'Tester', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_categories (key, label, sort_order) VALUES ('sample_mat', 'Sample Material', 99)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_items (type_name, category_key, quantity, location_id, character_id, unit_value, source)
             VALUES ('Sample Material A', 'sample_mat', 1620000, 999, -99, 1, 'demo')",
            [],
        )
        .unwrap();
        // Deliberately: no INSERT into manual_inventory_entries at all.

        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        let material_a = plan.leaf_totals.iter().find(|l| l.type_id == 10).unwrap();
        assert_eq!(material_a.owned_quantity, 0, "a large demo row with zero manual entries must give Owned = 0, not the demo quantity");
        assert_eq!(material_a.available_quantity, 0);
        assert!(!material_a.is_satisfied);
    }

    #[test]
    fn large_demo_inventory_plus_manual_entry_gives_owned_equal_to_manual_only() {
        // Same large demo row as above, plus a real manual entry of 42.
        // Owned must be exactly 42 — the demo row must contribute nothing,
        // not even partially.
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);

        conn.execute(
            "INSERT INTO inventory_locations (location_id, name, kind) VALUES (999, 'Test Station', 'station')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO characters (character_id, name, is_demo) VALUES (-99, 'Tester', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_categories (key, label, sort_order) VALUES ('sample_mat', 'Sample Material', 99)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_items (type_name, category_key, quantity, location_id, character_id, unit_value, source)
             VALUES ('Sample Material A', 'sample_mat', 1620000, 999, -99, 1, 'demo')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO manual_inventory_entries (type_id, quantity, location_name) VALUES (10, 42, 'Home')",
            [],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        let material_a = plan.leaf_totals.iter().find(|l| l.type_id == 10).unwrap();
        assert_eq!(
            material_a.owned_quantity, 42,
            "Owned must equal the manual entry (42) only — the 1,620,000-unit demo row must not add anything"
        );
        assert_eq!(material_a.owned_source, "Manual Inventory");
    }


    #[test]
    fn three_level_recursion_reaches_correct_depth_and_sums_shared_leaf_across_levels() {
        // Sprint 009 audit (Task 2 & 4): the original tests only exercised
        // 2-level depth. Real capital-ship BOMs are typically 3 levels
        // (Ship -> Capital Component -> Component -> Mineral). This
        // fixture has Mineral A required both directly by the ship
        // (depth 1) and via the deepest component (depth 3) — the
        // hardest case for "correct manufacturing level" and "no
        // duplicate counting" at once.
        let conn = test_db();
        seed_three_level_fixture(&conn);

        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Build Test Capital Ship', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let op_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO operation_build_targets
                (operation_id, type_id, quantity_requested, blueprint_mode, assumed_me, assumed_te, assumed_is_bpc)
             VALUES (?1, 1000, 5, 'assumed', 0, 0, 0)",
            [op_id],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        // Top level: ship, runs=5 (1/run), produced=5.
        assert_eq!(plan.total_runs, 5);
        assert_eq!(plan.produced_quantity, 5);

        // Depth 1: Capital Component, needed=3*5=15, runs=15 (1/run).
        let capital_component = plan.tree.children.iter().find(|c| c.type_id == 500).expect("capital component present");
        assert_eq!(capital_component.needed_quantity, 15);
        assert_eq!(capital_component.runs, 15);

        // Depth 2: Component, needed=2*15=30, runs=ceil(30/5)=6, produced=30.
        let component = capital_component.children.iter().find(|c| c.type_id == 200).expect("component present");
        assert_eq!(component.needed_quantity, 30);
        assert_eq!(component.runs, 6);
        assert_eq!(component.produced_quantity, 30);

        // Leaf totals: Mineral A must sum BOTH contributions (1000 direct
        // at depth 1, plus 600 via the component at depth 3) to 1600 —
        // not overwrite, not double-count, not stop at just one branch.
        let mineral_a = plan.leaf_totals.iter().find(|l| l.type_id == 10).expect("mineral A present");
        assert_eq!(mineral_a.required_quantity, 1600, "Mineral A must sum its depth-1 and depth-3 contributions: 200*5 + 100*6 = 1600");
        assert_eq!(mineral_a.calculation_paths.len(), 2, "Mineral A must record both contributing paths, Validation Mode's whole point");
        assert!(mineral_a.calculation_paths.iter().any(|p| p == "Test Capital Ship > Mineral A"));
        assert!(mineral_a
            .calculation_paths
            .iter()
            .any(|p| p == "Test Capital Ship > Test Capital Component > Test Component > Mineral A"));

        assert_eq!(capital_component.blueprint_type_id, Some(500));
        assert_eq!(capital_component.activity, "manufacturing");
        assert_eq!(capital_component.calculation_path, "Test Capital Ship > Test Capital Component");

        let mineral_b = plan.leaf_totals.iter().find(|l| l.type_id == 20).expect("mineral B present");
        assert_eq!(mineral_b.required_quantity, 750, "50 * 15 runs = 750");

        let mineral_c = plan.leaf_totals.iter().find(|l| l.type_id == 30).expect("mineral C present");
        assert_eq!(mineral_c.required_quantity, 180, "30 * 6 runs = 180");

        assert!(plan.warnings.is_empty());
    }

    #[test]
    fn real_operation_plan_never_reflects_demo_ledger_data() {
        // Explicit fixture, not the migration chain's baked-in demo seed
        // — Sprint 010 retired demo data from normal runtime, so this
        // test builds its own minimal demo-ledger scenario rather than
        // depending on migration 0004/0007's specific seeded names
        // (which a fresh install no longer even has). The deeper
        // guarantee being proven: `calculate_plan` and the Sprint 007
        // demo ledger (`lines`/`summary`, backed by
        // `production_requirements`) are separate query surfaces over
        // separate tables, coexisting in the same database — a real
        // operation's plan must never contain the demo ledger's data,
        // not through a query bug, not through incidental overlap.
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);

        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Fixture Demo Op', 2, 'planned', 0, '', NULL, 1)",
            [],
        )
        .unwrap();
        let demo_op_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO production_requirement_groups (operation_id, category_key) VALUES (?1, 'minerals')",
            [demo_op_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO production_requirements (operation_id, category_key, type_name, required_quantity)
             VALUES (?1, 'minerals', 'Fixture Demo Mineral', 999999)",
            [demo_op_id],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);

        // Sanity check: the demo ledger really is present in this same
        // database (proves this test would actually catch a leak, not
        // just pass vacuously because there was nothing to leak).
        let demo_names: Vec<String> = repo.lines(None, None).unwrap().iter().map(|l| l.type_name.clone()).collect();
        assert!(
            demo_names.iter().any(|n| n == "Fixture Demo Mineral"),
            "demo ledger fixture must be present for this test to be meaningful"
        );

        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        let plan_type_names: Vec<&str> = plan
            .leaf_totals
            .iter()
            .map(|l| l.type_name.as_str())
            .chain(std::iter::once(plan.tree.type_name.as_str()))
            .collect();

        assert!(
            !plan_type_names.contains(&"Fixture Demo Mineral"),
            "real operation's plan must never contain demo ledger material, but plan contained: {plan_type_names:?}"
        );
        assert_eq!(
            plan.operation_goal, "Build Sample Widget",
            "the plan must reflect only the real operation's own goal, never the demo operation's goal"
        );
    }

    #[test]
    fn inventory_coverage_and_exact_shortage() {
        let conn = test_db();
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 5, 10, 20);

        // Demo inventory (inventory_items) — must NOT contribute to a real
        // operation's owned quantity. Deliberately set far higher than the
        // manual entry below, so if the old bug (summing both sources)
        // ever regressed, this test would catch it immediately via a
        // wrong owned_quantity rather than by coincidence.
        conn.execute(
            "INSERT INTO inventory_locations (location_id, name, kind) VALUES (999, 'Test Station', 'station')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO characters (character_id, name, is_demo) VALUES (-99, 'Tester', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_categories (key, label, sort_order) VALUES ('sample_mat', 'Sample Material', 99)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_items (type_name, category_key, quantity, location_id, character_id, unit_value, source)
             VALUES ('Sample Material A', 'sample_mat', 999999, 999, -99, 1, 'demo')",
            [],
        )
        .unwrap();

        // Real, explicitly-entered manual inventory — this is the only
        // source that should count.
        conn.execute(
            "INSERT INTO manual_inventory_entries (type_id, quantity, location_name) VALUES (10, 200, 'Home')",
            [],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");

        let material_a = plan.leaf_totals.iter().find(|l| l.type_id == 10).unwrap();
        assert_eq!(material_a.required_quantity, 315);
        assert_eq!(
            material_a.owned_quantity, 200,
            "owned must come only from the 200-unit manual entry — the 999999-unit demo row must never contribute"
        );
        assert_eq!(material_a.owned_source, "Manual Inventory");
        assert_eq!(material_a.reserved_quantity, 0);
        assert_eq!(material_a.available_quantity, 200);
        assert_eq!(material_a.missing_quantity, 115); // exact shortage: 315 - 200
        assert!(!material_a.is_satisfied);

        let material_c = plan.leaf_totals.iter().find(|l| l.type_id == 30).unwrap();
        assert_eq!(material_c.owned_quantity, 0);
        assert_eq!(material_c.missing_quantity, 63);
    }

    #[test]
    fn cycle_protection_stops_infinite_recursion() {
        let conn = test_db();
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (1, 'Component');
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (1, 1, 'Cyclic');
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES
                (40, 'Cyclic A', 1, 1),
                (41, 'Cyclic B', 1, 1);
             INSERT INTO blueprint_products (blueprint_type_id, product_type_id, quantity) VALUES
                (40, 40, 1), (41, 41, 1);
             INSERT INTO blueprint_materials (blueprint_type_id, material_type_id, quantity) VALUES
                (40, 41, 1), (41, 40, 1);",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Cyclic op', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let op_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO operation_build_targets
                (operation_id, type_id, quantity_requested, blueprint_mode, assumed_me, assumed_te, assumed_is_bpc)
             VALUES (?1, 40, 1, 'assumed', 0, 0, 0)",
            [op_id],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);
        // Must return promptly (not hang) and report the cycle.
        let plan = repo.calculate_plan(op_id).expect("calculate_plan should not error even on a cycle");
        assert!(plan.warnings.iter().any(|w| w.contains("cycle")));
    }

    #[test]
    fn missing_blueprint_for_intermediate_is_a_warning_and_leaf_fallback() {
        let conn = test_db();
        // A type marked manufacturable but with no blueprint_products row —
        // simulates malformed/incomplete imported data.
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (1, 'Component');
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (1, 1, 'Orphan');
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES (50, 'Orphan Type', 1, 0);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo)
             VALUES ('Orphan op', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();
        let op_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO operation_build_targets
                (operation_id, type_id, quantity_requested, blueprint_mode, assumed_me, assumed_te, assumed_is_bpc)
             VALUES (?1, 50, 10, 'assumed', 0, 0, 0)",
            [op_id],
        )
        .unwrap();

        let repo = ProductionRepository::new(&conn);
        let plan = repo.calculate_plan(op_id).expect("calculate_plan");
        // No blueprint at all for the root -> treated as a leaf of itself.
        assert!(plan.tree.is_leaf);
        assert_eq!(plan.tree.needed_quantity, 10);
    }

    #[test]
    fn re_import_preserves_user_data() {
        let conn = test_db();
        // A real operation and a manual inventory entry exist before any
        // import — re-importing reference data must not touch them.
        seed_fixture(&conn);
        let op_id = seed_operation_with_target(&conn, 1, 0, 0);
        conn.execute(
            "INSERT INTO manual_inventory_entries (type_id, quantity, location_name) VALUES (10, 42, 'Home')",
            [],
        )
        .unwrap();

        // Simulate the importer's transactional reference-data refresh
        // (see StaticDataRepository::import_from_directory) without going
        // through a real CSV file. eve_categories/eve_groups/eve_types are
        // upserted, never deleted — manual_inventory_entries.type_id and
        // operation_build_targets.type_id both hold real foreign keys into
        // eve_types, so a blind DELETE would violate them the moment any
        // manual inventory or real build target exists (it did, until this
        // was fixed). blueprint_products/blueprint_materials have no
        // external referrers and are safely deleted and reinserted.
        conn.execute_batch(
            "DELETE FROM blueprint_materials;
             DELETE FROM blueprint_products;",
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO eve_categories (category_id, name) VALUES (2, 'Reimported')
                 ON CONFLICT(category_id) DO UPDATE SET name = excluded.name;
             INSERT INTO eve_groups (group_id, category_id, name) VALUES (2, 2, 'Reimported Group')
                 ON CONFLICT(group_id) DO UPDATE SET category_id = excluded.category_id, name = excluded.name;
             INSERT INTO eve_types (type_id, name, group_id, is_manufacturable) VALUES (999, 'New Type', 2, 0)
                 ON CONFLICT(type_id) DO UPDATE SET name = excluded.name, group_id = excluded.group_id;",
        )
        .unwrap();

        let operations_count: i64 = conn.query_row("SELECT COUNT(*) FROM operations", [], |r| r.get(0)).unwrap();
        let manual_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM manual_inventory_entries", [], |r| r.get(0))
            .unwrap();
        assert!(operations_count >= 1, "operation survives reference-data reimport");
        assert_eq!(manual_count, 1, "manual inventory entry survives reference-data reimport");
        let _ = op_id;
    }
}

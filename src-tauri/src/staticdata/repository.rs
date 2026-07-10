//! `StaticDataRepository` — the only place that reads import files and
//! writes SQL against the reference-data tables. Mirrors the repository
//! pattern from `inventory`, `operation`, `reservation`, `blueprint`, and
//! `production`, adapted for a file-driven import rather than a live
//! query surface.
//!
//! Scope note: this sprint's importer reads the CSV-derivative SDE format
//! (invTypes.csv, invGroups.csv, invCategories.csv,
//! industryActivityProducts.csv, industryActivityMaterials.csv) — the
//! same shape long-established community tools export from the official
//! SDE — rather than parsing CCP's raw multi-file YAML package directly.
//! Building and shipping a full YAML SDE parser without a real fixture
//! file to verify it against was judged too large a risk for one sprint;
//! the CSV format is still 100% locally-sourced, official-data-derived,
//! and involves no network access.

//! `StaticDataRepository` — the only place that reads import files and
//! writes SQL against the reference-data tables. Mirrors the repository
//! pattern from `inventory`, `operation`, `reservation`, `blueprint`, and
//! `production`, adapted for a file-driven import rather than a live
//! query surface.
//!
//! Two import sources, one shared transaction path
//! (`apply_parsed_import`): the CSV-derivative sample fixture format
//! (invTypes.csv etc. — see `import_from_directory`) and, as of Sprint
//! 008.2, CCP's official JSON Lines SDE export (see `import_official_sde`
//! in `jsonl.rs`). Both parse into the same normalized `Parsed*` shapes
//! (`models.rs`) before handing off here, so the upsert/transaction logic
//! that keeps user data safe on re-import exists exactly once, not twice.

use super::models::{
    ImportSummary, ParsedCategory, ParsedGroup, ParsedMaterial, ParsedProduct, ParsedType,
    TypeSearchResult,
};
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct CsvCategory {
    #[serde(rename = "categoryID")]
    category_id: i64,
    #[serde(rename = "categoryName")]
    category_name: String,
    published: i64,
}

#[derive(Deserialize)]
struct CsvGroup {
    #[serde(rename = "groupID")]
    group_id: i64,
    #[serde(rename = "groupName")]
    group_name: String,
    #[serde(rename = "categoryID")]
    category_id: i64,
    published: i64,
}

#[derive(Deserialize)]
struct CsvType {
    #[serde(rename = "typeID")]
    type_id: i64,
    #[serde(rename = "typeName")]
    type_name: String,
    #[serde(rename = "groupID")]
    group_id: i64,
    published: i64,
}

#[derive(Deserialize)]
struct CsvActivityProduct {
    #[serde(rename = "typeID")]
    blueprint_type_id: i64,
    #[serde(rename = "activityID")]
    activity_id: i64,
    #[serde(rename = "productTypeID")]
    product_type_id: i64,
    quantity: i64,
}

#[derive(Deserialize)]
struct CsvActivityMaterial {
    #[serde(rename = "typeID")]
    blueprint_type_id: i64,
    #[serde(rename = "activityID")]
    activity_id: i64,
    #[serde(rename = "materialTypeID")]
    material_type_id: i64,
    quantity: i64,
}

const MANUFACTURING_ACTIVITY_ID: i64 = 1;

pub struct StaticDataRepository<'a> {
    conn: &'a Connection,
}

impl<'a> StaticDataRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Reads the five expected CSV files from `dir_path` and hands the
    /// parsed, normalized records to `apply_parsed_import`. Malformed rows
    /// are skipped and reported, not silently dropped; a missing file
    /// fails the whole import (no partial table replacement).
    pub fn import_from_directory(&self, dir_path: &str) -> Result<ImportSummary, String> {
        let dir = Path::new(dir_path);
        let required = [
            "invCategories.csv",
            "invGroups.csv",
            "invTypes.csv",
            "industryActivityProducts.csv",
            "industryActivityMaterials.csv",
        ];
        for f in required {
            if !dir.join(f).is_file() {
                return Err(format!(
                    "required file '{f}' not found in '{dir_path}' — expected the CSV-derivative SDE bundle (invCategories.csv, invGroups.csv, invTypes.csv, industryActivityProducts.csv, industryActivityMaterials.csv)"
                ));
            }
        }

        let mut errors: Vec<String> = Vec::new();

        let raw_categories = Self::read_csv::<CsvCategory>(&dir.join("invCategories.csv"), &mut errors, "invCategories.csv");
        let raw_groups = Self::read_csv::<CsvGroup>(&dir.join("invGroups.csv"), &mut errors, "invGroups.csv");
        let raw_types = Self::read_csv::<CsvType>(&dir.join("invTypes.csv"), &mut errors, "invTypes.csv");
        let raw_products = Self::read_csv::<CsvActivityProduct>(
            &dir.join("industryActivityProducts.csv"),
            &mut errors,
            "industryActivityProducts.csv",
        );
        let raw_materials = Self::read_csv::<CsvActivityMaterial>(
            &dir.join("industryActivityMaterials.csv"),
            &mut errors,
            "industryActivityMaterials.csv",
        );

        let categories: Vec<ParsedCategory> = raw_categories
            .iter()
            .map(|c| ParsedCategory {
                category_id: c.category_id,
                name: c.category_name.clone(),
                published: c.published != 0,
            })
            .collect();
        let groups: Vec<ParsedGroup> = raw_groups
            .iter()
            .map(|g| ParsedGroup {
                group_id: g.group_id,
                name: g.group_name.clone(),
                category_id: g.category_id,
                published: g.published != 0,
            })
            .collect();
        let types: Vec<ParsedType> = raw_types
            .iter()
            .map(|t| ParsedType {
                type_id: t.type_id,
                name: t.type_name.clone(),
                group_id: t.group_id,
                published: t.published != 0,
            })
            .collect();
        let products: Vec<ParsedProduct> = raw_products
            .iter()
            .filter(|p| p.activity_id == MANUFACTURING_ACTIVITY_ID)
            .map(|p| ParsedProduct {
                blueprint_type_id: p.blueprint_type_id,
                product_type_id: p.product_type_id,
                quantity: p.quantity,
            })
            .collect();
        let materials: Vec<ParsedMaterial> = raw_materials
            .iter()
            .filter(|m| m.activity_id == MANUFACTURING_ACTIVITY_ID)
            .map(|m| ParsedMaterial {
                blueprint_type_id: m.blueprint_type_id,
                material_type_id: m.material_type_id,
                quantity: m.quantity,
            })
            .collect();

        let error_summary = if errors.is_empty() { None } else { Some(errors.join("\n")) };

        self.apply_parsed_import(dir_path, "csv_bundle", None, &categories, &groups, &types, &products, &materials, error_summary)
    }

    /// The one shared transaction: upsert categories/groups/types (never
    /// delete — `manual_inventory_entries.type_id` and
    /// `operation_build_targets.type_id` hold real foreign keys into
    /// `eve_types`), fully recompute `is_manufacturable`, and replace
    /// `blueprint_products`/`blueprint_materials` (safe to delete —
    /// nothing holds a foreign key into those). Records an `sde_imports`
    /// row under the given `format` regardless of source. Used by both
    /// `import_from_directory` (CSV) and `jsonl::import_official_sde`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_parsed_import(
        &self,
        source_path: &str,
        format: &str,
        source_build: Option<&str>,
        categories: &[ParsedCategory],
        groups: &[ParsedGroup],
        types: &[ParsedType],
        products: &[ParsedProduct],
        materials: &[ParsedMaterial],
        parse_error_summary: Option<String>,
    ) -> Result<ImportSummary, String> {
        self.conn
            .execute_batch("BEGIN;")
            .map_err(|e| format!("failed to begin import transaction: {e}"))?;

        let result: Result<(), String> = (|| {
            self.conn
                .execute_batch(
                    "DELETE FROM blueprint_materials;
                     DELETE FROM blueprint_products;
                     DELETE FROM blueprint_activities;",
                )
                .map_err(|e| format!("failed to clear reference tables: {e}"))?;

            for c in categories {
                self.conn
                    .execute(
                        "INSERT INTO eve_categories (category_id, name, published) VALUES (?1, ?2, ?3)
                         ON CONFLICT(category_id) DO UPDATE SET name = excluded.name, published = excluded.published",
                        (c.category_id, &c.name, c.published as i64),
                    )
                    .map_err(|e| format!("failed to upsert category {}: {e}", c.category_id))?;
            }
            for g in groups {
                self.conn
                    .execute(
                        "INSERT INTO eve_groups (group_id, category_id, name, published) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(group_id) DO UPDATE SET
                             category_id = excluded.category_id, name = excluded.name, published = excluded.published",
                        (g.group_id, g.category_id, &g.name, g.published as i64),
                    )
                    .map_err(|e| format!("failed to upsert group {}: {e}", g.group_id))?;
            }
            for t in types {
                self.conn
                    .execute(
                        "INSERT INTO eve_types (type_id, name, group_id, published, is_manufacturable)
                         VALUES (?1, ?2, ?3, ?4, 0)
                         ON CONFLICT(type_id) DO UPDATE SET
                             name = excluded.name, group_id = excluded.group_id, published = excluded.published",
                        (t.type_id, &t.name, t.group_id, t.published as i64),
                    )
                    .map_err(|e| format!("failed to upsert type {}: {e}", t.type_id))?;
            }
            self.conn
                .execute("UPDATE eve_types SET is_manufacturable = 0", [])
                .map_err(|e| format!("failed to reset is_manufacturable: {e}"))?;
            for p in products {
                self.conn
                    .execute(
                        "INSERT INTO blueprint_products (blueprint_type_id, product_type_id, quantity)
                         VALUES (?1, ?2, ?3)
                         ON CONFLICT(blueprint_type_id, product_type_id) DO UPDATE SET quantity = excluded.quantity",
                        (p.blueprint_type_id, p.product_type_id, p.quantity),
                    )
                    .map_err(|e| format!("failed to insert blueprint product {}: {e}", p.blueprint_type_id))?;
            }
            for m in materials {
                self.conn
                    .execute(
                        "INSERT INTO blueprint_materials (blueprint_type_id, material_type_id, quantity)
                         VALUES (?1, ?2, ?3)",
                        (m.blueprint_type_id, m.material_type_id, m.quantity),
                    )
                    .map_err(|e| format!("failed to insert blueprint material for bp {}: {e}", m.blueprint_type_id))?;
            }
            self.conn
                .execute(
                    "UPDATE eve_types SET is_manufacturable = 1
                     WHERE type_id IN (SELECT product_type_id FROM blueprint_products)",
                    [],
                )
                .map_err(|e| format!("failed to denormalize is_manufacturable: {e}"))?;

            Ok(())
        })();

        if let Err(e) = result {
            let _ = self.conn.execute_batch("ROLLBACK;");
            return Err(e);
        }

        let status = if parse_error_summary.is_none() { "success" } else { "partial" };

        self.conn
            .execute(
                "INSERT INTO sde_imports
                    (source_path, format, source_build, status,
                     type_count, group_count, category_count, blueprint_count, material_count, error_summary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                (
                    source_path,
                    format,
                    source_build,
                    status,
                    types.len() as i64,
                    groups.len() as i64,
                    categories.len() as i64,
                    products.len() as i64,
                    materials.len() as i64,
                    &parse_error_summary,
                ),
            )
            .map_err(|e| format!("failed to record import metadata: {e}"))?;

        self.conn
            .execute_batch("COMMIT;")
            .map_err(|e| format!("failed to commit import transaction: {e}"))?;

        self.latest_import()
            .map_err(|e| format!("import committed but failed to read back summary: {e}"))
    }

    fn read_csv<T: for<'de> Deserialize<'de>>(
        path: &Path,
        errors: &mut Vec<String>,
        file_label: &str,
    ) -> Vec<T> {
        let mut out = Vec::new();
        let mut reader = match csv::Reader::from_path(path) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{file_label}: could not open file: {e}"));
                return out;
            }
        };
        for (line_no, record) in reader.deserialize::<T>().enumerate() {
            match record {
                Ok(row) => out.push(row),
                Err(e) => errors.push(format!("{file_label} row {}: {e}", line_no + 2)),
            }
        }
        out
    }

    pub fn latest_import(&self) -> Result<ImportSummary, String> {
        self.conn
            .query_row(
                "SELECT id, source_path, format, source_build, imported_at, status,
                        type_count, group_count, category_count, blueprint_count, material_count, error_summary
                 FROM sde_imports ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok(ImportSummary {
                        id: row.get(0)?,
                        source_path: row.get(1)?,
                        format: row.get(2)?,
                        source_build: row.get(3)?,
                        imported_at: row.get(4)?,
                        status: row.get(5)?,
                        type_count: row.get(6)?,
                        group_count: row.get(7)?,
                        category_count: row.get(8)?,
                        blueprint_count: row.get(9)?,
                        material_count: row.get(10)?,
                        error_summary: row.get(11)?,
                    })
                },
            )
            .map_err(|e| format!("no import found: {e}"))
    }

    /// Fast text search over imported types, for the Build Target
    /// Selector. Never returns unpublished types. `is_manufacturable`
    /// comes straight from the denormalized column set at import time.
    pub fn search_types(&self, query: &str, limit: i64) -> Result<Vec<TypeSearchResult>, String> {
        let pattern = format!("%{}%", query.replace('%', ""));
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.type_id, t.name, g.name AS group_name, c.name AS category_name,
                        t.is_manufacturable,
                        (SELECT bp.blueprint_type_id FROM blueprint_products bp
                         WHERE bp.product_type_id = t.type_id LIMIT 1) AS producing_blueprint_type_id
                 FROM eve_types t
                 LEFT JOIN eve_groups g ON g.group_id = t.group_id
                 LEFT JOIN eve_categories c ON c.category_id = g.category_id
                 WHERE t.published = 1 AND t.name LIKE ?1
                 ORDER BY t.is_manufacturable DESC, t.name ASC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map((pattern, limit), |row| {
                let is_manufacturable: i64 = row.get(4)?;
                Ok(TypeSearchResult {
                    type_id: row.get(0)?,
                    name: row.get(1)?,
                    group_name: row.get(2).unwrap_or_else(|_| "Unknown".to_string()),
                    category_name: row.get(3).unwrap_or_else(|_| "Unknown".to_string()),
                    is_manufacturable: is_manufacturable != 0,
                    producing_blueprint_type_id: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

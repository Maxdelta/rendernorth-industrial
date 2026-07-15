//! Official CCP JSON Lines SDE import (Sprint 008.2).
//!
//! **Disclosure, read before trusting this file:** no real official CCP
//! SDE JSONL export was available in the environment this was written in
//! to verify field names against. Every field is looked up through a list
//! of plausible key-name candidates (documented per field below) rather
//! than a single hard-coded name, and localized name objects (`{"en":
//! "..."}, {"de": "..."}`, a pattern CCP's newer exports are known to use
//! for display strings) are unwrapped automatically. This maximizes the
//! chance of working against the real file without ever having seen one,
//! and any record that still doesn't parse is skipped and reported by
//! exact file, line number, and reason — never silently dropped — so a
//! mismatch is immediately diagnosable rather than a mysterious empty
//! import. Treat this path as unverified until you run it tonight; the
//! CSV sample-fixture path (`repository.rs::import_from_directory`)
//! remains the only import path verified end-to-end against real data.
//!
//! Expected files in the selected directory (an already-extracted
//! official JSONL SDE folder — this sprint does not open a ZIP directly,
//! see docs/REAL_PRODUCTION_PLANNER.md for why):
//!   - categories.jsonl
//!   - groups.jsonl
//!   - types.jsonl
//!   - blueprints.jsonl
//!
//! One JSON object per line in each file (JSON Lines format). Malformed
//! or unparseable lines are skipped and counted, not fatal; a missing
//! *file* is fatal (no partial reference-data replacement), same policy
//! as the CSV path.

use super::models::{ParsedCategory, ParsedGroup, ParsedMaterial, ParsedProduct, ParsedType};
use super::repository::StaticDataRepository;
use serde_json::Value;
use std::fs;
use std::path::Path;

const MANUFACTURING_ACTIVITY_KEYS: &[&str] = &["manufacturing", "Manufacturing", "1"];

/// Look up the first present key among `candidates`, unwrapping a
/// localized `{"en": "..."}`-style object into its `en` string if that's
/// the shape found.
fn get_str(v: &Value, candidates: &[&str]) -> Option<String> {
    for key in candidates {
        if let Some(found) = v.get(key) {
            if let Some(s) = found.as_str() {
                return Some(s.to_string());
            }
            if let Some(obj) = found.as_object() {
                if let Some(en) = obj.get("en").and_then(|x| x.as_str()) {
                    return Some(en.to_string());
                }
                if let Some((_, first)) = obj.iter().next() {
                    if let Some(s) = first.as_str() {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

fn get_i64(v: &Value, candidates: &[&str]) -> Option<i64> {
    for key in candidates {
        if let Some(found) = v.get(key) {
            if let Some(n) = found.as_i64() {
                return Some(n);
            }
            if let Some(s) = found.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn get_f64(v: &Value, candidates: &[&str]) -> Option<f64> {
    for key in candidates {
        if let Some(found) = v.get(key) {
            if let Some(number) = found.as_f64() {
                if number.is_finite() {
                    return Some(number);
                }
            }
            if let Some(text) = found.as_str() {
                if let Ok(number) = text.parse::<f64>() {
                    if number.is_finite() {
                        return Some(number);
                    }
                }
            }
        }
    }
    None
}

fn get_bool(v: &Value, candidates: &[&str], default: bool) -> bool {
    for key in candidates {
        if let Some(found) = v.get(key) {
            if let Some(b) = found.as_bool() {
                return b;
            }
        }
    }
    default
}

/// Reads and parses one `.jsonl` file, applying `parse_line` to every
/// non-blank line and collecting successes; failures are pushed onto
/// `errors` with file name and line number, never silently dropped.
fn read_jsonl<T>(
    path: &Path,
    file_label: &str,
    errors: &mut Vec<String>,
    parse_line: impl Fn(&Value) -> Option<T>,
) -> Vec<T> {
    let mut out = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("{file_label}: could not open file: {e}"));
            return out;
        }
    };
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => match parse_line(&v) {
                Some(parsed) => out.push(parsed),
                None => errors.push(format!(
                    "{file_label} line {}: record did not contain the expected fields (checked common CCP field-name variants — see jsonl.rs doc comment)",
                    line_no + 1
                )),
            },
            Err(e) => errors.push(format!("{file_label} line {}: invalid JSON: {e}", line_no + 1)),
        }
    }
    out
}

/// Same as `read_jsonl`, but also returns the 1-based line number each
/// successfully-parsed record came from. Used only for blueprints, where
/// a stub-creation warning needs to point back at the exact source line
/// that referenced a missing type — categories/groups/types don't
/// currently need this since their own records don't reference anything
/// that could be missing.
fn read_jsonl_with_lines(path: &Path, file_label: &str, errors: &mut Vec<String>) -> Vec<(usize, Value)> {
    let mut out = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("{file_label}: could not open file: {e}"));
            return out;
        }
    };
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => out.push((line_no + 1, v)),
            Err(e) => errors.push(format!("{file_label} line {}: invalid JSON: {e}", line_no + 1)),
        }
    }
    out
}

/// Types.jsonl gets its own dedicated parsing pass, rather than sharing
/// the generic `read_jsonl` used by categories/groups, for two reasons:
/// it's by far the largest file (tens of thousands of records in a real
/// export) and the one whose completeness matters most, and it needs
/// per-field skip-reason diagnostics that the generic path doesn't track.
/// `type_id` and `name` are genuinely required (the schema has `NOT
/// NULL` on `name`, and `type_id` is the primary key); `group_id` is
/// NOT required — `eve_types.group_id` has no `NOT NULL`, so a type
/// without a recognizable group is still imported, just with a null
/// group, rather than being rejected outright. This distinction is
/// exactly the bug this function fixes: the first JSONL attempt required
/// group_id and silently dropped every type that didn't have one.
fn parse_types_with_diagnostics(path: &Path, errors: &mut Vec<String>) -> Vec<ParsedType> {
    let mut out = Vec::new();
    let mut expected = 0i64;
    let mut skip_reasons: std::collections::HashMap<&'static str, i64> = std::collections::HashMap::new();

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("types.jsonl: could not open file: {e}"));
            return out;
        }
    };

    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        expected += 1;

        let v = match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => v,
            Err(e) => {
                *skip_reasons.entry("invalid JSON").or_insert(0) += 1;
                errors.push(format!("types.jsonl line {}: invalid JSON: {e}", line_no + 1));
                continue;
            }
        };

        let Some(type_id) = get_i64(&v, &["type_id", "typeID", "_key", "id"]) else {
            *skip_reasons.entry("missing type_id").or_insert(0) += 1;
            errors.push(format!("types.jsonl line {}: no recognizable type_id field — skipped", line_no + 1));
            continue;
        };
        let Some(name) = get_str(&v, &["name", "typeName"]) else {
            *skip_reasons.entry("missing name").or_insert(0) += 1;
            errors.push(format!("types.jsonl line {}: type {type_id} has no recognizable name field — skipped", line_no + 1));
            continue;
        };
        // group_id is intentionally NOT a rejection reason — the schema
        // allows null, so a type without one is still imported.
        let group_id = get_i64(&v, &["group_id", "groupID"]);
        let published = get_bool(&v, &["published"], true);
        let volume_m3 = get_f64(&v, &["volume", "volume_m3", "volumeM3"])
            .filter(|value| *value >= 0.0);

        out.push(ParsedType { type_id, name, group_id, published, volume_m3 });
    }

    let imported = out.len() as i64;
    let skipped = expected - imported;
    // Only surface this as a reportable diagnostic when something was
    // actually skipped — a fully clean import should still report
    // status "success" with no error summary, not be downgraded to
    // "partial" just for stating a good outcome.
    if skipped > 0 {
        let mut reason_parts: Vec<String> = skip_reasons.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        reason_parts.sort();
        let reason_summary = reason_parts.join(", ");
        errors.push(format!(
            "types.jsonl summary: expected {expected} records, imported {imported}, skipped {skipped} (skip reasons — {reason_summary})"
        ));
    }

    out
}

pub fn import_official_sde(
    repo: &StaticDataRepository<'_>,
    dir_path: &str,
) -> Result<super::models::ImportSummary, String> {
    let dir = Path::new(dir_path);
    let required = ["categories.jsonl", "groups.jsonl", "types.jsonl", "blueprints.jsonl"];
    for f in required {
        if !dir.join(f).is_file() {
            return Err(format!(
                "required file '{f}' not found in '{dir_path}' — expected an extracted official CCP JSONL SDE directory containing categories.jsonl, groups.jsonl, types.jsonl, and blueprints.jsonl. If you have the official ZIP, extract it first — this sprint reads an already-extracted directory, not the ZIP directly."
            ));
        }
    }

    let mut errors: Vec<String> = Vec::new();

    let categories: Vec<ParsedCategory> = read_jsonl(&dir.join("categories.jsonl"), "categories.jsonl", &mut errors, |v| {
        Some(ParsedCategory {
            category_id: get_i64(v, &["category_id", "categoryID", "_key", "id"])?,
            name: get_str(v, &["name", "categoryName"])?,
            published: get_bool(v, &["published"], true),
        })
    });

    let groups: Vec<ParsedGroup> = read_jsonl(&dir.join("groups.jsonl"), "groups.jsonl", &mut errors, |v| {
        Some(ParsedGroup {
            group_id: get_i64(v, &["group_id", "groupID", "_key", "id"])?,
            name: get_str(v, &["name", "groupName"])?,
            category_id: get_i64(v, &["category_id", "categoryID"])?,
            published: get_bool(v, &["published"], true),
        })
    });

    let types = parse_types_with_diagnostics(&dir.join("types.jsonl"), &mut errors);

    // Two-pass by construction: categories/groups/types are fully parsed
    // and (later, in apply_parsed_import) fully inserted before a single
    // blueprint row is touched — blueprints are read here, in this same
    // function, strictly after the three calls above. This is what makes
    // it safe for apply_parsed_import to insert all reference-taxonomy
    // rows in pass one and all blueprint-dependent rows in pass two.
    //
    // Blueprints are the least certain shape: try a nested
    // `activities.manufacturing.{materials,products}` structure first
    // (the pattern CCP's newer exports are believed to use), then fall
    // back to flat top-level `materials`/`products` arrays in case the
    // real export isn't nested by activity at all.
    let mut products: Vec<ParsedProduct> = Vec::new();
    let mut materials: Vec<ParsedMaterial> = Vec::new();
    let blueprint_lines = read_jsonl_with_lines(&dir.join("blueprints.jsonl"), "blueprints.jsonl", &mut errors);
    for (line_no, v) in &blueprint_lines {
        let Some(blueprint_type_id) = get_i64(v, &["blueprint_type_id", "blueprintTypeID", "type_id", "typeID", "_key", "id"]) else {
            errors.push(format!("blueprints.jsonl line {line_no}: a record had no recognizable blueprint type id — skipped"));
            continue;
        };

        let manufacturing = MANUFACTURING_ACTIVITY_KEYS
            .iter()
            .find_map(|k| v.get("activities").and_then(|a| a.get(*k)))
            .or(Some(v));

        let Some(manufacturing) = manufacturing else { continue };

        if let Some(product_list) = manufacturing.get("products").and_then(|p| p.as_array()) {
            for p in product_list {
                let product_type_id = get_i64(p, &["type_id", "typeID", "product_type_id", "productTypeID"]);
                let quantity = get_i64(p, &["quantity", "count"]).unwrap_or(1);
                match product_type_id {
                    Some(pid) => products.push(ParsedProduct {
                        blueprint_type_id,
                        product_type_id: pid,
                        quantity,
                        source_line: Some(*line_no),
                    }),
                    None => errors.push(format!(
                        "blueprints.jsonl line {line_no}: blueprint {blueprint_type_id} had a product entry with no recognizable type id — skipped"
                    )),
                }
            }
        }

        if let Some(material_list) = manufacturing.get("materials").and_then(|m| m.as_array()) {
            for m in material_list {
                let material_type_id = get_i64(m, &["type_id", "typeID", "material_type_id", "materialTypeID"]);
                let quantity = get_i64(m, &["quantity", "count"]).unwrap_or(1);
                match material_type_id {
                    Some(mid) => materials.push(ParsedMaterial {
                        blueprint_type_id,
                        material_type_id: mid,
                        quantity,
                        source_line: Some(*line_no),
                    }),
                    None => errors.push(format!(
                        "blueprints.jsonl line {line_no}: blueprint {blueprint_type_id} had a material entry with no recognizable type id — skipped"
                    )),
                }
            }
        }
    }

    let error_summary = if errors.is_empty() { None } else { Some(errors.join("\n")) };

    let source_build = crate::onboarding::detect_source_build(dir);
    repo.apply_parsed_import(dir_path, "jsonl_official", source_build.as_deref(), &categories, &groups, &types, &products, &materials, error_summary)
}

// ============================================================
// Tests. Written and reasoned through carefully; NOT run through `cargo
// test` in the authoring environment (no cargo available there) — run
// `cargo test` locally to confirm. Uses real temp files on disk (no new
// dependency) and a real in-memory SQLite connection with the full
// migration chain applied, mirroring the pattern already established in
// `production::repository::tests`.
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::atomic::{AtomicU32, Ordering};

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
        include_str!("../../migrations/0016_type_volume.sql"),
    ];

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for m in MIGRATIONS {
            conn.execute_batch(m).expect("apply migration");
        }
        conn
    }

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("rni_jsonl_test_{label}_{n}_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// The tolerant field-name matching's core claim: two structurally
    /// different but semantically equivalent JSONL shapes (one using
    /// snake_case + a localized name object, one using the older
    /// camelCase-with-flat-string convention) both parse to the same
    /// result.
    #[test]
    fn tolerant_field_matching_handles_both_naming_conventions() {
        let dir = temp_dir("tolerant");
        fs::write(
            dir.join("categories.jsonl"),
            r#"{"category_id": 1, "name": {"en": "Component"}, "published": true}"#,
        )
        .unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": {"en": "Sample Component"}, "category_id": 1, "published": true}"#).unwrap();
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 100, \"name\": {\"en\": \"Sample Widget\"}, \"group_id\": 1, \"published\": true, \"volume\": 12.5}\n\
             {\"typeID\": 10, \"typeName\": \"Sample Material A\", \"groupID\": 1, \"published\": 1, \"volumeM3\": 0.01}",
        )
        .unwrap();
        fs::write(
            dir.join("blueprints.jsonl"),
            r#"{"blueprint_type_id": 100, "activities": {"manufacturing": {"products": [{"type_id": 100, "quantity": 2}], "materials": [{"type_id": 10, "quantity": 100}]}}}"#,
        )
        .unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap()).expect("import should succeed");

        assert_eq!(summary.type_count, 2);
        assert_eq!(summary.blueprint_count, 1);
        assert_eq!(summary.material_count, 1);

        let widget_name: String = conn
            .query_row("SELECT name FROM eve_types WHERE type_id = 100", [], |r| r.get(0))
            .unwrap();
        assert_eq!(widget_name, "Sample Widget");
        let material_name: String = conn
            .query_row("SELECT name FROM eve_types WHERE type_id = 10", [], |r| r.get(0))
            .unwrap();
        assert_eq!(material_name, "Sample Material A");
        let volumes: (f64, f64) = conn.query_row(
            "SELECT (SELECT volume_m3 FROM eve_types WHERE type_id=100), (SELECT volume_m3 FROM eve_types WHERE type_id=10)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(volumes, (12.5, 0.01));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn blueprint_product_and_material_mapping_is_correct() {
        let dir = temp_dir("bp_mapping");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 100, \"name\": \"Widget\", \"group_id\": 1, \"published\": true}\n\
             {\"type_id\": 10, \"name\": \"Material A\", \"group_id\": 1, \"published\": true}\n\
             {\"type_id\": 20, \"name\": \"Material B\", \"group_id\": 1, \"published\": true}",
        )
        .unwrap();
        fs::write(
            dir.join("blueprints.jsonl"),
            r#"{"blueprint_type_id": 100, "activities": {"manufacturing": {"products": [{"type_id": 100, "quantity": 3}], "materials": [{"type_id": 10, "quantity": 50}, {"type_id": 20, "quantity": 7}]}}}"#,
        )
        .unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        import_official_sde(&repo, dir.to_str().unwrap()).expect("import should succeed");

        let (product_type_id, quantity): (i64, i64) = conn
            .query_row("SELECT product_type_id, quantity FROM blueprint_products WHERE blueprint_type_id = 100", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(product_type_id, 100);
        assert_eq!(quantity, 3);

        let material_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM blueprint_materials WHERE blueprint_type_id = 100", [], |r| r.get(0))
            .unwrap();
        assert_eq!(material_count, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_lines_are_skipped_and_reported_not_fatal() {
        let dir = temp_dir("malformed");
        fs::write(dir.join("categories.jsonl"), "{\"category_id\": 1, \"name\": \"Component\", \"published\": true}\nnot json at all\n").unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(dir.join("types.jsonl"), r#"{"type_id": 100, "name": "Widget", "group_id": 1, "published": true}"#).unwrap();
        fs::write(dir.join("blueprints.jsonl"), "").unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap()).expect("import should still succeed overall");

        assert_eq!(summary.status, "partial");
        assert!(summary.error_summary.is_some());
        assert!(summary.error_summary.unwrap().contains("categories.jsonl"));
        // The one good category row still made it in.
        assert_eq!(summary.category_count, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_fails_the_whole_import_no_partial_replacement() {
        let dir = temp_dir("missing_file");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        // groups.jsonl, types.jsonl, blueprints.jsonl intentionally absent.

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let result = import_official_sde(&repo, dir.to_str().unwrap());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("groups.jsonl"), "error should name the missing file: {err}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reimport_via_jsonl_preserves_operations_and_manual_inventory() {
        let dir = temp_dir("reimport");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(dir.join("types.jsonl"), r#"{"type_id": 10, "name": "Material A", "group_id": 1, "published": true}"#).unwrap();
        fs::write(dir.join("blueprints.jsonl"), "").unwrap();

        let conn = test_db();

        // Seed the type once (as if a prior import already ran), then
        // attach real user data to it.
        conn.execute(
            "INSERT INTO eve_categories (category_id, name, published) VALUES (1, 'Component', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO eve_groups (group_id, category_id, name, published) VALUES (1, 1, 'Sample', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO eve_types (type_id, name, group_id, published, is_manufacturable) VALUES (10, 'Material A', 1, 1, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO manual_inventory_entries (type_id, quantity, location_name) VALUES (10, 42, 'Home')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO operations (goal, priority, status, progress, notes, deadline, is_demo) VALUES ('op', 2, 'planned', 0, '', NULL, 0)",
            [],
        )
        .unwrap();

        let repo = StaticDataRepository::new(&conn);
        import_official_sde(&repo, dir.to_str().unwrap()).expect("re-import should succeed");

        let manual_count: i64 = conn.query_row("SELECT COUNT(*) FROM manual_inventory_entries", [], |r| r.get(0)).unwrap();
        // Migration 0004 seeds 6 demo operations into every fresh test
        // database, so asserting an exact total here would be checking the
        // wrong thing — what actually matters is that *this test's own*
        // operation specifically survived the re-import, regardless of how
        // many other (demo) rows happen to coexist with it.
        let test_op_survives: i64 = conn
            .query_row("SELECT COUNT(*) FROM operations WHERE goal = 'op'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(manual_count, 1, "manual inventory survives JSONL re-import");
        assert_eq!(test_op_survives, 1, "the specific real operation created in this test survives JSONL re-import");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_type_search_works_after_jsonl_import() {
        let dir = temp_dir("search");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 6, "name": "Ship", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 30, "name": "Titan", "category_id": 6, "published": true}"#).unwrap();
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 11567, \"name\": \"Avatar\", \"group_id\": 30, \"published\": true}\n\
             {\"type_id\": 11568, \"name\": \"Erebus\", \"group_id\": 30, \"published\": true}",
        )
        .unwrap();
        fs::write(
            dir.join("blueprints.jsonl"),
            r#"{"blueprint_type_id": 11567, "activities": {"manufacturing": {"products": [{"type_id": 11567, "quantity": 1}], "materials": []}}}"#,
        )
        .unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        import_official_sde(&repo, dir.to_str().unwrap()).expect("import should succeed");

        let results = repo.search_types("Avatar", 10).expect("search should succeed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Avatar");
        assert!(results[0].is_manufacturable, "Avatar has a blueprint in this fixture");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn blueprint_referencing_undefined_type_self_heals_instead_of_failing() {
        // This is the exact shape of the real bug this fix addresses:
        // real official CCP data referenced a product type_id (37398 in
        // the original report) that never appeared among the successfully
        // parsed types — whether from a genuine gap in the export or a
        // record this importer's tolerant parsing didn't recognize. This
        // must no longer fail the whole import; a placeholder stub is
        // created for the missing type instead, and reported as a
        // warning, so the import completes with status "partial" rather
        // than failing outright.
        let dir = temp_dir("self_heal");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(dir.join("types.jsonl"), r#"{"type_id": 200, "name": "Known Type", "group_id": 1, "published": true}"#).unwrap();
        // material_type_id 999 is never defined in types.jsonl — the exact
        // real-world gap that used to produce "FOREIGN KEY constraint
        // failed".
        fs::write(
            dir.join("blueprints.jsonl"),
            r#"{"blueprint_type_id": 200, "activities": {"manufacturing": {"products": [{"type_id": 200, "quantity": 1}], "materials": [{"type_id": 999, "quantity": 1}]}}}"#,
        )
        .unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap())
            .expect("import must self-heal rather than fail on an undefined referenced type");

        assert_eq!(summary.status, "partial", "a stub creation must be reported as a partial import, not silently treated as success");
        let warnings = summary.error_summary.expect("stub creation must be recorded in the error summary");
        assert!(warnings.contains("999"), "the warning must name the specific type that needed a placeholder: {warnings}");
        assert!(warnings.contains("blueprint 200"), "the warning must name which blueprint referenced the missing type: {warnings}");
        assert!(warnings.contains("material"), "the warning must say which role (product/material) the missing type played: {warnings}");
        assert!(warnings.contains("blueprints.jsonl line"), "the warning must include the source file and line number: {warnings}");

        let stub_name: String = conn
            .query_row("SELECT name FROM eve_types WHERE type_id = 999", [], |r| r.get(0))
            .expect("the stub row must actually exist, satisfying the foreign key");
        assert_eq!(stub_name, "Unknown Type 999");

        let material_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM blueprint_materials WHERE blueprint_type_id = 200", [], |r| r.get(0))
            .unwrap();
        assert_eq!(material_count, 1, "the material line itself must still have been inserted, not dropped");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn self_healing_never_fires_for_a_complete_dataset() {
        // The other half of the fix's correctness: a normal, complete
        // import — where every referenced type/group/category really is
        // provided — must produce zero stub warnings. Stub creation uses
        // `ON CONFLICT DO NOTHING`, so a real row inserted first always
        // wins; this confirms that in practice, not just by inspection.
        let dir = temp_dir("no_false_stubs");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 100, \"name\": \"Widget\", \"group_id\": 1, \"published\": true}\n\
             {\"type_id\": 10, \"name\": \"Material A\", \"group_id\": 1, \"published\": true}",
        )
        .unwrap();
        fs::write(
            dir.join("blueprints.jsonl"),
            r#"{"blueprint_type_id": 100, "activities": {"manufacturing": {"products": [{"type_id": 100, "quantity": 2}], "materials": [{"type_id": 10, "quantity": 100}]}}}"#,
        )
        .unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap()).expect("complete import should succeed");

        assert_eq!(summary.status, "success", "a fully complete dataset must never be marked partial");
        assert!(summary.error_summary.is_none(), "a fully complete dataset must produce zero stub warnings");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn type_without_group_id_is_imported_not_rejected() {
        // This is the actual root cause behind the real-world report: a
        // batch of otherwise-valid type IDs (37236-37279 in the original
        // report) went missing from eve_types because the first JSONL
        // attempt required group_id to be present and parseable, silently
        // dropping the entire type record if not — even though
        // eve_types.group_id has no NOT NULL constraint. A type with no
        // recognizable group_id must still be imported, with a null
        // group, not rejected.
        let dir = temp_dir("no_group_id");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 100, \"name\": \"Grouped Type\", \"group_id\": 1, \"published\": true}\n\
             {\"type_id\": 200, \"name\": \"Ungrouped Type\", \"published\": true}",
        )
        .unwrap();
        fs::write(dir.join("blueprints.jsonl"), "").unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap()).expect("import should succeed");

        assert_eq!(summary.type_count, 2, "both types must be imported, including the one with no group_id");

        let ungrouped_exists: i64 = conn
            .query_row("SELECT COUNT(*) FROM eve_types WHERE type_id = 200", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ungrouped_exists, 1, "the type without a group_id must exist in eve_types, not have been silently dropped");

        let ungrouped_group_id: Option<i64> = conn
            .query_row("SELECT group_id FROM eve_types WHERE type_id = 200", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ungrouped_group_id, None, "its group_id should be null, matching the schema, not a rejection reason");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn types_skip_diagnostics_report_expected_imported_skipped_and_reasons() {
        let dir = temp_dir("skip_diagnostics");
        fs::write(dir.join("categories.jsonl"), r#"{"category_id": 1, "name": "Component", "published": true}"#).unwrap();
        fs::write(dir.join("groups.jsonl"), r#"{"group_id": 1, "name": "Sample", "category_id": 1, "published": true}"#).unwrap();
        // 4 total records: 1 good, 1 missing type_id, 1 missing name, 1 invalid JSON.
        fs::write(
            dir.join("types.jsonl"),
            "{\"type_id\": 100, \"name\": \"Good Type\", \"group_id\": 1, \"published\": true}\n\
             {\"name\": \"No Type Id\", \"group_id\": 1, \"published\": true}\n\
             {\"type_id\": 300, \"group_id\": 1, \"published\": true}\n\
             not valid json at all",
        )
        .unwrap();
        fs::write(dir.join("blueprints.jsonl"), "").unwrap();

        let conn = test_db();
        let repo = StaticDataRepository::new(&conn);
        let summary = import_official_sde(&repo, dir.to_str().unwrap()).expect("import should still succeed overall");

        assert_eq!(summary.status, "partial");
        assert_eq!(summary.type_count, 1, "only the one genuinely valid type should be imported");

        let warnings = summary.error_summary.expect("skip diagnostics must be present");
        assert!(warnings.contains("expected 4 records"), "{warnings}");
        assert!(warnings.contains("imported 1"), "{warnings}");
        assert!(warnings.contains("skipped 3"), "{warnings}");
        assert!(warnings.contains("missing type_id: 1"), "{warnings}");
        assert!(warnings.contains("missing name: 1"), "{warnings}");
        assert!(warnings.contains("invalid JSON: 1"), "{warnings}");

        let _ = fs::remove_dir_all(&dir);
    }
}

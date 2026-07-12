//! Static Data Import DTOs. Mirror `src/lib/backend.ts` — change both
//! together. This module owns official reference-data ingestion only: it
//! never touches operations, inventory, reservations, or settings.

use serde::Serialize;

/// Normalized, source-agnostic parsed records (Sprint 008.2). Both the
/// sample CSV importer and the official JSONL importer parse into these
/// same shapes before handing off to
/// `StaticDataRepository::apply_parsed_import` — one shared transaction/
/// upsert path, not two parallel ones, regardless of source format.
pub struct ParsedCategory {
    pub category_id: i64,
    pub name: String,
    pub published: bool,
}

pub struct ParsedGroup {
    pub group_id: i64,
    pub name: String,
    pub category_id: i64,
    pub published: bool,
}

pub struct ParsedType {
    pub type_id: i64,
    pub name: String,
    /// Nullable, matching `eve_types.group_id`'s actual schema (no `NOT
    /// NULL`). Sprint 008.2's first JSONL attempt wrongly *required* this
    /// field to be present and parseable — rejecting the entire type
    /// record if not, silently dropping any type CCP's real data doesn't
    /// assign a recognizable group to. That was the real cause of a large
    /// consecutive range of valid type IDs going missing from `eve_types`
    /// (surfaced only indirectly, as blueprint-reference placeholder
    /// warnings, once blueprints.jsonl referenced those same IDs).
    pub group_id: Option<i64>,
    pub published: bool,
}

pub struct ParsedProduct {
    pub blueprint_type_id: i64,
    pub product_type_id: i64,
    pub quantity: i64,
    /// Line number in the source file this was parsed from, when known
    /// (JSONL only — the CSV path already reports row numbers through its
    /// own per-row error handling in `read_csv`). Used to make a stub-
    /// creation warning traceable back to the exact record that needed
    /// one, not just the type id.
    pub source_line: Option<usize>,
}

pub struct ParsedMaterial {
    pub blueprint_type_id: i64,
    pub material_type_id: i64,
    pub quantity: i64,
    pub source_line: Option<usize>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub id: i64,
    pub source_path: String,
    pub format: String,
    pub source_build: Option<String>,
    pub imported_at: String,
    pub status: String, // success / partial / failed
    pub type_count: i64,
    pub group_count: i64,
    pub category_count: i64,
    pub blueprint_count: i64,
    pub material_count: i64,
    pub error_summary: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TypeSearchResult {
    pub type_id: i64,
    pub name: String,
    pub group_name: String,
    pub category_name: String,
    pub is_manufacturable: bool,
    /// Present when a blueprint produces this type — the blueprint's own
    /// type_id, so the caller can immediately look up its materials.
    pub producing_blueprint_type_id: Option<i64>,
}


-- RenderNorth Industrial — migration 0008 (Sprint 008)
-- Real Production Planner vertical slice. Additive only; 0001–0007 are
-- untouched.
--
-- This migration adds two families of tables:
--
-- 1. Static reference data (eve_categories, eve_groups, eve_types,
--    blueprint_activities, blueprint_products, blueprint_materials,
--    sde_imports) — owned by the Static Data Import module. Populated
--    from a locally-selected file, never a network call. Re-importable:
--    a fresh import replaces reference rows but never touches operations,
--    inventory, reservations, or settings.
--
-- 2. Real planner tables (operation_build_targets, manual_inventory_entries)
--    — a user's actual build target/quantity/blueprint choice for an
--    operation, and a user's actual, manually-entered inventory. These
--    coexist with the Sprint 001–007 demo tables rather than replacing
--    them, same coexistence pattern as operations/build_projects.
--
-- Existing demo data (6 operations, 10 blueprints) is preserved and now
-- explicitly flagged via an additive is_demo column, so it can never be
-- confused with a real operation or a real owned blueprint going forward.

-- ============================================================
-- Static reference data (Static Data Import module)
-- ============================================================

CREATE TABLE IF NOT EXISTS sde_imports (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    source_path    TEXT NOT NULL,
    format         TEXT NOT NULL, -- csv_bundle (invTypes/invGroups/invCategories/industryActivityProducts/industryActivityMaterials)
    source_build   TEXT,          -- optional build/version identifier, if present in the source
    imported_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    status         TEXT NOT NULL, -- success / partial / failed
    type_count     INTEGER NOT NULL DEFAULT 0,
    group_count    INTEGER NOT NULL DEFAULT 0,
    category_count INTEGER NOT NULL DEFAULT 0,
    blueprint_count INTEGER NOT NULL DEFAULT 0,
    material_count INTEGER NOT NULL DEFAULT 0,
    error_summary  TEXT -- newline-joined list of malformed/unsupported rows, NULL if none
);

CREATE TABLE IF NOT EXISTS eve_categories (
    category_id INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    published   INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS eve_groups (
    group_id    INTEGER PRIMARY KEY,
    category_id INTEGER REFERENCES eve_categories(category_id),
    name        TEXT NOT NULL,
    published   INTEGER NOT NULL DEFAULT 1
);

-- `is_manufacturable` is denormalized at import time (true iff some row in
-- blueprint_products has product_type_id = this type). It is reference
-- data refreshed wholesale on every re-import, not a live-derived engine
-- number like coverage/is_blocked elsewhere in the app — a different,
-- legitimate kind of "stored" value because the whole table is replaced
-- atomically on import, never hand-edited.
CREATE TABLE IF NOT EXISTS eve_types (
    type_id            INTEGER PRIMARY KEY,
    name               TEXT NOT NULL,
    group_id           INTEGER REFERENCES eve_groups(group_id),
    published          INTEGER NOT NULL DEFAULT 1,
    is_manufacturable  INTEGER NOT NULL DEFAULT 0
);

-- Manufacturing activity only this sprint (no reactions, invention, or
-- copying) — `activity` is a free column so those can be added later
-- without a schema change.
CREATE TABLE IF NOT EXISTS blueprint_activities (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    blueprint_type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    activity          TEXT NOT NULL DEFAULT 'manufacturing',
    time_seconds      INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS blueprint_products (
    blueprint_type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    product_type_id   INTEGER NOT NULL REFERENCES eve_types(type_id),
    quantity          INTEGER NOT NULL,
    PRIMARY KEY (blueprint_type_id, product_type_id)
);

CREATE TABLE IF NOT EXISTS blueprint_materials (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    blueprint_type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    material_type_id  INTEGER NOT NULL REFERENCES eve_types(type_id),
    quantity          INTEGER NOT NULL
);

-- ============================================================
-- Real planner tables
-- ============================================================

-- Marks existing (and future) demo operations so they can never be
-- confused with a real, user-created one. Additive; existing rows
-- backfilled below.
ALTER TABLE operations ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0;
UPDATE operations SET is_demo = 1;

-- Same for blueprints: the 10 seeded in migration 0006 are demo capability
-- records, not something a real owner researched or copied.
ALTER TABLE blueprints ADD COLUMN is_demo INTEGER NOT NULL DEFAULT 0;
UPDATE blueprints SET is_demo = 1;

-- One row per operation that has a real build target selected. Nullable
-- owned_blueprint_id/assumed_* columns because an operation may plan
-- against either an owned blueprint or an assumption, never both.
CREATE TABLE IF NOT EXISTS operation_build_targets (
    operation_id        INTEGER PRIMARY KEY REFERENCES operations(operation_id),
    type_id             INTEGER NOT NULL REFERENCES eve_types(type_id),
    quantity_requested  INTEGER NOT NULL,
    blueprint_mode      TEXT NOT NULL, -- owned / assumed
    owned_blueprint_id  INTEGER REFERENCES blueprints(blueprint_id),
    assumed_is_bpc      INTEGER,
    assumed_me          INTEGER,
    assumed_te          INTEGER,
    assumed_runs        INTEGER, -- only meaningful when assumed_is_bpc = 1
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Real, user-entered inventory. Deliberately a separate ledger from
-- `inventory_items` (migration 0003), which remains the untouched demo
-- inventory system — this table is 100% real data, always. No is_demo
-- column: rows here are never demo by construction.
CREATE TABLE IF NOT EXISTS manual_inventory_entries (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    type_id      INTEGER NOT NULL REFERENCES eve_types(type_id),
    quantity     INTEGER NOT NULL,
    location_name TEXT NOT NULL DEFAULT 'Unspecified',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Optional audit snapshot of a calculation, for reproducibility. Never
-- read back as authoritative coverage — every live view recomputes.
CREATE TABLE IF NOT EXISTS requirement_calculation_snapshots (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id        INTEGER NOT NULL REFERENCES operations(operation_id),
    calculated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    source_build        TEXT, -- sde_imports.source_build at calculation time, if any
    inputs_json         TEXT NOT NULL, -- {type_id, quantity, me, te, is_bpc, runs}
    result_json         TEXT NOT NULL  -- the full computed tree + leaf totals, as returned to the UI
);

-- ============================================================
-- No demo seed in this migration — static reference data is populated by
-- the Static Data Import workflow (Settings), not by a fixed SQL seed,
-- since it must reflect whatever file the user actually imports.
-- ============================================================

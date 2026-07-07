-- RenderNorth Industrial — migration 0006 (Sprint 006)
-- Blueprint Domain Foundation. Additive only; 0001–0005 are untouched.
--
-- Blueprints are industrial capability records, not just inventory items.
-- `inventory_items` (migration 0003) already has a handful of rows in the
-- "blueprints" category — those remain and are unchanged; they represent
-- a blueprint as a physical stack sitting in a location, the same as any
-- other item. This migration adds a richer, dedicated domain on top: BPO
-- vs BPC, ME/TE level, runs remaining, research/copy status, and what an
-- operation needs versus what's actually owned. Where a natural match
-- exists, a new `blueprints` row links back to its `inventory_items`
-- counterpart via `inventory_item_id` (nullable) — same coexistence
-- pattern as `operations`/`build_projects` from migration 0004.
--
-- Blueprint Engine owns industrial capability. It does not touch
-- inventory tables, reservation tables, or operation lifecycle — it only
-- answers what blueprints exist, their research/copy state, and whether
-- an operation's required blueprints are actually on hand.

CREATE TABLE IF NOT EXISTS blueprints (
    blueprint_id      INTEGER PRIMARY KEY,
    type_name         TEXT NOT NULL,          -- what it produces, e.g. "Avatar Blueprint Copy"
    is_copy           INTEGER NOT NULL DEFAULT 0, -- 0 = BPO (original), 1 = BPC (copy)
    me_level          INTEGER NOT NULL DEFAULT 0,
    te_level          INTEGER NOT NULL DEFAULT 0,
    runs_remaining    INTEGER,                -- NULL for a BPO (infinite runs); an integer for a BPC
    character_id      INTEGER REFERENCES characters(character_id),
    location_id       INTEGER REFERENCES inventory_locations(location_id),
    status            TEXT NOT NULL DEFAULT 'idle', -- idle / researching / copying / in_use
    inventory_item_id INTEGER REFERENCES inventory_items(item_id),
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- What an operation needs, by blueprint type name — independent of
-- whether it's currently owned. Missing-blueprint detection is always a
-- live comparison against this table and `blueprints`, never a stored
-- flag (same "derive, don't trust a stale flag" precedent as
-- Operation Engine's `is_blocked` and Reservation Engine's `conflicts`).
CREATE TABLE IF NOT EXISTS operation_blueprint_requirements (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id INTEGER NOT NULL REFERENCES operations(operation_id),
    type_name    TEXT NOT NULL,
    required_me  INTEGER, -- minimum ME needed; NULL = no minimum
    required_te  INTEGER, -- minimum TE needed; NULL = no minimum
    reason       TEXT NOT NULL
);

-- ============================================================
-- Demo seed. Ten blueprints spanning BPOs and BPCs, several statuses, and
-- one deliberately unowned requirement (Capital Jump Drive Blueprint,
-- needed by both Avatar and Prepare Titan Components) so missing-blueprint
-- detection has something genuine to find.
-- ============================================================

INSERT INTO blueprints
    (blueprint_id, type_name, is_copy, me_level, te_level, runs_remaining, character_id, location_id, status, inventory_item_id) VALUES
    (1,  'Avatar Blueprint Copy',                 1, 10, 20, 1,    -1, 3, 'idle',        4),
    (2,  'Capital Construction Parts Blueprint',  0, 8,  12, NULL, -1, 3, 'idle',        5),
    (3,  'Capital Armor Plates Blueprint',         0, 6,  8,  NULL, -1, 3, 'researching', 6),
    (4,  'Broadcast Node Blueprint Copy',         1, 0,  0,  10,   -2, 9, 'idle',        7),
    (5,  'Broadcast Node Blueprint Copy',         1, 2,  4,  5,    -2, 9, 'idle',        NULL),
    (6,  'Navy Revelation Blueprint',              0, 4,  6,  NULL, -1, 2, 'idle',        NULL),
    (7,  'Apostle Blueprint Copy',                1, 0,  0,  3,    -1, 3, 'copying',     NULL),
    (8,  'Capital Capacitor Batteries Blueprint',  0, 10, 10, NULL, -1, 3, 'idle',        NULL),
    (9,  'Auto-Integrity Preservation Seal Blueprint', 0, 5, 5, NULL, -1, 3, 'idle',      NULL),
    (10, 'Capital Shield Extender II Blueprint',   0, 3,  3,  NULL, -2, 6, 'idle',        NULL)
ON CONFLICT(blueprint_id) DO NOTHING;

INSERT INTO operation_blueprint_requirements (id, operation_id, type_name, required_me, required_te, reason) VALUES
    (1, 1, 'Avatar Blueprint Copy',                1,    NULL, 'Primary hull blueprint for the Avatar build'),
    (2, 1, 'Capital Construction Parts Blueprint',  NULL, NULL, 'Feeds the capital component line for Avatar'),
    (3, 1, 'Capital Jump Drive Blueprint',          NULL, NULL, 'Jump drive component not yet blueprinted'),
    (4, 2, 'Navy Revelation Blueprint',             NULL, NULL, 'Primary hull blueprint for the Navy Revelation build'),
    (5, 2, 'Capital Armor Plates Blueprint',        NULL, NULL, 'Faction armor tier component supply'),
    (6, 3, 'Apostle Blueprint Copy',                NULL, NULL, 'Primary hull blueprint for the Apostle build'),
    (7, 3, 'Capital Capacitor Batteries Blueprint', NULL, NULL, 'Capacitor component supply for Apostle'),
    (8, 4, 'Capital Construction Parts Blueprint',  NULL, NULL, 'Direct production blueprint for this run'),
    (9, 5, 'Broadcast Node Blueprint Copy',         NULL, NULL, 'PI commodity blueprint for this run'),
    (10, 6, 'Capital Construction Parts Blueprint', NULL, NULL, 'Shares the Capital Construction Parts supply with Avatar'),
    (11, 6, 'Capital Jump Drive Blueprint',         NULL, NULL, 'Shared Titan-class component staging requirement')
ON CONFLICT(id) DO NOTHING;

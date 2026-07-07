-- RenderNorth Industrial — migration 0007 (Sprint 007)
-- Production Requirement Engine Foundation. Additive only; 0001–0006 are
-- untouched.
--
-- The Production Requirement Engine answers "what does this operation
-- actually require to complete?" It owns required inputs — never owns
-- inventory, reservations, operation lifecycle, or blueprints. Coverage,
-- shortage, and bottleneck figures are always computed live by joining
-- this table's required quantities against `inventory_items` (and, for
-- operation-specific reservation context, `inventory_reservations`) —
-- never a stored, editable flag. This is the same "derive, don't trust a
-- stale flag" precedent as Operation Engine's `is_blocked`, Reservation
-- Engine's `conflicts()`, and Blueprint Engine's missing-blueprint report.

-- What categories of requirement exist for an operation. Structural
-- scope, not computed numbers — an operation can declare it will need
-- Reaction Materials before a single line item is itemized. Coverage
-- math never reads from this table; it exists purely for category
-- grouping and display order (via a join to inventory_categories).
CREATE TABLE IF NOT EXISTS production_requirement_groups (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id INTEGER NOT NULL REFERENCES operations(operation_id),
    category_key TEXT NOT NULL REFERENCES inventory_categories(key)
);

-- The concrete ledger: what quantity of a specific material an operation
-- requires. `type_name` matches `inventory_items.type_name` the same way
-- `missing_materials` and `operation_blueprint_requirements` already do —
-- owned/shortage/coverage are computed by joining on it, never stored
-- here.
CREATE TABLE IF NOT EXISTS production_requirements (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id     INTEGER NOT NULL REFERENCES operations(operation_id),
    category_key     TEXT NOT NULL REFERENCES inventory_categories(key),
    type_name        TEXT NOT NULL,
    required_quantity INTEGER NOT NULL,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Provenance: why this requirement exists and how its quantity was
-- justified. A requirement can in principle have more than one
-- contributing source (e.g. a blueprint material line plus a manual
-- buffer) — this sprint seeds exactly one source per requirement, but the
-- shape supports more once real derivation (SDE-based material trees)
-- lands.
CREATE TABLE IF NOT EXISTS production_requirement_sources (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    requirement_id       INTEGER NOT NULL REFERENCES production_requirements(id),
    source_kind          TEXT NOT NULL, -- e.g. demo_seed; later: blueprint_material, manual_buffer
    contributed_quantity INTEGER NOT NULL,
    note                 TEXT NOT NULL
);

-- ============================================================
-- Demo seed. Sixteen requirement lines across six operations, chosen so
-- two materials are deliberately over-committed across more than one
-- operation (Megacyte: op2 satisfied, op3 short; Capital Construction
-- Parts: op6 satisfied, op1 short) — genuine, derivable "critical
-- bottleneck" examples, not a hand-set flag.
-- ============================================================

INSERT INTO production_requirements (id, operation_id, category_key, type_name, required_quantity) VALUES
    (1,  1, 'minerals',           'Nocxium',                          2000000),
    (2,  1, 'capital_components', 'Capital Construction Parts',       250),
    (3,  1, 'capital_components', 'Capital Armor Plates',             70),
    (4,  1, 'advanced_components','Auto-Integrity Preservation Seal', 60),
    (5,  1, 'pi',                 'Broadcast Node',                   60),
    (6,  2, 'minerals',           'Megacyte',                         150000),
    (7,  2, 'capital_components', 'Capital Armor Plates',             90),
    (8,  3, 'minerals',           'Megacyte',                         320000),
    (9,  3, 'capital_components', 'Capital Capacitor Batteries',      50),
    (10, 4, 'minerals',           'Tritanium',                        900000000),
    (11, 4, 'reaction_materials', 'Mechanical Parts',                 120000),
    (12, 4, 'fuel',               'Nitrogen Fuel Block',              110000),
    (13, 5, 'pi',                 'Ukomi Superconductors',            320),
    (14, 5, 'pi',                 'Condensates',                      300),
    (15, 5, 'pi',                 'High-Tech Transmitters',           280),
    (16, 6, 'capital_components', 'Capital Construction Parts',       180)
ON CONFLICT(id) DO NOTHING;

INSERT INTO production_requirement_groups (id, operation_id, category_key) VALUES
    (1,  1, 'minerals'),
    (2,  1, 'capital_components'),
    (3,  1, 'advanced_components'),
    (4,  1, 'pi'),
    (5,  2, 'minerals'),
    (6,  2, 'capital_components'),
    (7,  3, 'minerals'),
    (8,  3, 'capital_components'),
    (9,  4, 'minerals'),
    (10, 4, 'reaction_materials'),
    (11, 4, 'fuel'),
    (12, 5, 'pi'),
    (13, 6, 'capital_components')
ON CONFLICT(id) DO NOTHING;

INSERT INTO production_requirement_sources (id, requirement_id, source_kind, contributed_quantity, note) VALUES
    (1,  1,  'demo_seed', 2000000,   'Avatar advanced component and hull plating mineral demand'),
    (2,  2,  'demo_seed', 250,       'Avatar capital hull assembly requirement'),
    (3,  3,  'demo_seed', 70,        'Avatar armor tier requirement'),
    (4,  4,  'demo_seed', 60,        'Avatar advanced component line requirement'),
    (5,  5,  'demo_seed', 60,        'Avatar PI-fed advanced component requirement'),
    (6,  6,  'demo_seed', 150000,    'Navy Revelation mineral demand'),
    (7,  7,  'demo_seed', 90,        'Navy Revelation faction armor tier requirement'),
    (8,  8,  'demo_seed', 320000,    'Apostle mineral demand'),
    (9,  9,  'demo_seed', 50,        'Apostle capacitor component requirement'),
    (10, 10, 'demo_seed', 900000000,'Capital Construction Parts production run mineral demand'),
    (11, 11, 'demo_seed', 120000,   'Capital Construction Parts reaction material demand'),
    (12, 12, 'demo_seed', 110000,   'Capital Construction Parts run fuel demand'),
    (13, 13, 'demo_seed', 320,      'Broadcast Node P3 input demand'),
    (14, 14, 'demo_seed', 300,      'Broadcast Node P3 input demand'),
    (15, 15, 'demo_seed', 280,      'Broadcast Node P3 input demand'),
    (16, 16, 'demo_seed', 180,      'Titan component staging demand, shares supply with Avatar')
ON CONFLICT(id) DO NOTHING;

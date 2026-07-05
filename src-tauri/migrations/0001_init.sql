-- RenderNorth Industrial — migration 0001
-- Sprint 001 foundation tables + demo seed (see docs/DATABASE_SCHEMA.md)

CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS characters (
    character_id INTEGER PRIMARY KEY,
    name         TEXT NOT NULL,
    corporation  TEXT,
    is_demo      INTEGER NOT NULL DEFAULT 0,
    added_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Generic build targets: any manufacturable type. Nothing Avatar-specific.
CREATE TABLE IF NOT EXISTS build_projects (
    project_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name             TEXT NOT NULL,
    target_type_id   INTEGER,          -- FK to sde_types once the SDE lands (NULL for demo)
    target_type_name TEXT NOT NULL,    -- display name until sde_types exists
    status           TEXT NOT NULL DEFAULT 'active',
    overall_progress REAL NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE TABLE IF NOT EXISTS build_requirement_groups (
    group_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES build_projects(project_id) ON DELETE CASCADE,
    label      TEXT NOT NULL,
    coverage   REAL NOT NULL,
    sort_order INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS missing_materials (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER REFERENCES build_projects(project_id) ON DELETE CASCADE,
    type_name  TEXT NOT NULL,
    quantity   INTEGER NOT NULL,
    category   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS factory_snapshot (
    metric TEXT PRIMARY KEY,
    value  REAL NOT NULL,
    as_of  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recommendations (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER REFERENCES build_projects(project_id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    reason       TEXT NOT NULL,
    priority     INTEGER NOT NULL DEFAULT 1,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    dismissed_at TEXT
);

-- ---------- Sprint 001 demo seed (flagged, purgeable) ----------

INSERT INTO app_meta (key, value) VALUES ('seed_version', 'sprint-001-demo')
    ON CONFLICT(key) DO NOTHING;

INSERT INTO characters (character_id, name, corporation, is_demo)
VALUES
    (-1, 'Demo Forgemaster', 'RenderNorth Heavy Works', 1),
    (-2, 'Demo Hauler',      'RenderNorth Heavy Works', 1)
ON CONFLICT(character_id) DO NOTHING;

INSERT INTO build_projects (project_id, name, target_type_name, status, overall_progress)
VALUES (1, 'Avatar', 'Avatar (Titan — Amarr capital hull)', 'active', 0.61)
ON CONFLICT(project_id) DO NOTHING;

INSERT INTO build_requirement_groups (group_id, project_id, label, coverage, sort_order)
VALUES
    (1, 1, 'Minerals',            1.00, 1),
    (2, 1, 'Capital Components',  0.63, 2),
    (3, 1, 'Advanced Components', 0.44, 3),
    (4, 1, 'PI',                  0.80, 4)
ON CONFLICT(group_id) DO NOTHING;

INSERT INTO missing_materials (id, project_id, type_name, quantity, category)
VALUES
    (1, 1, 'Nocxium',                          812000, 'mineral'),
    (2, 1, 'Auto-Integrity Preservation Seal',     43, 'component'),
    (3, 1, 'Broadcast Node',                       18, 'pi')
ON CONFLICT(id) DO NOTHING;

INSERT INTO factory_snapshot (metric, value, as_of)
VALUES
    ('running_jobs',    34,            strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('idle_characters',  2,            strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('idle_bpos',        7,            strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('wallet_isk',      50200000000.0, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
ON CONFLICT(metric) DO NOTHING;

INSERT INTO recommendations (id, project_id, title, reason, priority)
VALUES (
    1,
    1,
    'Start Capital Construction Parts',
    '{"rule_id":"REC-001 idle-slots + complete-inputs + lowest-coverage-component","summary":"Capital component coverage is 63% against a 100% target while mineral coverage is complete and 2 characters with open industry slots are idle. Capital Construction Parts are the longest-lead component line still short.","inputs":{"capital_component_coverage":0.63,"mineral_coverage":1.0,"idle_characters":2}}',
    1
)
ON CONFLICT(id) DO NOTHING;

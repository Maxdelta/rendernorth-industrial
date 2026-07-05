-- RenderNorth Industrial — migration 0004 (Sprint 004)
-- Operation Domain Foundation. Additive only; 0001–0003 are untouched.
--
-- RenderNorth Industrial revolves around Operations, not inventory or
-- blueprints. An Operation represents industrial intent ("Build Avatar",
-- "Manufacture 500 Capital Construction Parts", "Stockpile Broadcast
-- Nodes") and owns its own lifecycle, priority, status, timeline, notes,
-- target, progress, and dependencies. It never owns inventory and never
-- performs production — it requests services from other engines.
--
-- Coexistence note: `build_projects` (migrations 0001–0002) remains the
-- live table backing the existing, working Build Targets / Inventory
-- Coverage flow — it is NOT rewritten here. `operations` below is the new,
-- richer Operation domain table. For this sprint the two share the same
-- id space (operation_id 1–5 mirror build_projects.project_id 1–5) so the
-- demo data tells one coherent story; consolidating them into a single
-- table is deferred to a future sprint once the Operation Engine is ready
-- to take over target selection. Operation 6 has no build_projects
-- counterpart — proof an Operation isn't required to map 1:1 to a hull.

CREATE TABLE IF NOT EXISTS operations (
    operation_id     INTEGER PRIMARY KEY,
    goal             TEXT NOT NULL,           -- "Build Avatar", "Stockpile Broadcast Nodes"...
    target_type_name TEXT,                    -- display target; nullable — a goal need not name one type
    priority         INTEGER NOT NULL DEFAULT 3, -- 1 = highest
    status           TEXT NOT NULL DEFAULT 'planned', -- planned/active/blocked/paused/completed
    progress         REAL NOT NULL DEFAULT 0, -- 0..1; owned directly by the Operation Engine
    notes            TEXT NOT NULL DEFAULT '',
    deadline         TEXT,                    -- ISO date, nullable
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- Ordered milestones. Schema and repository exist so the Operation Engine
-- can own a timeline as specified — the Operations Workspace UI does not
-- render these yet (shown as a reserved placeholder this sprint).
CREATE TABLE IF NOT EXISTS operation_timeline (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id INTEGER NOT NULL REFERENCES operations(operation_id),
    label        TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending', -- pending/in_progress/done/blocked
    sort_order   INTEGER NOT NULL,
    target_date  TEXT
);

-- One operation can depend on another (e.g. Avatar depends on Capital
-- Construction Parts supply). "Blocked" is derived from this table, not a
-- hand-set flag alone: an operation is blocked if its own status says so,
-- OR if anything it depends on isn't complete yet.
CREATE TABLE IF NOT EXISTS operation_dependencies (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id            INTEGER NOT NULL REFERENCES operations(operation_id),
    depends_on_operation_id INTEGER NOT NULL REFERENCES operations(operation_id),
    reason                  TEXT NOT NULL
);

-- ============================================================
-- Demo seed — coherent with the existing build_projects/Inventory demo
-- scenario, plus one standalone operation (6) with no single build target.
-- ============================================================

INSERT INTO operations (operation_id, goal, target_type_name, priority, status, progress, notes, deadline) VALUES
    (1, 'Build Avatar',                      'Avatar (Titan — Amarr capital hull)',                 1, 'active',    0.72, 'Primary capital project. Waiting on capital component and PI supply lines.', '2026-09-01'),
    (2, 'Build Navy Revelation',             'Navy Revelation (Dreadnought — Amarr faction capital)', 2, 'planned',   0.28, 'Faction hull; LP store dependency for Purple Loyalty Tokens.', '2026-11-15'),
    (3, 'Build Apostle',                     'Apostle (Force Auxiliary — Amarr capital)',            3, 'planned',   0.52, 'Secondary support hull. Not yet resourced ahead of Avatar.', NULL),
    (4, 'Manufacture Capital Construction Parts', 'Capital Construction Parts (Capital component)',   1, 'active',    0.95, 'Feeds Avatar directly. Near complete on this run.', '2026-07-20'),
    (5, 'Manufacture Broadcast Nodes',       'Broadcast Node (PI — P4 advanced commodity)',          2, 'planned',   0.68, 'PI chain restock in progress; feeds Avatar''s advanced components.', '2026-07-10'),
    (6, 'Prepare Titan Components',          NULL,                                                    4, 'planned',   0.10, 'Cross-hull staging operation consolidating capital component stockpiles ahead of the next Titan-class build. Not tied to a single build target.', NULL)
ON CONFLICT(operation_id) DO NOTHING;

INSERT INTO operation_timeline (id, operation_id, label, status, sort_order, target_date) VALUES
    (1, 1, 'Mineral stockpile complete',       'done',        1, '2026-05-10'),
    (2, 1, 'Capital components online',        'in_progress', 2, '2026-08-01'),
    (3, 1, 'Hull assembly begins',             'pending',     3, '2026-09-01'),
    (4, 2, 'LP store token accumulation',      'in_progress', 1, '2026-09-30'),
    (5, 2, 'Faction component sourcing',       'pending',     2, '2026-11-15'),
    (6, 3, 'Mineral coverage review',          'in_progress', 1, NULL),
    (7, 4, 'Blueprint research complete',      'done',        1, '2026-06-01'),
    (8, 4, 'Final production run',             'in_progress', 2, '2026-07-20'),
    (9, 5, 'P3 input restock',                 'in_progress', 1, '2026-07-05'),
    (10, 6, 'Component stockpile survey',      'pending',     1, NULL)
ON CONFLICT(id) DO NOTHING;

-- Avatar is blocked on both its component and PI supply operations, even
-- though its own status is "active" — proof `is_blocked` is derived, not
-- a hand-set literal. Operation 6 shares the same component dependency.
INSERT INTO operation_dependencies (id, operation_id, depends_on_operation_id, reason) VALUES
    (1, 1, 4, 'Avatar cannot complete without full Capital Construction Parts coverage'),
    (2, 1, 5, 'Advanced component line needs the Broadcast Node PI chain'),
    (3, 6, 4, 'Shares the Capital Construction Parts supply with Avatar')
ON CONFLICT(id) DO NOTHING;

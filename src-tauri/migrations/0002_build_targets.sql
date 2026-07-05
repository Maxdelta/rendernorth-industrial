-- RenderNorth Industrial — migration 0002 (Sprint 002)
-- Build Target foundation: multiple demo targets, factory health metrics,
-- and the selected-target pointer. Additive only; 0001 is untouched.

-- Which build project Mission Control is currently focused on.
INSERT INTO app_meta (key, value) VALUES ('selected_project_id', '1')
    ON CONFLICT(key) DO NOTHING;

-- ---------- additional demo build targets (generic: hulls, components, PI) ----------

INSERT INTO build_projects (project_id, name, target_type_id, target_type_name, status, overall_progress)
VALUES
    (2, 'Navy Revelation',            NULL, 'Navy Revelation (Dreadnought — Amarr faction capital)', 'planned', 0.24),
    (3, 'Apostle',                    NULL, 'Apostle (Force Auxiliary — Amarr capital)',             'planned', 0.47),
    (4, 'Capital Construction Parts', NULL, 'Capital Construction Parts (Capital component)',       'planned', 0.72),
    (5, 'Broadcast Node',             NULL, 'Broadcast Node (PI — P4 advanced commodity)',          'planned', 0.35)
ON CONFLICT(project_id) DO NOTHING;

-- Requirement tiers are data, not code: labels differ per target on purpose.
INSERT INTO build_requirement_groups (group_id, project_id, label, coverage, sort_order)
VALUES
    -- Navy Revelation
    (5,  2, 'Minerals',            0.55, 1),
    (6,  2, 'Capital Components',  0.18, 2),
    (7,  2, 'Advanced Components', 0.10, 3),
    (8,  2, 'Faction Materials',   0.30, 4),
    -- Apostle
    (9,  3, 'Minerals',            0.80, 1),
    (10, 3, 'Capital Components',  0.40, 2),
    (11, 3, 'Advanced Components', 0.35, 3),
    -- Capital Construction Parts
    (12, 4, 'Minerals',            1.00, 1),
    (13, 4, 'Blueprint Research',  0.90, 2),
    -- Broadcast Node
    (14, 5, 'P3 Inputs',           0.35, 1),
    (15, 5, 'Launchpad Capacity',  1.00, 2)
ON CONFLICT(group_id) DO NOTHING;

INSERT INTO missing_materials (id, project_id, type_name, quantity, category)
VALUES
    -- Navy Revelation
    (4,  2, 'Isogen',                          1400000, 'mineral'),
    (5,  2, 'Capital Armor Plates',                 62, 'component'),
    (6,  2, 'Purple Loyalty Tokens (LP store)',  40000, 'faction'),
    -- Apostle
    (7,  3, 'Megacyte',                          96000, 'mineral'),
    (8,  3, 'Capital Capacitor Batteries',          21, 'component'),
    -- Capital Construction Parts
    (9,  4, 'Tritanium',                       5200000, 'mineral'),
    -- Broadcast Node
    (10, 5, 'Ukomi Superconductors',               120, 'p3'),
    (11, 5, 'Condensates',                         120, 'p3'),
    (12, 5, 'High-Tech Transmitters',              120, 'p3')
ON CONFLICT(id) DO NOTHING;

-- Deterministic recommendations per target (reason JSON: rule_id, summary, inputs).
INSERT INTO recommendations (id, project_id, title, reason, priority)
VALUES
    (2, 2, 'Research capital component BPOs before committing minerals',
     '{"rule_id":"REC-002 coverage-floor-before-spend","summary":"Capital component coverage is 18% and advanced component coverage is 10%; starting mineral purchases now would lock ISK ahead of the true bottleneck. Bring component blueprint lines online first.","inputs":{"capital_component_coverage":0.18,"advanced_component_coverage":0.10,"mineral_coverage":0.55}}',
     1),
    (3, 3, 'Buy Megacyte before the weekend restock cycle',
     '{"rule_id":"REC-003 single-mineral-blocker","summary":"Megacyte is the only mineral below target (96k short) while both component tiers have active jobs; clearing it unblocks the next component wave.","inputs":{"megacyte_missing":96000,"mineral_coverage":0.80,"capital_component_coverage":0.40}}',
     1),
    (4, 4, 'Start Capital Construction Parts runs on idle slots',
     '{"rule_id":"REC-001 idle-slots + complete-inputs + lowest-coverage-component","summary":"Mineral coverage is 100% and research is at 90%; idle manufacturing slots can begin runs immediately with owned stock.","inputs":{"mineral_coverage":1.0,"blueprint_research":0.90,"idle_characters":2}}',
     1),
    (5, 5, 'Restock P3 inputs across launchpads',
     '{"rule_id":"REC-004 pi-input-floor","summary":"All three P3 inputs are at 120 units short each while launchpad capacity is free; one hauling pass restores the P4 chain.","inputs":{"p3_missing_each":120,"launchpad_capacity":1.0}}',
     1)
ON CONFLICT(id) DO NOTHING;

-- ---------- factory health metrics (mock, deterministic derivation in code) ----------

INSERT INTO factory_snapshot (metric, value, as_of)
VALUES
    ('factory_health',        0.87,          strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('idle_slots',            12,            strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('blocked_jobs',          0,             strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('isk_locked_in_jobs',    18700000000.0, strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    ('projected_finish_days', 12,            strftime('%Y-%m-%dT%H:%M:%SZ','now'))
ON CONFLICT(metric) DO NOTHING;

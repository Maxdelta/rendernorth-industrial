-- RenderNorth Industrial — migration 0003 (Sprint 003)
-- Inventory Engine foundation. Additive only; 0001 and 0002 are untouched.
--
-- Architecture this migration supports (see docs/architecture/):
--   Mission Control -> Operations -> Reservation Engine -> Inventory Engine -> SQLite
-- Operations never own inventory. They request reservations against it.
-- Inventory remains the single source of truth for every category of item.
--
-- This sprint builds the Inventory Engine and its schema only. The
-- Reservation Engine's tables exist here so the shape is right, but the
-- engine that creates/releases reservations is an interface stub — see
-- src-tauri/src/inventory/reservation.rs — wired into nothing yet.

-- ---------- controlled vocabulary: lifecycle states ----------
-- Not every state is exercised by logic yet (Manufacturing, Research,
-- Reaction, In Transit, Contract, Delivery, Destroyed are placeholders for
-- later sprints), but every inventory_items row must reference one so the
-- schema never has to bolt states on later.
CREATE TABLE IF NOT EXISTS inventory_states (
    key        TEXT PRIMARY KEY,
    label      TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);

INSERT INTO inventory_states (key, label, sort_order) VALUES
    ('available',    'Available',     1),
    ('reserved',     'Reserved',      2),
    ('allocated',    'Allocated',     3),
    ('manufacturing','Manufacturing', 4),
    ('research',     'Research',      5),
    ('reaction',     'Reaction',      6),
    ('in_transit',   'In Transit',    7),
    ('asset_safety', 'Asset Safety',  8),
    ('contract',     'Contract',      9),
    ('delivery',     'Delivery',      10),
    ('destroyed',    'Destroyed',     11)
ON CONFLICT(key) DO NOTHING;

-- ---------- controlled vocabulary: categories ----------
-- Categories are views over one inventory, never separate systems.
CREATE TABLE IF NOT EXISTS inventory_categories (
    key        TEXT PRIMARY KEY,
    label      TEXT NOT NULL,
    sort_order INTEGER NOT NULL
);

INSERT INTO inventory_categories (key, label, sort_order) VALUES
    ('ships',               'Ships',               1),
    ('blueprints',          'Blueprints',          2),
    ('minerals',            'Minerals',            3),
    ('ore',                 'Ore',                 4),
    ('compressed_ore',      'Compressed Ore',      5),
    ('ice',                 'Ice',                 6),
    ('ice_products',        'Ice Products',        7),
    ('pi',                  'PI',                  8),
    ('reaction_materials',  'Reaction Materials',  9),
    ('components',          'Components',          10),
    ('capital_components',  'Capital Components',  11),
    ('advanced_components', 'Advanced Components', 12),
    ('modules',             'Modules',             13),
    ('charges',             'Charges',             14),
    ('fuel',                'Fuel',                15),
    ('structures',          'Structures',          16),
    ('deployables',         'Deployables',         17)
ON CONFLICT(key) DO NOTHING;

-- ---------- locations ----------
-- `kind` is free text on purpose: station/structure/asset_safety/contract/pos
-- today; citadel/deep-space/deployable-anchor later. No schema change needed
-- to add a new kind, only a new row's worth of behavior downstream.
CREATE TABLE IF NOT EXISTS inventory_locations (
    location_id INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL,
    system_name TEXT,
    region_name TEXT
);

INSERT INTO inventory_locations (location_id, name, kind, system_name, region_name) VALUES
    (1,  'Jita IV - Moon 4 - Caldari Navy Assembly Plant',        'station',      'Jita',       'The Forge'),
    (2,  'Amarr VIII (Oris) - Emperor Family Academy',            'station',      'Amarr',      'Domain'),
    (3,  'Egghelende - RenderNorth Forge (Keepstar)',             'structure',    'Egghelende', 'Placid'),
    (4,  'Saatuban - Cyno Beacon Storage (Fortizar)',             'structure',    'Saatuban',   'The Citadel'),
    (5,  'Asset Safety - Jita',                                   'asset_safety', 'Jita',       'The Forge'),
    (6,  '1DQ1-A - Sotiyo (corp staging)',                        'structure',    '1DQ1-A',     'Delve'),
    (7,  'Rens VI - Moon 8 - Brutor Tribe Treasury',              'station',      'Rens',       'Heimatar'),
    (8,  'Dodixie IX - Moon 20 - Federation Navy Assembly Plant', 'station',      'Dodixie',    'Sinq Laison'),
    (9,  'Nakugard - Home POS Silo',                              'structure',    'Nakugard',   'Aridia'),
    (10, 'In transit - Red Frog Courier',                         'contract',     NULL,         NULL)
ON CONFLICT(location_id) DO NOTHING;

-- ---------- items: the single inventory truth ----------
-- Nothing here knows what a Titan is. `character_id` (owner), `corporation_id`
-- (future corp ownership), `container_item_id` (future containers, self-FK),
-- `contract_id`/`delivery_id` (future contracts/deliveries) are all present
-- and nullable so those features slot in without another migration touching
-- this table's shape.
CREATE TABLE IF NOT EXISTS inventory_items (
    item_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    type_name         TEXT NOT NULL,
    category_key      TEXT NOT NULL REFERENCES inventory_categories(key),
    quantity          INTEGER NOT NULL,
    location_id       INTEGER REFERENCES inventory_locations(location_id),
    character_id      INTEGER REFERENCES characters(character_id),
    corporation_id    INTEGER,
    container_item_id INTEGER REFERENCES inventory_items(item_id),
    contract_id       INTEGER,
    delivery_id       INTEGER,
    state             TEXT NOT NULL DEFAULT 'available' REFERENCES inventory_states(key),
    unit_value        REAL NOT NULL DEFAULT 0,
    source            TEXT NOT NULL DEFAULT 'demo',
    synced_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ---------- allocations: soft, plan-level earmarking ----------
-- "This item is part of Operation X's plan." Does not reduce availability by
-- itself — that is the Reservation Engine's job. Lets Production planning
-- (later sprints) tag intent before committing a hard hold.
CREATE TABLE IF NOT EXISTS inventory_allocations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id    INTEGER NOT NULL REFERENCES inventory_items(item_id),
    project_id INTEGER NOT NULL REFERENCES build_projects(project_id),
    quantity   INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ---------- reservations: hard holds against an operation ----------
-- "18 of these 48 Broadcast Nodes are reserved for Avatar." Reduces the
-- available quantity shown for an item while released_at is NULL. Rows exist
-- here as data; the engine that creates/releases them is an interface stub
-- this sprint (src-tauri/src/inventory/reservation.rs), not wired to any
-- command yet — Operations cannot request a reservation through the UI.
CREATE TABLE IF NOT EXISTS inventory_reservations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id     INTEGER NOT NULL REFERENCES inventory_items(item_id),
    project_id  INTEGER REFERENCES build_projects(project_id),
    quantity    INTEGER NOT NULL,
    reason      TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    released_at TEXT
);

-- ============================================================
-- Demo seed (flagged via source='demo'; purgeable in one statement
-- alongside the Sprint 001 character seed when real ESI sync lands)
-- ============================================================

INSERT INTO inventory_items
    (item_id, type_name, category_key, quantity, location_id, character_id, unit_value, source) VALUES
    -- ships
    (1,  'Providence',                       'ships',               1,        1, -1, 980000000,  'demo'),
    (2,  'Orca',                              'ships',               2,        6, -2, 1450000000, 'demo'),
    (3,  'Rorqual',                           'ships',               1,        6, -2, 12800000000,'demo'),
    -- blueprints
    (4,  'Avatar Blueprint Copy',             'blueprints',          1,        3, -1, 0,           'demo'),
    (5,  'Capital Construction Parts Blueprint', 'blueprints',       1,        3, -1, 3500000000,  'demo'),
    (6,  'Capital Armor Plates Blueprint',    'blueprints',          1,        3, -1, 4200000000,  'demo'),
    (7,  'Broadcast Node Blueprint Copy',     'blueprints',          3,        9, -2, 0,           'demo'),
    -- minerals
    (8,  'Tritanium',                         'minerals',   842000000,          1, -1, 4.2,        'demo'),
    (9,  'Pyerite',                           'minerals',   210000000,          1, -1, 9.8,        'demo'),
    (10, 'Mexallon',                          'minerals',    38500000,         1, -1, 65,          'demo'),
    (11, 'Isogen',                            'minerals',    12100000,         1, -1, 145,         'demo'),
    (12, 'Nocxium',                           'minerals',     1620000,         3, -1, 850,         'demo'),
    (13, 'Zydrine',                           'minerals',      410000,         3, -1, 1450,        'demo'),
    (14, 'Megacyte',                          'minerals',      288000,         3, -1, 2100,        'demo'),
    (15, 'Morphite',                          'minerals',       62000,         3, -1, 8200,        'demo'),
    -- ore
    (16, 'Bistot',                            'ore',           340000,         7, -2, 950,         'demo'),
    (17, 'Arkonor',                           'ore',           210000,         7, -2, 1150,        'demo'),
    (18, 'Mercoxit',                          'ore',            48000,         7, -2, 3200,        'demo'),
    -- compressed ore
    (19, 'Compressed Bistot',                 'compressed_ore', 12400,         7, -2, 9500,        'demo'),
    (20, 'Compressed Arkonor',                'compressed_ore',  8600,         7, -2, 11500,       'demo'),
    -- ice
    (21, 'Glacial Mass',                      'ice',            96000,         4, -2, 550,         'demo'),
    (22, 'Krystallos',                        'ice',            41000,         4, -2, 780,         'demo'),
    -- ice products
    (23, 'Heavy Water',                       'ice_products',  620000,         4, -2, 320,         'demo'),
    (24, 'Liquid Ozone',                      'ice_products',  480000,         4, -2, 410,         'demo'),
    (25, 'Helium Isotopes',                   'ice_products',  510000,         4, -2, 260,         'demo'),
    -- PI
    (26, 'Broadcast Node',                    'pi',                48,         9, -2, 1450000,     'demo'),
    (27, 'Ukomi Superconductors',             'pi',               280,         9, -2, 68000,       'demo'),
    (28, 'Condensates',                       'pi',               260,         9, -2, 71000,       'demo'),
    (29, 'High-Tech Transmitters',            'pi',               240,         9, -2, 69500,       'demo'),
    -- reaction materials
    (30, 'Mechanical Parts',                  'reaction_materials', 96000,     3, -1, 145,         'demo'),
    (31, 'Hypersynaptic Fibers',              'reaction_materials',  8200,     3, -1, 1650,        'demo'),
    -- components
    (32, 'Construction Blocks',               'components',      14200,        3, -1, 3200,        'demo'),
    (33, 'Nanite Compound',                   'components',       6100,        3, -1, 5100,        'demo'),
    -- capital components
    (34, 'Capital Construction Parts',        'capital_components',  210,      3, -1, 1180000,     'demo'),
    (35, 'Capital Armor Plates',               'capital_components',   68,     3, -1, 2450000,     'demo'),
    (36, 'Capital Capacitor Batteries',       'capital_components',    42,     3, -1, 2980000,      'demo'),
    (37, 'Capital Jump Drive',                'capital_components',    6,      3, -1, 8900000,      'demo'),
    -- advanced components
    (38, 'Auto-Integrity Preservation Seal',  'advanced_components',   57,     3, -1, 3650000,      'demo'),
    (39, 'Life Support Backup Unit',          'advanced_components',   34,     3, -1, 2100000,      'demo'),
    -- modules
    (40, 'Capital Shield Extender II',        'modules',                4,     6, -2, 42000000,     'demo'),
    (41, 'Large Armor Repairer II',           'modules',               18,     6, -2, 18500000,     'demo'),
    -- charges
    (42, 'Antimatter Charge L',               'charges',           240000,     6, -2, 480,          'demo'),
    (43, 'Void L',                            'charges',           180000,     6, -2, 520,          'demo'),
    -- fuel
    (44, 'Nitrogen Fuel Block',               'fuel',               96000,     9, -2, 1180,         'demo'),
    (45, 'Oxygen Isotopes',                   'fuel',              620000,     4, -2, 210,          'demo'),
    -- deployables
    (46, 'Mobile Depot',                      'deployables',           11,     6, -2, 1650000,      'demo'),
    (47, 'Mobile Tractor Unit',               'deployables',            7,     6, -2, 2950000,      'demo')
ON CONFLICT(item_id) DO NOTHING;

-- Soft plan-level allocations: earmark inventory against active operations
-- without reducing what shows as available. See build_projects (0001/0002).
INSERT INTO inventory_allocations (id, item_id, project_id, quantity) VALUES
    (1, 34, 1, 210),  -- Capital Construction Parts -> Avatar
    (2, 35, 1, 68),   -- Capital Armor Plates -> Avatar
    (3, 26, 1, 48),   -- Broadcast Node -> Avatar
    (4, 12, 2, 1620000), -- Nocxium -> Navy Revelation
    (5, 14, 3, 288000)   -- Megacyte -> Apostle
ON CONFLICT(id) DO NOTHING;

-- Hard reservations: reduce available quantity for a named operation. This
-- is the exact shape of the "18 of 48 Broadcast Nodes reserved for Avatar"
-- example — seeded as data, not created by any running engine yet.
INSERT INTO inventory_reservations (id, item_id, project_id, quantity, reason) VALUES
    (1, 26, 1, 18, 'Reserved for Avatar PI requirement'),
    (2, 34, 1, 90, 'Reserved for Avatar capital component requirement'),
    (3, 12, 2, 900000, 'Reserved for Navy Revelation mineral requirement')
ON CONFLICT(id) DO NOTHING;

CREATE TABLE doctrine_groups (
    doctrine_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT 'General',
    fleet_notes TEXT NOT NULL DEFAULT '',
    is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
    version INTEGER NOT NULL DEFAULT 1 CHECK(version > 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE doctrine_fits (
    fit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    doctrine_id INTEGER NOT NULL REFERENCES doctrine_groups(doctrine_id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    hull_type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    hull_name TEXT NOT NULL,
    desired_quantity INTEGER NOT NULL DEFAULT 1 CHECK(desired_quantity >= 0),
    original_eft_text TEXT NOT NULL,
    source_name TEXT NOT NULL DEFAULT 'EFT Text',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE doctrine_fit_items (
    fit_item_id INTEGER PRIMARY KEY AUTOINCREMENT,
    fit_id INTEGER NOT NULL REFERENCES doctrine_fits(fit_id) ON DELETE CASCADE,
    type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    item_name TEXT NOT NULL,
    item_kind TEXT NOT NULL CHECK(item_kind IN ('Module','Charge','Script','Rig','Subsystem','Drone','Cargo')),
    quantity INTEGER NOT NULL CHECK(quantity > 0)
);
CREATE INDEX idx_doctrine_fits_group ON doctrine_fits(doctrine_id);
CREATE INDEX idx_doctrine_fit_items_fit ON doctrine_fit_items(fit_id);

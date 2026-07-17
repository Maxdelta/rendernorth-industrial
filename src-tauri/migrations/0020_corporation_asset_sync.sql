-- REQ-001 — isolated corporation asset synchronization.
-- Corporation assets deliberately do not participate in personal/manual totals.

CREATE TABLE corporations (
    corporation_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    authorizing_character_id INTEGER REFERENCES characters(character_id) ON DELETE SET NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

CREATE TABLE corporation_asset_sync_state (
    corporation_id INTEGER PRIMARY KEY REFERENCES corporations(corporation_id) ON DELETE CASCADE,
    authorizing_character_id INTEGER REFERENCES characters(character_id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'never',
    required_role TEXT NOT NULL DEFAULT 'Director',
    role_verified INTEGER NOT NULL DEFAULT 0 CHECK(role_verified IN (0,1)),
    last_attempt_at TEXT,
    last_success_at TEXT,
    asset_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE corporation_character_sync_state (
    character_id INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    corporation_id INTEGER,
    corporation_name TEXT,
    status TEXT NOT NULL DEFAULT 'never',
    required_role TEXT NOT NULL DEFAULT 'Director',
    role_verified INTEGER NOT NULL DEFAULT 0 CHECK(role_verified IN (0,1)),
    last_attempt_at TEXT,
    last_success_at TEXT,
    asset_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE corporation_divisions (
    corporation_id INTEGER NOT NULL REFERENCES corporations(corporation_id) ON DELETE CASCADE,
    division_number INTEGER NOT NULL CHECK(division_number BETWEEN 1 AND 7),
    name TEXT NOT NULL,
    synced_at TEXT NOT NULL,
    PRIMARY KEY(corporation_id, division_number)
);

CREATE TABLE corporation_assets (
    corporation_id INTEGER NOT NULL REFERENCES corporations(corporation_id) ON DELETE CASCADE,
    item_id INTEGER NOT NULL,
    type_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL,
    location_id INTEGER NOT NULL,
    location_type TEXT NOT NULL,
    location_flag TEXT NOT NULL,
    is_singleton INTEGER NOT NULL CHECK(is_singleton IN (0,1)),
    division_number INTEGER,
    division_name TEXT,
    synced_at TEXT NOT NULL,
    PRIMARY KEY(corporation_id, item_id)
);

CREATE INDEX idx_corporation_assets_type ON corporation_assets(type_id);
CREATE INDEX idx_corporation_assets_location ON corporation_assets(location_id);
CREATE INDEX idx_corporation_assets_owner_location ON corporation_assets(corporation_id, location_id);

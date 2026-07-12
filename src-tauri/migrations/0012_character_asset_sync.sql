-- RenderNorth Industrial — migration 0012
-- Sprint 011B: Character Asset Synchronization.

CREATE TABLE character_asset_sync_state (
    character_id     INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    status           TEXT NOT NULL DEFAULT 'never',
    last_attempt_at  TEXT,
    last_success_at  TEXT,
    asset_count      INTEGER NOT NULL DEFAULT 0,
    page_count       INTEGER NOT NULL DEFAULT 0,
    last_error       TEXT
);

CREATE TABLE character_assets (
    character_id   INTEGER NOT NULL REFERENCES characters(character_id) ON DELETE CASCADE,
    item_id        INTEGER NOT NULL,
    type_id        INTEGER NOT NULL,
    quantity       INTEGER NOT NULL,
    location_id    INTEGER NOT NULL,
    location_type  TEXT NOT NULL,
    location_flag  TEXT NOT NULL,
    is_singleton   INTEGER NOT NULL,
    synced_at      TEXT NOT NULL,
    PRIMARY KEY (character_id, item_id)
);

CREATE INDEX idx_character_assets_type ON character_assets(type_id);
CREATE INDEX idx_character_assets_location ON character_assets(character_id, location_id);

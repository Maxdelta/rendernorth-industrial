-- RenderNorth Industrial — migration 0013
-- RNI-150: read-only character blueprint snapshots.

CREATE TABLE character_blueprint_sync_state (
    character_id       INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    status             TEXT NOT NULL DEFAULT 'never',
    last_attempt_at    TEXT,
    last_success_at    TEXT,
    blueprint_count    INTEGER NOT NULL DEFAULT 0,
    page_count         INTEGER NOT NULL DEFAULT 0,
    last_error         TEXT
);

CREATE TABLE character_blueprints (
    character_id         INTEGER NOT NULL REFERENCES characters(character_id) ON DELETE CASCADE,
    item_id              INTEGER NOT NULL,
    type_id              INTEGER NOT NULL,
    location_id          INTEGER NOT NULL,
    location_flag        TEXT NOT NULL,
    quantity             INTEGER NOT NULL,
    material_efficiency  INTEGER NOT NULL,
    time_efficiency      INTEGER NOT NULL,
    runs                 INTEGER NOT NULL,
    source               TEXT NOT NULL DEFAULT 'ESI Character Blueprints',
    synced_at            TEXT NOT NULL,
    PRIMARY KEY (character_id, item_id)
);

CREATE INDEX idx_character_blueprints_type ON character_blueprints(type_id);
CREATE INDEX idx_character_blueprints_location ON character_blueprints(character_id, location_id);

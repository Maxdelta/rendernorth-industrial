CREATE TABLE location_cache (
    location_id INTEGER PRIMARY KEY,
    location_kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    solar_system_id INTEGER,
    solar_system_name TEXT,
    constellation_id INTEGER,
    constellation_name TEXT,
    region_id INTEGER,
    region_name TEXT,
    resolution_status TEXT NOT NULL,
    resolution_source TEXT NOT NULL,
    resolved_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    last_error TEXT
);
CREATE INDEX idx_location_cache_system ON location_cache(solar_system_id);
CREATE INDEX idx_location_cache_region ON location_cache(region_id);

-- RNI-161 — local, read-only character market-order history.
-- CCP history states remain authoritative; local seen state never writes to ESI.

CREATE TABLE character_market_order_history_sync_state (
    character_id INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'never',
    last_attempt_at TEXT,
    last_success_at TEXT,
    history_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE character_market_order_history (
    character_id INTEGER NOT NULL REFERENCES characters(character_id) ON DELETE CASCADE,
    order_id INTEGER NOT NULL,
    type_id INTEGER NOT NULL,
    is_buy_order INTEGER NOT NULL CHECK(is_buy_order IN (0,1)),
    is_corporation INTEGER NOT NULL CHECK(is_corporation IN (0,1)),
    location_id INTEGER NOT NULL,
    region_id INTEGER NOT NULL,
    price_isk TEXT NOT NULL,
    volume_total INTEGER NOT NULL,
    volume_remain INTEGER NOT NULL,
    min_volume INTEGER,
    issued_at TEXT NOT NULL,
    duration_days INTEGER NOT NULL,
    order_range TEXT NOT NULL,
    escrow_isk TEXT,
    esi_state TEXT NOT NULL CHECK(esi_state IN ('cancelled','expired')),
    first_seen_at TEXT NOT NULL,
    last_observed_at TEXT NOT NULL,
    seen_at TEXT,
    source TEXT NOT NULL DEFAULT 'ESI Character Market Order History',
    PRIMARY KEY(character_id, order_id)
);

CREATE INDEX idx_character_market_order_history_seen
    ON character_market_order_history(seen_at,first_seen_at);
CREATE INDEX idx_character_market_order_history_character_state
    ON character_market_order_history(character_id,esi_state);
CREATE INDEX idx_character_market_order_history_type
    ON character_market_order_history(type_id);
CREATE INDEX idx_character_market_order_history_location
    ON character_market_order_history(location_id);

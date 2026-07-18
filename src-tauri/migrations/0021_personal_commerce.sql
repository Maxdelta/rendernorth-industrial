-- RNI-159 — read-only personal Commerce snapshots.
-- Market orders and contracts are isolated by feature and character.

CREATE TABLE character_market_order_sync_state (
    character_id INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'never',
    last_attempt_at TEXT,
    last_success_at TEXT,
    order_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    etag TEXT,
    last_modified TEXT,
    last_error TEXT
);

CREATE TABLE character_market_orders (
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
    source TEXT NOT NULL DEFAULT 'ESI Character Market Orders',
    synced_at TEXT NOT NULL,
    PRIMARY KEY(character_id, order_id)
);
CREATE INDEX idx_character_market_orders_type ON character_market_orders(type_id);
CREATE INDEX idx_character_market_orders_location ON character_market_orders(location_id);
CREATE INDEX idx_character_market_orders_character_side ON character_market_orders(character_id,is_buy_order);

CREATE TABLE character_contract_sync_state (
    character_id INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'never',
    last_attempt_at TEXT,
    last_success_at TEXT,
    contract_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE character_contracts (
    character_id INTEGER NOT NULL REFERENCES characters(character_id) ON DELETE CASCADE,
    contract_id INTEGER NOT NULL,
    issuer_id INTEGER NOT NULL,
    issuer_corporation_id INTEGER NOT NULL,
    assignee_id INTEGER NOT NULL,
    acceptor_id INTEGER NOT NULL,
    contract_type TEXT NOT NULL,
    availability TEXT NOT NULL,
    status TEXT NOT NULL,
    title TEXT,
    date_issued TEXT NOT NULL,
    date_expired TEXT NOT NULL,
    date_accepted TEXT,
    date_completed TEXT,
    start_location_id INTEGER,
    end_location_id INTEGER,
    price_isk TEXT,
    reward_isk TEXT,
    collateral_isk TEXT,
    buyout_isk TEXT,
    volume_m3 TEXT,
    days_to_complete INTEGER,
    for_corporation INTEGER NOT NULL CHECK(for_corporation IN (0,1)),
    source TEXT NOT NULL DEFAULT 'ESI Character Contracts',
    synced_at TEXT NOT NULL,
    PRIMARY KEY(character_id, contract_id)
);
CREATE INDEX idx_character_contracts_status ON character_contracts(character_id,status);
CREATE INDEX idx_character_contracts_locations ON character_contracts(start_location_id,end_location_id);

CREATE TABLE character_contract_detail_state (
    character_id INTEGER NOT NULL,
    contract_id INTEGER NOT NULL,
    items_status TEXT NOT NULL DEFAULT 'never',
    items_loaded_at TEXT,
    items_error TEXT,
    bids_status TEXT NOT NULL DEFAULT 'not_applicable',
    bids_loaded_at TEXT,
    bids_error TEXT,
    PRIMARY KEY(character_id, contract_id),
    FOREIGN KEY(character_id,contract_id) REFERENCES character_contracts(character_id,contract_id) ON DELETE CASCADE
);

CREATE TABLE character_contract_items (
    character_id INTEGER NOT NULL,
    contract_id INTEGER NOT NULL,
    record_id INTEGER NOT NULL,
    type_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL,
    raw_quantity INTEGER,
    is_singleton INTEGER NOT NULL CHECK(is_singleton IN (0,1)),
    is_included INTEGER NOT NULL CHECK(is_included IN (0,1)),
    synced_at TEXT NOT NULL,
    PRIMARY KEY(character_id,contract_id,record_id),
    FOREIGN KEY(character_id,contract_id) REFERENCES character_contracts(character_id,contract_id) ON DELETE CASCADE
);

CREATE TABLE character_contract_bids (
    character_id INTEGER NOT NULL,
    contract_id INTEGER NOT NULL,
    bid_id INTEGER NOT NULL,
    bidder_id INTEGER NOT NULL,
    amount_isk TEXT NOT NULL,
    date_bid TEXT NOT NULL,
    synced_at TEXT NOT NULL,
    PRIMARY KEY(character_id,contract_id,bid_id),
    FOREIGN KEY(character_id,contract_id) REFERENCES character_contracts(character_id,contract_id) ON DELETE CASCADE
);

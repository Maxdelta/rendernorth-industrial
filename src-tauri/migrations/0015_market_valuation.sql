CREATE TABLE market_profiles (
    profile_id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL,
    region_id INTEGER NOT NULL,
    location_id INTEGER NOT NULL,
    refresh_interval_seconds INTEGER NOT NULL DEFAULT 300,
    is_selected INTEGER NOT NULL DEFAULT 0 CHECK(is_selected IN (0,1))
);
INSERT INTO market_profiles(profile_id,display_name,region_id,location_id,refresh_interval_seconds,is_selected)
VALUES(1,'Jita 4-4',10000002,60003760,300,1);

CREATE TABLE market_order_cache (
    profile_id INTEGER NOT NULL REFERENCES market_profiles(profile_id) ON DELETE CASCADE,
    order_id INTEGER NOT NULL,
    type_id INTEGER NOT NULL,
    is_buy_order INTEGER NOT NULL CHECK(is_buy_order IN (0,1)),
    price REAL NOT NULL,
    volume_remain INTEGER NOT NULL,
    min_volume INTEGER NOT NULL DEFAULT 1,
    order_range TEXT NOT NULL,
    location_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    fetched_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'CCP ESI Market Orders',
    PRIMARY KEY(profile_id,order_id)
);
CREATE INDEX idx_market_orders_quote ON market_order_cache(profile_id,type_id,is_buy_order,price);

CREATE TABLE market_refresh_state (
    profile_id INTEGER PRIMARY KEY REFERENCES market_profiles(profile_id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'never',
    fetched_at TEXT,
    expires_at TEXT,
    order_count INTEGER NOT NULL DEFAULT 0,
    page_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

CREATE TABLE operation_cost_assumptions (
    operation_id INTEGER PRIMARY KEY REFERENCES operations(operation_id) ON DELETE CASCADE,
    sales_tax_percent REAL NOT NULL DEFAULT 0,
    broker_fee_percent REAL NOT NULL DEFAULT 0,
    manufacturing_job_cost REAL NOT NULL DEFAULT 0,
    hauling_cost REAL NOT NULL DEFAULT 0,
    other_cost REAL NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

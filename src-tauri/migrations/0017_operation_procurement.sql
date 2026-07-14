-- Mutable, operation-specific procurement workflow state. Authoritative
-- quantities remain derived from the live production plan and inventory.
CREATE TABLE operation_procurement_lines (
    operation_id INTEGER NOT NULL REFERENCES operations(operation_id) ON DELETE CASCADE,
    type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    status TEXT NOT NULL DEFAULT 'Needed'
        CHECK(status IN ('Needed','Planned','Purchased','Skipped','Fulfilled Manually')),
    notes TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY(operation_id, type_id)
);

CREATE TABLE operation_procurement_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id INTEGER NOT NULL REFERENCES operations(operation_id) ON DELETE CASCADE,
    type_id INTEGER NOT NULL REFERENCES eve_types(type_id),
    previous_status TEXT,
    new_status TEXT,
    notes TEXT NOT NULL DEFAULT '',
    event_type TEXT NOT NULL CHECK(event_type IN ('updated','reset')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_procurement_events_operation
    ON operation_procurement_events(operation_id, created_at);

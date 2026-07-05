-- RenderNorth Industrial — migration 0005 (Sprint 005)
-- Reservation Engine Foundation. Additive only; 0001–0004 are untouched.
--
-- The Reservation Engine sits between the Operation Engine and the
-- Inventory Engine:
--
--   Inventory Engine -> Reservation Engine -> Operation Engine -> Decision Engine
--
-- Inventory Engine owns what exists. Reservation Engine owns who owns
-- inventory, reservation quantities, reservation history, and reservation
-- conflicts. Operation Engine owns work and references reservations but
-- never touches inventory tables directly. Decision Engine remains
-- interface-only.
--
-- `inventory_reservations` already exists (migration 0003). It is not
-- recreated here — only extended, additively, with an `operation_id`
-- column pointing at the now-real `operations` table (migration 0004).
-- The pre-existing `project_id` column is left in place for backward
-- compatibility with anything still reading it; `operation_id` is the
-- column the new Reservation Engine reads and writes going forward.

ALTER TABLE inventory_reservations ADD COLUMN operation_id INTEGER REFERENCES operations(operation_id);

-- Backfill: operations 1–5 share their id space with build_projects (see
-- migration 0004's note), so every existing reservation's project_id is
-- also a valid operation_id.
UPDATE inventory_reservations SET operation_id = project_id WHERE operation_id IS NULL AND project_id IS NOT NULL;

-- ---------- reservation event history ----------
-- Append-only log. History only — nothing writes to this table
-- automatically; every row here is either a seeded demo event or, once the
-- Reservation Engine's mutations are implemented (a later sprint), a
-- record of something a person or a calling engine explicitly did.
CREATE TABLE IF NOT EXISTS reservation_events (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    reservation_id     INTEGER NOT NULL REFERENCES inventory_reservations(id),
    event_type         TEXT NOT NULL, -- reserved / released / transferred / expired
    quantity           INTEGER NOT NULL,
    from_operation_id  INTEGER REFERENCES operations(operation_id),
    to_operation_id    INTEGER REFERENCES operations(operation_id),
    reason             TEXT NOT NULL,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ---------- reservation conflicts ----------
-- Detection only — nothing here resolves a conflict. Rows are illustrative
-- snapshots for this sprint; the Reservation Engine's live `conflicts()`
-- query recomputes conflicts fresh from current reservation state on every
-- read (same "derive, don't trust a stale flag" precedent as
-- Operation Engine's `is_blocked` and Inventory Engine's
-- `coverage_for_operation`). This table exists so a future scan can persist
-- history of what was detected and when, without changing the read shape.
CREATE TABLE IF NOT EXISTS reservation_conflicts (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id       INTEGER NOT NULL REFERENCES inventory_items(item_id),
    conflict_type TEXT NOT NULL, -- overlapping_operations / exceeds_stock / missing_inventory
    description   TEXT NOT NULL,
    detected_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- ============================================================
-- Demo seed — extends the existing reservation scenario from migration
-- 0003 with two new reservations that deliberately produce one of each of
-- the two conflict types the live query can detect from real data
-- (overlapping operations, exceeds stock). The third type — a reservation
-- against inventory that no longer exists at sufficient quantity — is
-- implemented and queried for, but nothing in this seed forces it; an
-- empty result for that type is itself an honest demo state.
-- ============================================================

-- Item 34 (Capital Construction Parts, 210 on hand) already has 90 units
-- held by operation 1 (Avatar). This reservation gives operation 6
-- ("Prepare Titan Components") a second, independent hold on the same
-- item — two distinct operations reserving the same inventory.
INSERT INTO inventory_reservations (id, item_id, project_id, operation_id, quantity, reason) VALUES
    (4, 34, NULL, 6, 50, 'Shared Capital Construction Parts stockpile for Titan component staging')
ON CONFLICT(id) DO NOTHING;

-- Item 38 (Auto-Integrity Preservation Seal, 57 on hand) reserved past its
-- available stock — a reservation that exceeds what's actually on hand.
INSERT INTO inventory_reservations (id, item_id, project_id, operation_id, quantity, reason) VALUES
    (5, 38, NULL, 3, 70, 'Apostle advanced component reservation exceeds current stock — flagged for review')
ON CONFLICT(id) DO NOTHING;

-- A closed reservation with a full reserved -> released history, to
-- demonstrate the "released" event type on something that isn't currently
-- active.
INSERT INTO inventory_reservations (id, item_id, project_id, operation_id, quantity, reason, created_at, released_at) VALUES
    (6, 44, NULL, 2, 5000, 'Temporary Navy Revelation fuel staging (returned to general stock)', '2026-05-01T00:00:00Z', '2026-05-14T00:00:00Z')
ON CONFLICT(id) DO NOTHING;

-- A closed reservation that lapsed rather than being explicitly released,
-- to demonstrate the "expired" event type.
INSERT INTO inventory_reservations (id, item_id, project_id, operation_id, quantity, reason, created_at, released_at) VALUES
    (7, 27, NULL, 5, 40, 'Broadcast Node PI staging hold (expired without confirmation)', '2026-06-01T00:00:00Z', '2026-06-08T00:00:00Z')
ON CONFLICT(id) DO NOTHING;

INSERT INTO reservation_events (id, reservation_id, event_type, quantity, from_operation_id, to_operation_id, reason, created_at) VALUES
    (1, 1, 'reserved',    18,    NULL, 1,    'Reserved for Avatar PI requirement', '2026-04-20T00:00:00Z'),
    (2, 2, 'reserved',    90,    NULL, 1,    'Reserved for Avatar capital component requirement', '2026-04-22T00:00:00Z'),
    (3, 3, 'reserved',    900000,NULL, 2,    'Reserved for Navy Revelation mineral requirement', '2026-04-25T00:00:00Z'),
    (4, 4, 'reserved',    50,    NULL, 6,    'Shared Capital Construction Parts stockpile for Titan component staging', '2026-06-20T00:00:00Z'),
    (5, 5, 'reserved',    70,    NULL, 3,    'Apostle advanced component reservation exceeds current stock', '2026-06-21T00:00:00Z'),
    (6, 6, 'reserved',    5000,  NULL, 2,    'Temporary Navy Revelation fuel staging', '2026-05-01T00:00:00Z'),
    (7, 6, 'released',    5000,  2,    NULL, 'Fuel staging complete; unused portion returned to general stock', '2026-05-14T00:00:00Z'),
    (8, 2, 'transferred', 90,    4,    1,    'Capital Construction Parts output reassigned directly to Avatar hull assembly', '2026-04-23T00:00:00Z'),
    (9, 7, 'reserved',    40,    NULL, 5,    'Broadcast Node PI staging hold', '2026-06-01T00:00:00Z'),
    (10, 7, 'expired',    40,    5,    NULL, 'Hold window elapsed without confirmation', '2026-06-08T00:00:00Z')
ON CONFLICT(id) DO NOTHING;

INSERT INTO reservation_conflicts (id, item_id, conflict_type, description, detected_at) VALUES
    (1, 34, 'overlapping_operations', 'Capital Construction Parts held by both Avatar (90) and Prepare Titan Components (50) — 2 operations, 140 of 210 on hand', '2026-06-20T00:00:00Z'),
    (2, 38, 'exceeds_stock', 'Auto-Integrity Preservation Seal reserved 70 against 57 on hand — 13 units over-committed', '2026-06-21T00:00:00Z')
ON CONFLICT(id) DO NOTHING;

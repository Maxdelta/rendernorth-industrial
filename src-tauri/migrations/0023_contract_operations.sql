-- RNI-162 — durable local contract activity state.
-- CCP contract records remain authoritative; read/seen state is local-only.

ALTER TABLE character_contracts ADD COLUMN first_observed_at TEXT;
ALTER TABLE character_contracts ADD COLUMN last_observed_at TEXT;
ALTER TABLE character_contracts ADD COLUMN seen_at TEXT;

UPDATE character_contracts
SET first_observed_at = synced_at,
    last_observed_at = synced_at
WHERE first_observed_at IS NULL OR last_observed_at IS NULL;

CREATE INDEX idx_character_contracts_activity_seen
    ON character_contracts(seen_at, first_observed_at);
CREATE INDEX idx_character_contracts_activity_status
    ON character_contracts(status, date_completed, date_expired);

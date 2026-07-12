-- RenderNorth Industrial — migration 0011
-- Sprint 011A: EVE SSO Character Authentication Foundation.
-- Additive only; 0001–0010 are untouched.
--
-- Scope deliberately narrow: this migration adds only what character
-- authentication itself needs. No asset sync, no universe map, no
-- inventory scope, no operation integration — those are explicitly
-- out of scope for this sprint and belong to a later one.
--
-- SECURITY NOTE: access/refresh tokens are NEVER stored in this
-- database. They live in the OS keychain (Windows Credential Manager
-- via the `keyring` crate), keyed by character_id. Every column added
-- to `characters` below is non-secret bookkeeping — safe to read from
-- the frontend, safe to log, safe to back up. If you're looking for
-- where a token is stored, it isn't in this file.

ALTER TABLE characters ADD COLUMN scopes_granted TEXT NOT NULL DEFAULT '';
ALTER TABLE characters ADD COLUMN enabled INTEGER NOT NULL DEFAULT 1;
-- 'authorized' | 'expired' | 'revoked'
ALTER TABLE characters ADD COLUMN authorization_status TEXT NOT NULL DEFAULT 'authorized';
ALTER TABLE characters ADD COLUMN token_expires_at TEXT;
ALTER TABLE characters ADD COLUMN last_login_at TEXT;

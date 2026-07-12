//! All SQL for connected characters. Sprint 011A scope: authentication
//! bookkeeping only — no asset tables exist yet, so there is nothing
//! here resembling a sync or snapshot operation.

use super::models::{CharacterSummary, NewCharacter};
use rusqlite::Connection;

pub struct CharacterRepository<'a> {
    conn: &'a Connection,
}

impl<'a> CharacterRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Adding a character that's already connected re-authorizes it
    /// (fresh scopes/expiry, status reset to "authorized", last_login_at
    /// bumped to now) rather than erroring — re-running Add Character
    /// for an existing character is exactly how you'd recover from a
    /// revoked/expired grant.
    pub fn add_character(&self, new: &NewCharacter) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO characters (character_id, name, is_demo, scopes_granted, enabled, authorization_status, token_expires_at, last_login_at)
                 VALUES (?1, ?2, 0, ?3, 1, 'authorized', ?4, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
                 ON CONFLICT(character_id) DO UPDATE SET
                     name = excluded.name,
                     scopes_granted = excluded.scopes_granted,
                     authorization_status = 'authorized',
                     token_expires_at = excluded.token_expires_at,
                     last_login_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')",
                (new.character_id, &new.name, new.scopes_granted.join(" "), &new.token_expires_at),
            )
            .map_err(|e| format!("failed to add character {}: {e}", new.character_id))?;
        Ok(())
    }

    /// Deletes the character row. Does NOT touch the OS keychain
    /// credential — that's the engine's job, since this repository is
    /// SQL-only by convention (see every other `*::repository` in this
    /// codebase).
    pub fn remove_character(&self, character_id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM characters WHERE character_id = ?1 AND is_demo = 0", [character_id])
            .map_err(|e| format!("failed to delete character {character_id}: {e}"))?;
        Ok(())
    }

    pub fn set_enabled(&self, character_id: i64, enabled: bool) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE characters SET enabled = ?2 WHERE character_id = ?1 AND is_demo = 0",
                (character_id, enabled as i64),
            )
            .map_err(|e| format!("failed to update character {character_id}: {e}"))?;
        Ok(())
    }

    /// Real characters only — is_demo = 0 always, same discipline as
    /// every other list query since Sprint 010.
    pub fn list_characters(&self) -> Result<Vec<CharacterSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT c.character_id, c.name, c.enabled, c.authorization_status, c.last_login_at,
                        CASE WHEN instr(' ' || c.scopes_granted || ' ', ' esi-assets.read_assets.v1 ') > 0 THEN 1 ELSE 0 END,
                        COALESCE(s.status, 'never'), s.last_success_at,
                        COALESCE(s.asset_count, 0), COALESCE(s.page_count, 0), s.last_error
                 FROM characters c
                 LEFT JOIN character_asset_sync_state s ON s.character_id = c.character_id
                 WHERE c.is_demo = 0 ORDER BY c.name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CharacterSummary {
                    character_id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    authorization_status: row.get(3)?,
                    last_login_at: row.get(4)?,
                    asset_scope_granted: row.get::<_, i64>(5)? != 0,
                    sync_status: row.get(6)?,
                    last_sync_at: row.get(7)?,
                    asset_count: row.get(8)?,
                    page_count: row.get(9)?,
                    sync_error: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIGRATIONS: &[&str] = &[
        include_str!("../../migrations/0001_init.sql"),
        include_str!("../../migrations/0002_build_targets.sql"),
        include_str!("../../migrations/0003_inventory_foundation.sql"),
        include_str!("../../migrations/0004_operation_foundation.sql"),
        include_str!("../../migrations/0005_reservation_foundation.sql"),
        include_str!("../../migrations/0006_blueprint_foundation.sql"),
        include_str!("../../migrations/0007_production_requirement_foundation.sql"),
        include_str!("../../migrations/0008_real_production_planner.sql"),
        include_str!("../../migrations/0009_inventory_scope.sql"),
        include_str!("../../migrations/0010_demo_category_correction.sql"),
        include_str!("../../migrations/0011_esi_character_auth.sql"),
        include_str!("../../migrations/0012_character_asset_sync.sql"),
    ];

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for m in MIGRATIONS {
            conn.execute_batch(m).expect("apply migration");
        }
        conn
    }

    fn sample_character(character_id: i64, name: &str) -> NewCharacter {
        NewCharacter {
            character_id,
            name: name.to_string(),
            scopes_granted: vec![],
            token_expires_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn add_character_then_list_shows_it_enabled_and_authorized() {
        let conn = test_db();
        let repo = CharacterRepository::new(&conn);
        repo.add_character(&sample_character(2112073677, "Test Pilot")).unwrap();

        let list = repo.list_characters().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].character_id, 2112073677);
        assert_eq!(list[0].name, "Test Pilot");
        assert!(list[0].enabled, "a newly added character must be enabled by default");
        assert_eq!(list[0].authorization_status, "authorized");
        assert!(list[0].last_login_at.is_some(), "last_login_at must be set on add");
    }

    #[test]
    fn re_adding_an_existing_character_reauthorizes_rather_than_erroring() {
        let conn = test_db();
        let repo = CharacterRepository::new(&conn);
        repo.add_character(&sample_character(2112073677, "Test Pilot")).unwrap();
        conn.execute("UPDATE characters SET authorization_status = 'revoked' WHERE character_id = 2112073677", []).unwrap();

        repo.add_character(&sample_character(2112073677, "Test Pilot")).expect("re-adding must succeed, not error");

        let list = repo.list_characters().unwrap();
        assert_eq!(list.len(), 1, "must not create a duplicate row");
        assert_eq!(list[0].authorization_status, "authorized", "re-adding must reset a revoked status");
    }

    #[test]
    fn set_enabled_toggles_correctly() {
        let conn = test_db();
        let repo = CharacterRepository::new(&conn);
        repo.add_character(&sample_character(2112073677, "Test Pilot")).unwrap();

        repo.set_enabled(2112073677, false).unwrap();
        assert!(!repo.list_characters().unwrap()[0].enabled);

        repo.set_enabled(2112073677, true).unwrap();
        assert!(repo.list_characters().unwrap()[0].enabled);
    }

    #[test]
    fn remove_character_deletes_it() {
        let conn = test_db();
        let repo = CharacterRepository::new(&conn);
        repo.add_character(&sample_character(2112073677, "Test Pilot")).unwrap();
        assert_eq!(repo.list_characters().unwrap().len(), 1);

        repo.remove_character(2112073677).unwrap();
        assert_eq!(repo.list_characters().unwrap().len(), 0);
    }

    #[test]
    fn list_characters_never_returns_demo_rows() {
        let conn = test_db();
        // Explicit fixture, not relying on any migration-seeded demo
        // row — same "don't lean on incidental migration content"
        // discipline as the rest of this project since Sprint 010.
        conn.execute(
            "INSERT INTO characters (character_id, name, is_demo, enabled, authorization_status)
             VALUES (999, 'Demo Character', 1, 1, 'authorized')",
            [],
        )
        .unwrap();
        let repo = CharacterRepository::new(&conn);
        repo.add_character(&sample_character(2112073677, "Real Pilot")).unwrap();

        let list = repo.list_characters().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].character_id, 2112073677);
    }

    #[test]
    fn remove_character_never_deletes_a_demo_row() {
        let conn = test_db();
        conn.execute(
            "INSERT INTO characters (character_id, name, is_demo, enabled, authorization_status)
             VALUES (999, 'Demo Character', 1, 1, 'authorized')",
            [],
        )
        .unwrap();
        let repo = CharacterRepository::new(&conn);
        repo.remove_character(999).unwrap();

        let still_there: i64 = conn.query_row("SELECT COUNT(*) FROM characters WHERE character_id = 999", [], |r| r.get(0)).unwrap();
        assert_eq!(still_there, 1, "a demo character must never be deletable through this path");
    }
}

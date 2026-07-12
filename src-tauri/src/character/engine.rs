//! Orchestrates character CRUD (repository + OS keychain together).
//! Sprint 011A scope: no asset sync exists, so there is nothing here
//! resembling the prepare/fetch/commit split a sync feature would need.

use super::models::{CharacterSummary, NewCharacter};
use super::repository::CharacterRepository;
use crate::esi;
use rusqlite::Connection;

/// Blocking — runs the full OAuth flow (opens the browser, waits on the
/// loopback listener). The Tauri command calling this is a non-async
/// command function, which Tauri dispatches off its main thread
/// automatically.
pub fn add_character(conn: &Connection, client_id: &str) -> Result<CharacterSummary, String> {
    let result = esi::auth::run_add_character_flow(client_id)?;

    esi::token_store::store_refresh_token(result.identity.character_id, &result.refresh_token)?;

    let repo = CharacterRepository::new(conn);
    let expires_at = chrono::DateTime::from_timestamp(result.identity.expires_at_unix, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();
    repo.add_character(&NewCharacter {
        character_id: result.identity.character_id,
        name: result.identity.character_name,
        scopes_granted: result.identity.scopes,
        token_expires_at: expires_at,
    })?;

    let summary = repo
        .list_characters()?
        .into_iter()
        .find(|c| c.character_id == result.identity.character_id)
        .ok_or_else(|| "character was added but could not be read back".to_string())?;
    Ok(summary)
}

pub fn remove_character(conn: &Connection, character_id: i64) -> Result<(), String> {
    CharacterRepository::new(conn).remove_character(character_id)?;
    // Best-effort: the character row is already gone (the part that
    // actually matters); a failure to clear the keychain entry is not
    // fatal to the removal itself.
    let _ = esi::token_store::delete_refresh_token(character_id);
    Ok(())
}

pub fn set_enabled(conn: &Connection, character_id: i64, enabled: bool) -> Result<(), String> {
    CharacterRepository::new(conn).set_enabled(character_id, enabled)
}

pub fn list_characters(conn: &Connection) -> Result<Vec<CharacterSummary>, String> {
    CharacterRepository::new(conn).list_characters()
}

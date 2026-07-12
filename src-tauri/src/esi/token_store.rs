//! Refresh tokens live here, and nowhere else — never in SQLite, never
//! sent to the frontend, never logged. Backed by the OS credential store
//! via the `keyring` crate (Windows Credential Manager on the platform
//! this app ships for).
//!
//! HONEST LIMITATION: this sandbox has no OS keychain daemon, so none of
//! this has been exercised here — `keyring`'s own crate tests assume a
//! real OS credential store exists, which this environment doesn't
//! provide. This is straightforward, well-trodden use of a mature crate
//! (store/retrieve/delete by key), but the actual round-trip needs your
//! own verification on Windows — see the sprint's verification steps.

const SERVICE_NAME: &str = "RenderNorth Industrial";

fn entry_for(character_id: i64) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE_NAME, &character_id.to_string())
        .map_err(|e| format!("failed to open keychain entry for character {character_id}: {e}"))
}

pub fn store_refresh_token(character_id: i64, refresh_token: &str) -> Result<(), String> {
    entry_for(character_id)?
        .set_password(refresh_token)
        .map_err(|e| format!("failed to store refresh token for character {character_id}: {e}"))
}

pub fn get_refresh_token(character_id: i64) -> Result<String, String> {
    entry_for(character_id)?
        .get_password()
        .map_err(|e| format!("failed to retrieve refresh token for character {character_id}: {e}"))
}

/// Called when a character is removed — deletes its credential from the
/// OS store. Not finding one is not an error (it may already be gone,
/// or the character was added in a state where storage failed).
pub fn delete_refresh_token(character_id: i64) -> Result<(), String> {
    match entry_for(character_id)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("failed to delete stored credential for character {character_id}: {e}")),
    }
}

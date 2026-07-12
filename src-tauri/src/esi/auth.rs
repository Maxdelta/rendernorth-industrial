//! Orchestrates the full "Add Character" flow end to end: generate PKCE
//! + state, open the system browser, wait on the loopback listener,
//! exchange the code, validate the token, return the character's
//! identity and tokens for the caller to persist.
//!
//! This is the one module in `esi::` that composes everything else —
//! it has no logic of its own beyond sequencing, which is deliberate:
//! every piece it calls (pkce, loopback, client, jwt) is independently
//! unit-tested; this module's own correctness is "does it call them in
//! the right order with the right values," which is what
//! docs/ESI_INTEGRATION.md §2's numbered steps describe.
//!
//! SCOPES ARE DELIBERATELY EMPTY this sprint. Sprint 011A is
//! authentication only — no asset endpoint, no structure resolution, no
//! ESI game-data call of any kind exists anywhere in this build.
//! Requesting `esi-assets.read_assets.v1` or similar here, with no code
//! that would ever use it, would mean showing the user a CCP consent
//! screen asking for a permission this build doesn't act on — an empty
//! scope list authenticates identity only (who the character is),
//! nothing more. When a later sprint implements asset sync, the scope
//! list here will need to change, and every already-connected character
//! will need to go through Add Character again to grant the new
//! permission — that's expected and correct, not a bug to work around
//! now.

use super::{client, jwt, loopback, pkce};
use std::time::Duration;

pub const REDIRECT_PORT: u16 = 38473;
pub const REDIRECT_URI: &str = "http://localhost:38473/callback";
pub const SCOPES: &[&str] = &[];
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

pub struct AddCharacterResult {
    pub identity: jwt::CharacterIdentity,
    pub access_token: String,
    pub refresh_token: String,
}

/// Runs the full flow. Blocking (uses the synchronous loopback listener)
/// — the Tauri command calling this is deliberately a non-async command
/// function, which Tauri dispatches off its main thread automatically;
/// see commands::add_character's own doc comment for why.
pub fn run_add_character_flow(client_id: &str) -> Result<AddCharacterResult, String> {
    let verifier = pkce::generate_code_verifier();
    let challenge = pkce::code_challenge_s256(&verifier);
    let state = pkce::generate_state();

    let authorize_url = client::build_authorize_url(client_id, REDIRECT_URI, &challenge, &state, SCOPES);

    open::that(&authorize_url).map_err(|e| format!("failed to open the system browser: {e}"))?;

    let callback = loopback::listen_for_callback(REDIRECT_PORT, &state, LOGIN_TIMEOUT)?;

    let http = reqwest::Client::new();
    let tokens = tauri::async_runtime::block_on(client::exchange_code_for_tokens(&http, client_id, &callback.code, &verifier))?;

    let now_unix = chrono::Utc::now().timestamp();
    let identity = jwt::parse_and_validate(&tokens.access_token, client_id, now_unix)?;

    // With SCOPES empty, `requested.is_subset(&granted)` is trivially
    // true for any grant — this check is kept (not deleted) because it
    // becomes meaningful again the moment a future sprint adds real
    // scopes here, and there's no reason to remove correct, if currently
    // inert, logic.
    let requested: std::collections::HashSet<&str> = SCOPES.iter().copied().collect();
    let granted: std::collections::HashSet<&str> = identity.scopes.iter().map(|s| s.as_str()).collect();
    if !requested.is_subset(&granted) {
        return Err(format!(
            "EVE SSO granted fewer scopes than requested — requested {SCOPES:?}, granted {:?}. \
             Try Add Character again and accept every requested permission.",
            identity.scopes
        ));
    }

    Ok(AddCharacterResult {
        identity,
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}

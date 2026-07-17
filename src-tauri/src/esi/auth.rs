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
//! The application requests its complete read-only capability set together.
//! Characters connected before a scope is introduced must run Add Character
//! again so CCP can grant the expanded consent explicitly.

use super::{client, jwt, loopback, pkce};
use std::time::Duration;

pub const REDIRECT_PORT: u16 = 38473;
pub const REDIRECT_URI: &str = "http://localhost:38473/callback";
pub const SCOPES: &[&str] = &[
    "esi-assets.read_assets.v1",
    "esi-characters.read_blueprints.v1",
    "esi-universe.read_structures.v1",
    "esi-assets.read_corporation_assets.v1",
    "esi-characters.read_corporation_roles.v1",
    "esi-corporations.read_divisions.v1",
    "esi-corporations.read_blueprints.v1",
];
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

pub struct AddCharacterResult {
    pub identity: jwt::CharacterIdentity,
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
        refresh_token: tokens.refresh_token,
    })
}

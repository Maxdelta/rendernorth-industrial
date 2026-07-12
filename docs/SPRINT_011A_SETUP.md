# Sprint 011A Setup — EVE SSO Character Authentication Foundation

Scope: authentication only. No asset sync, no universe map, no inventory
scope, no production integration. Every value below is copied directly
from the source in this delivery, cross-checked immediately before
writing this file.

## 1. EVE Developer application

You've already created this. Confirm it has:
- **Connection Type**: Authentication & API Access
- **Callback URL**: exactly `http://localhost:38473/callback` (hardcoded
  in `esi::auth::REDIRECT_URI` — must match exactly)
- **Permissions**: none required for this sprint. `esi::auth::SCOPES` is
  an empty list — this build authenticates identity only and makes no
  ESI game-data call of any kind. If your application has scopes
  selected from earlier registration, that's fine; this build simply
  won't request them yet.

## 2. Client Secret — confirmed from the implementation, not documentation

- **Used?** No. Grepped `esi::client` just now: the token exchange and
  refresh functions send exactly `grant_type`, `code`/`refresh_token`,
  `code_verifier` (exchange only), and `client_id`. No `client_secret`
  field exists anywhere in either request.
- **Where configured?** Nowhere — there's no field for it anywhere in
  this codebase or the Characters panel UI.
- **Why not required?** This is Authorization Code with PKCE, a native
  desktop app can't keep a secret safe (it ships inside the binary), so
  PKCE's `code_verifier`/`code_challenge` pair replaces the need for one
  — CCP's own recommended pattern for this app type.
- Paste only the **Client ID** into the Characters panel. Ignore the
  Client Secret.

## 3. Build

```powershell
cd src-tauri
cargo build
```

This is the actual first compile of this code — I have no Rust compiler
in my environment. If it fails, send me the exact error.

```powershell
cargo test
```

## 4. Local verification, in order

1. `cargo build` — see above.
2. `cargo test` — see above.
3. `npm run tauri dev` (from the project root) — launches the app; this
   also runs migration 0011 for the first time against your real
   database (adds `scopes_granted`, `enabled`, `authorization_status`,
   `token_expires_at`, `last_login_at` to `characters` — nothing else).
4. Settings → Characters → paste your Client ID → **Add Character**.
5. Your system browser should open to `login.eveonline.com`.
6. Log in, review the (empty) permission list, accept.
7. The browser tab shows "Login successful — you can close this window."
   The app has been waiting on the loopback callback in the background.
8. The character should appear in the Characters list within a few
   seconds, showing Authorization: `authorized`, Enabled: `ON`, Last
   Login: just now.

Stop there — that's the sprint's complete success criterion. Test Enable/
Disable (toggles the ON/OFF button) and Remove (two-step confirm) if you
want, but nothing beyond the workflow above is in scope.

**If it fails**: the error should say which step failed (browser didn't
open, callback never arrived, state mismatch, token exchange failed). If
the browser opens but nothing happens after you approve, check nothing
else on your machine is using port 38473.

## 5. What I could not verify myself, and why

No network route to `login.eveonline.com` in my environment, and no OS
keychain daemon to test `keyring` against. PKCE math is verified against
RFC 7636's own published test vector; JWT claim validation and the
loopback listener's request parsing are unit-tested with hand-built data;
the SQL schema and transactions are validated against real SQLite. What I
could not validate is the actual network round-trip to CCP's servers or
the real Windows Credential Manager — step 4 above is the first live test
of that boundary.

## 6. Explicitly not implemented (per this sprint's scope)

Asset synchronization, asset parsing, inventory import, jump
calculations, universe graph, inventory scope, production integration,
build location logic, corporation support, wallet, market, manufacturing
jobs, recommendations, automation. No placeholder code for any of these
exists in this delivery — they were fully removed, not stubbed out.

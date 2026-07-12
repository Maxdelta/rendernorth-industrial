//! The HTTP boundary to CCP's SSO — token exchange and refresh only.
//! Sprint 011A is authentication-only: no asset endpoints, no structure
//! resolution, nothing beyond what's needed to log a character in and
//! keep its authorization state accurate.
//!
//! IMPORTANT, HONEST LIMITATION: this environment has no network route
//! to login.eveonline.com, so the token exchange/refresh calls below
//! have not been exercised against real CCP servers by me. They're
//! written precisely to CCP's documented SSO v2 contract. The
//! response-parsing logic is unit-tested against hand-built JSON
//! matching that documented shape — but the live round-trip needs your
//! own testing, per the sprint's verification steps.

use serde::Deserialize;

const SSO_TOKEN_URL: &str = "https://login.eveonline.com/v2/oauth/token";
pub const SSO_AUTHORIZE_URL: &str = "https://login.eveonline.com/v2/oauth/authorize";

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

/// Builds the URL the system browser is opened at. Per
/// docs/ESI_INTEGRATION.md §2 step 2. PKCE only — no client secret
/// parameter exists anywhere in this function or anywhere else in this
/// module; see the sprint's explicit confirmation of this.
pub fn build_authorize_url(client_id: &str, redirect_uri: &str, code_challenge: &str, state: &str, scopes: &[&str]) -> String {
    let scope_param = scopes.join(" ");
    format!(
        "{SSO_AUTHORIZE_URL}?response_type=code&redirect_uri={}&client_id={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        urlencoding_encode(redirect_uri),
        urlencoding_encode(client_id),
        urlencoding_encode(&scope_param),
        urlencoding_encode(code_challenge),
        urlencoding_encode(state),
    )
}

/// Exchanges an authorization code for an access/refresh token pair.
/// Per docs/ESI_INTEGRATION.md §2 step 4. Native/PKCE apps authenticate
/// with `client_id` + `code_verifier` only — no client secret. This is
/// the exact, complete form body sent; there is no other field.
pub async fn exchange_code_for_tokens(client: &reqwest::Client, client_id: &str, code: &str, code_verifier: &str) -> Result<TokenResponse, String> {
    let form = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("client_id", client_id),
    ];
    let resp = client
        .post(SSO_TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;
    parse_token_response(resp).await
}

/// Refreshes an access token. ESI refresh tokens for PKCE apps don't
/// expire on their own but are invalidated if the user revokes access.
/// Same no-client-secret shape as the exchange above.
pub async fn refresh_access_token(client: &reqwest::Client, client_id: &str, refresh_token: &str) -> Result<TokenResponse, String> {
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];
    let resp = client
        .post(SSO_TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("token refresh request failed: {e}"))?;
    parse_token_response(resp).await
}

async fn parse_token_response(resp: reqwest::Response) -> Result<TokenResponse, String> {
    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("failed to read token response body: {e}"))?;
    if !status.is_success() {
        return Err(format!("SSO token endpoint returned {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("failed to parse token response: {e} — body was: {body}"))
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_includes_every_required_parameter_and_no_client_secret() {
        let url = build_authorize_url(
            "my-client-id",
            "http://localhost:38473/callback",
            "the-challenge",
            "the-state",
            &["esi-assets.read_assets.v1"],
        );
        assert!(url.starts_with(SSO_AUTHORIZE_URL));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("code_challenge=the-challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=the-state"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A38473%2Fcallback"));
        assert!(!url.to_lowercase().contains("secret"), "no client secret parameter should ever appear in the authorize URL");
    }

    #[test]
    fn urlencoding_matches_expected_percent_encoding() {
        assert_eq!(urlencoding_encode("a b"), "a%20b");
        assert_eq!(urlencoding_encode("esi-assets.read_assets.v1"), "esi-assets.read_assets.v1");
        assert_eq!(urlencoding_encode("http://localhost:38473/callback"), "http%3A%2F%2Flocalhost%3A38473%2Fcallback");
    }

    #[test]
    fn token_response_json_matching_documented_shape_parses_correctly() {
        let json = r#"{"access_token": "eyJhbGciOi...", "token_type": "Bearer", "expires_in": 1199, "refresh_token": "abcdef123456"}"#;
        let parsed: TokenResponse = serde_json::from_str(json).expect("should parse (token_type ignored)");
        assert_eq!(parsed.expires_in, 1199);
        assert_eq!(parsed.refresh_token, "abcdef123456");
    }
}

//! EVE SSO access tokens are JWTs. This module decodes the claims we
//! need (character id, name, owner hash, scopes, expiry) and validates
//! issuer/audience/expiry per docs/ESI_INTEGRATION.md §2 step 5.
//!
//! DISCLOSED LIMITATION: this validates claims, not the cryptographic
//! signature. Full verification requires fetching and caching CCP's
//! JWKS (https://login.eveonline.com/oauth/jwks) and implementing RS256
//! signature verification — a real, separate piece of work. Until that
//! lands, this trusts that a token obtained through our own PKCE
//! exchange (over TLS, directly from login.eveonline.com, never from
//! anywhere else) is authentic by provenance rather than by verifying
//! its signature. This is a meaningfully weaker guarantee than full
//! verification and should be closed before this app is ever used with
//! untrusted token sources.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    name: String,
    owner: Option<String>,
    exp: i64,
    iss: String,
    #[serde(default)]
    aud: Vec<String>,
    #[serde(default)]
    scp: ScopeClaim,
}

/// EVE SSO returns `scp` as a single string when one scope was granted,
/// or an array when multiple were — both are seen in practice.
#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum ScopeClaim {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl ScopeClaim {
    fn into_vec(self) -> Vec<String> {
        match self {
            ScopeClaim::None => vec![],
            ScopeClaim::One(s) => vec![s],
            ScopeClaim::Many(v) => v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterIdentity {
    pub character_id: i64,
    pub character_name: String,
    pub owner_hash: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at_unix: i64,
}

const EXPECTED_ISSUERS: &[&str] = &["https://login.eveonline.com", "login.eveonline.com"];
const EXPECTED_AUDIENCE: &str = "EVE Online";

/// Decodes and validates an EVE SSO access token's claims, returning the
/// character identity they describe. Does not verify the cryptographic
/// signature — see the module-level doc comment.
pub fn parse_and_validate(access_token: &str, client_id: &str, now_unix: i64) -> Result<CharacterIdentity, String> {
    let parts: Vec<&str> = access_token.split('.').collect();
    if parts.len() != 3 {
        return Err("access token is not a well-formed JWT (expected 3 dot-separated parts)".into());
    }

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| format!("failed to base64-decode JWT payload: {e}"))?;
    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| format!("failed to parse JWT payload as the expected claim set: {e}"))?;

    if !EXPECTED_ISSUERS.contains(&claims.iss.as_str()) {
        return Err(format!("unexpected token issuer '{}' — refusing a token not issued by EVE SSO", claims.iss));
    }
    if !claims.aud.iter().any(|a| a == EXPECTED_AUDIENCE) && !claims.aud.iter().any(|a| a == client_id) {
        return Err(format!(
            "token audience {:?} does not include this app's client_id or 'EVE Online' — refusing",
            claims.aud
        ));
    }
    if claims.exp <= now_unix {
        return Err(format!("token expired at {} (now {})", claims.exp, now_unix));
    }

    let character_id_str = claims
        .sub
        .strip_prefix("CHARACTER:EVE:")
        .ok_or_else(|| format!("unexpected subject claim shape: '{}'", claims.sub))?;
    let character_id: i64 = character_id_str
        .parse()
        .map_err(|e| format!("subject claim's character id was not a valid integer: {e}"))?;

    Ok(CharacterIdentity {
        character_id,
        character_name: claims.name,
        owner_hash: claims.owner,
        scopes: claims.scp.into_vec(),
        expires_at_unix: claims.exp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    /// Builds an unsigned (test-only) JWT with the given claims JSON —
    /// signature verification isn't implemented (see module doc), so
    /// this is sufficient to exercise the claims-parsing and validation
    /// logic without needing a real EVE SSO token.
    fn fake_jwt(claims_json: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(claims_json);
        format!("{header}.{payload}.fake-signature-not-verified")
    }

    #[test]
    fn valid_token_parses_into_expected_identity() {
        let token = fake_jwt(
            r#"{"sub":"CHARACTER:EVE:2112073677","name":"Test Pilot","owner":"abc123",
                "exp":2000000000,"iss":"https://login.eveonline.com","aud":["my-client-id","EVE Online"],
                "scp":["esi-assets.read_assets.v1","esi-universe.read_structures.v1"]}"#,
        );
        let identity = parse_and_validate(&token, "my-client-id", 1_000_000_000).expect("should parse");
        assert_eq!(identity.character_id, 2112073677);
        assert_eq!(identity.character_name, "Test Pilot");
        assert_eq!(identity.owner_hash, Some("abc123".to_string()));
        assert_eq!(identity.scopes, vec!["esi-assets.read_assets.v1", "esi-universe.read_structures.v1"]);
    }

    #[test]
    fn single_scope_claim_as_bare_string_is_accepted() {
        let token = fake_jwt(
            r#"{"sub":"CHARACTER:EVE:123","name":"X","exp":2000000000,
                "iss":"login.eveonline.com","aud":["EVE Online"],"scp":"esi-assets.read_assets.v1"}"#,
        );
        let identity = parse_and_validate(&token, "cid", 1_000_000_000).expect("should parse");
        assert_eq!(identity.scopes, vec!["esi-assets.read_assets.v1"]);
    }

    #[test]
    fn expired_token_is_rejected() {
        let token = fake_jwt(
            r#"{"sub":"CHARACTER:EVE:123","name":"X","exp":100,
                "iss":"https://login.eveonline.com","aud":["EVE Online"],"scp":[]}"#,
        );
        let result = parse_and_validate(&token, "cid", 1_000_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expired"));
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let token = fake_jwt(
            r#"{"sub":"CHARACTER:EVE:123","name":"X","exp":2000000000,
                "iss":"https://not-eve-online.example","aud":["EVE Online"],"scp":[]}"#,
        );
        let result = parse_and_validate(&token, "cid", 1_000_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("issuer"));
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let token = fake_jwt(
            r#"{"sub":"CHARACTER:EVE:123","name":"X","exp":2000000000,
                "iss":"https://login.eveonline.com","aud":["some-other-app"],"scp":[]}"#,
        );
        let result = parse_and_validate(&token, "cid", 1_000_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("audience"));
    }

    #[test]
    fn malformed_subject_claim_is_rejected() {
        let token = fake_jwt(
            r#"{"sub":"not-the-right-shape","name":"X","exp":2000000000,
                "iss":"https://login.eveonline.com","aud":["EVE Online"],"scp":[]}"#,
        );
        let result = parse_and_validate(&token, "cid", 1_000_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("subject"));
    }
}

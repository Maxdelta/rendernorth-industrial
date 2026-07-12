//! PKCE (RFC 7636) verifier/challenge generation and OAuth `state` token
//! generation. Pure functions — no network, no filesystem — so every
//! byte of this is verifiable offline, including against RFC 7636's own
//! published worked example (see the test below). This is deliberately
//! the smallest possible hand-rolled PKCE implementation rather than a
//! full OAuth crate, per Sprint 011's approved dependency scope.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// A `code_verifier` per RFC 7636 §4.1: 43–128 characters from the
/// unreserved URL character set. We generate 32 random bytes and
/// base64url-encode them (no padding), which always yields exactly 43
/// characters — within spec, and simpler than the general case.
pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `code_challenge` per RFC 7636 §4.2, S256 method: base64url(no pad) of
/// SHA-256(ascii(code_verifier)). Verified against RFC 7636 Appendix B's
/// published example in the test below — this is not just internally
/// self-consistent, it matches the standard's own authoritative output.
pub fn code_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

/// A random `state` value for CSRF protection on the OAuth redirect —
/// the loopback listener refuses any callback whose `state` doesn't
/// match what was sent in the authorize request.
pub fn generate_state() -> String {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_challenge_matches_rfc_7636_appendix_b_worked_example() {
        // https://www.rfc-editor.org/rfc/rfc7636#appendix-B — the
        // standard's own published verifier/challenge pair. If this
        // passes, the S256 computation is provably correct, independent
        // of anything else in this codebase or any live network call.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(code_challenge_s256(verifier), expected_challenge);
    }

    #[test]
    fn generated_verifier_is_43_chars_and_url_safe() {
        let v = generate_code_verifier();
        assert_eq!(v.len(), 43, "32 raw bytes base64url-encoded without padding is always 43 chars");
        assert!(v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn generated_verifier_and_state_are_not_deterministic() {
        // Not a security proof, but catches the class of bug where a
        // fixed seed or a copy-paste error makes every value identical.
        let a = generate_code_verifier();
        let b = generate_code_verifier();
        assert_ne!(a, b);
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
    }

    #[test]
    fn challenge_is_deterministic_for_the_same_verifier() {
        let v = generate_code_verifier();
        assert_eq!(code_challenge_s256(&v), code_challenge_s256(&v));
    }
}

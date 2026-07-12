//! Everything related to EVE SSO authentication and ESI API access.
//! Read-only, official-endpoints-only, per docs/ESI_INTEGRATION.md.
//! Tokens never leave this module boundary except into the OS keychain
//! (token_store) — nothing outside `esi::` ever sees a raw access or
//! refresh token.

pub mod auth;
pub mod assets;
pub mod client;
pub mod jwt;
pub mod loopback;
pub mod pkce;
pub mod token_store;

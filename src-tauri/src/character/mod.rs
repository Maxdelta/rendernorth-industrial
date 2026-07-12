//! Connected EVE characters — Sprint 011A scope: authentication only.
//! Auth-state bookkeeping (never tokens — see esi::token_store), Add/
//! Remove/Enable/List. No asset sync, no location resolution, no
//! inventory integration — those belong to a later sprint.

pub mod engine;
pub mod models;
pub mod repository;

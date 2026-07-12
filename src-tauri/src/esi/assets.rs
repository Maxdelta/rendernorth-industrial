//! Read-only ESI character asset transport. Raw tokens remain inside the
//! `esi` boundary and are never returned to commands or the frontend.

use serde::{Deserialize, Serialize};

const ESI_BASE_URL: &str = "https://esi.evetech.net/latest";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AssetRecord {
    pub item_id: i64,
    pub type_id: i64,
    pub quantity: i64,
    pub location_id: i64,
    pub location_type: String,
    pub location_flag: String,
    pub is_singleton: bool,
}

pub struct AssetPage {
    pub records: Vec<AssetRecord>,
    pub total_pages: u32,
}

pub trait AssetPageSource {
    fn fetch_page(&self, access_token: &str, character_id: i64, page: u32) -> Result<AssetPage, String>;
}

pub trait AccessTokenRefresher {
    fn refresh(&self, client_id: &str, refresh_token: &str) -> Result<super::client::TokenResponse, String>;
}

struct LiveAccessTokenRefresher { http: reqwest::Client }
impl AccessTokenRefresher for LiveAccessTokenRefresher {
    fn refresh(&self, client_id: &str, refresh_token: &str) -> Result<super::client::TokenResponse, String> {
        tauri::async_runtime::block_on(super::client::refresh_access_token(&self.http, client_id, refresh_token))
    }
}

struct LiveAssetPageSource {
    http: reqwest::Client,
}

impl AssetPageSource for LiveAssetPageSource {
    fn fetch_page(&self, access_token: &str, character_id: i64, page: u32) -> Result<AssetPage, String> {
        let url = format!("{ESI_BASE_URL}/characters/{character_id}/assets/?page={page}");
        let response = tauri::async_runtime::block_on(
            self.http.get(url).bearer_auth(access_token).send(),
        )
        .map_err(|e| format!("asset page {page} request failed: {e}"))?;
        let status = response.status();
        let total_pages = response
            .headers()
            .get("x-pages")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);
        let body = tauri::async_runtime::block_on(response.text())
            .map_err(|e| format!("failed to read asset page {page}: {e}"))?;
        if !status.is_success() {
            return Err(format!("ESI assets page {page} returned {status}: {body}"));
        }
        let records = serde_json::from_str(&body)
            .map_err(|e| format!("failed to parse asset page {page}: {e}"))?;
        Ok(AssetPage { records, total_pages })
    }
}

pub fn fetch_all_pages<S: AssetPageSource>(source: &S, access_token: &str, character_id: i64) -> Result<(Vec<AssetRecord>, u32), String> {
    let first = source.fetch_page(access_token, character_id, 1)?;
    let total_pages = first.total_pages.max(1);
    let mut records = first.records;
    for page in 2..=total_pages {
        records.extend(source.fetch_page(access_token, character_id, page)?.records);
    }
    Ok((records, total_pages))
}

#[cfg(test)]
pub fn refresh_then_fetch<R: AccessTokenRefresher, S: AssetPageSource>(
    refresher: &R, source: &S, client_id: &str, refresh_token: &str, character_id: i64,
) -> Result<(Vec<AssetRecord>, u32, String), String> {
    let tokens = refresher.refresh(client_id, refresh_token)?;
    if tokens.expires_in <= 0 {
        return Err("token refresh returned a non-positive expiry".into());
    }
    let (assets, pages) = fetch_all_pages(source, &tokens.access_token, character_id)?;
    Ok((assets, pages, tokens.refresh_token))
}

/// Refresh first, then fetch every page. This avoids persisting access
/// tokens and guarantees a usable token even when the prior grant expired.
pub fn fetch_character_assets(client_id: &str, character_id: i64) -> Result<(Vec<AssetRecord>, u32), String> {
    let refresh_token = super::token_store::get_refresh_token(character_id)?;
    let http = reqwest::Client::new();
    let refreshed = LiveAccessTokenRefresher { http: http.clone() }.refresh(client_id, &refresh_token)?;
    if refreshed.expires_in <= 0 {
        return Err("token refresh returned a non-positive expiry".into());
    }
    // CCP may rotate the refresh token. Persist it before the first asset
    // request so a later page failure cannot strand the authorization.
    super::token_store::store_refresh_token(character_id, &refreshed.refresh_token)?;
    fetch_all_pages(&LiveAssetPageSource { http }, &refreshed.access_token, character_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct FakePages { calls: RefCell<Vec<u32>> }
    impl AssetPageSource for FakePages {
        fn fetch_page(&self, _token: &str, _character_id: i64, page: u32) -> Result<AssetPage, String> {
            self.calls.borrow_mut().push(page);
            Ok(AssetPage {
                records: vec![AssetRecord {
                    item_id: page as i64,
                    type_id: 34,
                    quantity: page as i64,
                    location_id: 60003760,
                    location_type: "station".into(),
                    location_flag: "Hangar".into(),
                    is_singleton: false,
                }],
                total_pages: 3,
            })
        }
    }

    #[test]
    fn multi_page_asset_fetch_reads_every_reported_page() {
        let source = FakePages { calls: RefCell::new(vec![]) };
        let (records, pages) = fetch_all_pages(&source, "secret-never-logged", 42).unwrap();
        assert_eq!(pages, 3);
        assert_eq!(records.len(), 3);
        assert_eq!(*source.calls.borrow(), vec![1, 2, 3]);
    }

    struct FakeRefresh { calls: RefCell<usize> }
    impl AccessTokenRefresher for FakeRefresh {
        fn refresh(&self, _client_id:&str, refresh_token:&str)->Result<super::super::client::TokenResponse,String>{
            assert_eq!(refresh_token,"old-refresh");
            *self.calls.borrow_mut() += 1;
            Ok(super::super::client::TokenResponse { access_token:"new-access".into(), refresh_token:"new-refresh".into(), expires_in:1200 })
        }
    }

    #[test]
    fn token_refresh_happens_before_asset_fetch_and_rotated_token_is_returned() {
        let refresh=FakeRefresh{calls:RefCell::new(0)};
        let pages=FakePages{calls:RefCell::new(vec![])};
        let (assets,count,rotated)=refresh_then_fetch(&refresh,&pages,"client","old-refresh",42).unwrap();
        assert_eq!(*refresh.calls.borrow(),1);
        assert_eq!((assets.len(),count,rotated.as_str()),(3,3,"new-refresh"));
    }
}

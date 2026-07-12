//! Read-only ESI character blueprint transport. The official route returns
//! one record per blueprint item and paginates via `X-Pages`.
//!
//! ESI semantics: `runs == -1` is an original (BPO). Copies (BPCs) have
//! `runs >= 0`; `quantity` is typically -1 for a singleton original and -2
//! for a copy, but `runs` is the authoritative BPO/BPC discriminator.

use serde::{Deserialize, Serialize};

const ESI_BASE_URL: &str = "https://esi.evetech.net/latest";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EsiBlueprint {
    pub item_id: i64,
    pub type_id: i64,
    pub location_id: i64,
    pub location_flag: String,
    pub quantity: i64,
    pub material_efficiency: i64,
    pub time_efficiency: i64,
    pub runs: i64,
}

#[cfg(test)]
impl EsiBlueprint {
    pub fn is_copy(&self) -> bool { self.runs != -1 }
}

pub struct BlueprintPage { pub records: Vec<EsiBlueprint>, pub total_pages: u32 }

pub trait BlueprintPageSource {
    fn fetch_page(&self, access_token: &str, character_id: i64, page: u32) -> Result<BlueprintPage, String>;
}

#[cfg(test)]
trait TokenRefresher { fn refresh(&self, client_id:&str, refresh_token:&str)->Result<super::client::TokenResponse,String>; }

#[cfg(test)]
fn refresh_then_fetch<R:TokenRefresher,S:BlueprintPageSource>(refresher:&R,source:&S,client_id:&str,refresh_token:&str,character_id:i64)->Result<(Vec<EsiBlueprint>,u32,String),String>{
    let tokens=refresher.refresh(client_id,refresh_token)?;
    let (rows,pages)=fetch_all_pages(source,&tokens.access_token,character_id)?;
    Ok((rows,pages,tokens.refresh_token))
}

struct LiveBlueprintPageSource { http: reqwest::Client }
impl BlueprintPageSource for LiveBlueprintPageSource {
    fn fetch_page(&self, access_token: &str, character_id: i64, page: u32) -> Result<BlueprintPage, String> {
        let url = format!("{ESI_BASE_URL}/characters/{character_id}/blueprints/?page={page}");
        let response = tauri::async_runtime::block_on(self.http.get(url).bearer_auth(access_token).send())
            .map_err(|e| format!("blueprint page {page} request failed: {e}"))?;
        let status = response.status();
        let total_pages = response.headers().get("x-pages")
            .and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u32>().ok()).unwrap_or(1);
        let body = tauri::async_runtime::block_on(response.text())
            .map_err(|e| format!("failed to read blueprint page {page}: {e}"))?;
        if !status.is_success() {
            return Err(format!("ESI blueprints page {page} returned {status}: {body}"));
        }
        let records = serde_json::from_str(&body)
            .map_err(|e| format!("failed to parse blueprint page {page}: {e}"))?;
        Ok(BlueprintPage { records, total_pages })
    }
}

pub fn fetch_all_pages<S: BlueprintPageSource>(source: &S, access_token: &str, character_id: i64) -> Result<(Vec<EsiBlueprint>, u32), String> {
    let first = source.fetch_page(access_token, character_id, 1)?;
    let total_pages = first.total_pages.max(1);
    let mut records = first.records;
    for page in 2..=total_pages { records.extend(source.fetch_page(access_token, character_id, page)?.records); }
    Ok((records, total_pages))
}

pub fn fetch_character_blueprints(client_id: &str, character_id: i64) -> Result<(Vec<EsiBlueprint>, u32), String> {
    let refresh_token = super::token_store::get_refresh_token(character_id)?;
    let http = reqwest::Client::new();
    let refreshed = tauri::async_runtime::block_on(super::client::refresh_access_token(&http, client_id, &refresh_token))?;
    if refreshed.expires_in <= 0 { return Err("token refresh returned a non-positive expiry".into()); }
    super::token_store::store_refresh_token(character_id, &refreshed.refresh_token)?;
    fetch_all_pages(&LiveBlueprintPageSource { http }, &refreshed.access_token, character_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Pages { calls: RefCell<Vec<u32>> }
    impl BlueprintPageSource for Pages {
        fn fetch_page(&self, _: &str, _: i64, page: u32) -> Result<BlueprintPage, String> {
            self.calls.borrow_mut().push(page);
            Ok(BlueprintPage { records: vec![EsiBlueprint { item_id:page as i64,type_id:681,location_id:600,location_flag:"Hangar".into(),quantity:-1,material_efficiency:10,time_efficiency:20,runs:-1 }], total_pages:3 })
        }
    }
    #[test]
    fn multi_page_blueprint_fetch_reads_every_page() {
        let source=Pages{calls:RefCell::new(vec![])};
        let (rows,pages)=fetch_all_pages(&source,"token",42).unwrap();
        assert_eq!(pages,3); assert_eq!(rows.len(),3); assert_eq!(*source.calls.borrow(),vec![1,2,3]);
    }
    #[test]
    fn bpo_bpc_interpretation_uses_runs_semantics() {
        let mut bp=EsiBlueprint{item_id:1,type_id:681,location_id:1,location_flag:"Hangar".into(),quantity:-1,material_efficiency:10,time_efficiency:20,runs:-1};
        assert!(!bp.is_copy());
        bp.quantity=-2; bp.runs=5; assert!(bp.is_copy());
    }
    struct Refresh{calls:RefCell<usize>} impl TokenRefresher for Refresh{fn refresh(&self,_:&str,token:&str)->Result<super::super::client::TokenResponse,String>{assert_eq!(token,"old");*self.calls.borrow_mut()+=1;Ok(super::super::client::TokenResponse{access_token:"access".into(),refresh_token:"rotated".into(),expires_in:1200})}}
    #[test]fn blueprint_fetch_refreshes_access_token_first(){let refresh=Refresh{calls:RefCell::new(0)};let pages=Pages{calls:RefCell::new(vec![])};let (rows,count,rotated)=refresh_then_fetch(&refresh,&pages,"client","old",42).unwrap();assert_eq!(*refresh.calls.borrow(),1);assert_eq!((rows.len(),count,rotated.as_str()),(3,3,"rotated"));}
}

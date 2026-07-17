//! Read-only corporation asset transport. Access and refresh tokens never
//! leave the ESI boundary or enter logs/frontend DTOs.

use super::assets::AssetRecord;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

const ESI_BASE: &str = "https://esi.evetech.net/latest";
const COMPATIBILITY_DATE: &str = "2026-07-17";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Division {
    pub number: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CorporationSnapshot {
    pub corporation_id: i64,
    pub corporation_name: String,
    pub divisions: Vec<Division>,
    pub assets: Vec<AssetRecord>,
    pub page_count: u32,
}

#[derive(Deserialize)]
struct CharacterInfo {
    corporation_id: i64,
}
#[derive(Deserialize)]
struct CorporationInfo {
    name: String,
}
#[derive(Deserialize, Default)]
struct CharacterRoles {
    #[serde(default)]
    roles: Vec<String>,
}
#[derive(Deserialize, Default)]
struct DivisionResponse {
    #[serde(default)]
    hangar: Vec<DivisionEntry>,
}
#[derive(Deserialize)]
struct DivisionEntry {
    division: i64,
    name: String,
}

fn body(response: reqwest::Response, context: &str) -> Result<String, String> {
    let status = response.status();
    let text = tauri::async_runtime::block_on(response.text())
        .map_err(|e| format!("failed to read {context}: {e}"))?;
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(format!("{context} was forbidden by CCP ESI. Corporation asset synchronization requires the Director role in the character's current corporation."));
    }
    if !status.is_success() {
        return Err(format!("{context} returned {status}: {text}"));
    }
    Ok(text)
}

fn request(
    http: &reqwest::Client,
    url: String,
    token: Option<&str>,
    context: &str,
) -> Result<(String, reqwest::header::HeaderMap), String> {
    let mut req = http
        .get(url)
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .header(
            "User-Agent",
            concat!("RenderNorthIndustrial/", env!("CARGO_PKG_VERSION")),
        );
    if let Some(token) = token {
        req = req.bearer_auth(token);
    }
    let response = tauri::async_runtime::block_on(req.send())
        .map_err(|e| format!("{context} request failed: {e}"))?;
    let headers = response.headers().clone();
    Ok((body(response, context)?, headers))
}

pub trait CorporationSnapshotSource {
    fn fetch(
        &self,
        client_id: &str,
        authorizing_character_id: i64,
    ) -> Result<CorporationSnapshot, String>;
}

pub struct LiveCorporationSnapshotSource;
impl CorporationSnapshotSource for LiveCorporationSnapshotSource {
    fn fetch(&self, client_id: &str, character_id: i64) -> Result<CorporationSnapshot, String> {
        let refresh = super::token_store::get_refresh_token(character_id)?;
        let http = reqwest::Client::new();
        let tokens = tauri::async_runtime::block_on(super::client::refresh_access_token(
            &http, client_id, &refresh,
        ))?;
        if tokens.expires_in <= 0 {
            return Err("token refresh returned a non-positive expiry".into());
        }
        super::token_store::store_refresh_token(character_id, &tokens.refresh_token)?;

        let (text, _) = request(
            &http,
            format!("{ESI_BASE}/characters/{character_id}/"),
            None,
            "character corporation lookup",
        )?;
        let character: CharacterInfo = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse character corporation lookup: {e}"))?;
        let corporation_id = character.corporation_id;

        let (text, _) = request(
            &http,
            format!("{ESI_BASE}/characters/{character_id}/roles/"),
            Some(&tokens.access_token),
            "corporation role check",
        )?;
        let roles: CharacterRoles = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse corporation roles: {e}"))?;
        if !roles.roles.iter().any(|role| role == "Director") {
            return Err(format!("character {character_id} lacks the required Director role for corporation asset synchronization"));
        }

        let (text, _) = request(
            &http,
            format!("{ESI_BASE}/corporations/{corporation_id}/"),
            None,
            "corporation identity lookup",
        )?;
        let corporation: CorporationInfo = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse corporation identity: {e}"))?;

        let (text, _) = request(
            &http,
            format!("{ESI_BASE}/corporations/{corporation_id}/divisions/"),
            Some(&tokens.access_token),
            "corporation divisions",
        )?;
        let division_response: DivisionResponse = serde_json::from_str(&text)
            .map_err(|e| format!("failed to parse corporation divisions: {e}"))?;
        let divisions = division_response
            .hangar
            .into_iter()
            .map(|entry| Division {
                number: entry.division,
                name: entry.name,
            })
            .collect();

        let mut assets = Vec::new();
        let mut page = 1;
        let total_pages = loop {
            let (text, headers) = request(
                &http,
                format!("{ESI_BASE}/corporations/{corporation_id}/assets/?page={page}"),
                Some(&tokens.access_token),
                &format!("corporation assets page {page}"),
            )?;
            let mut rows: Vec<AssetRecord> = serde_json::from_str(&text)
                .map_err(|e| format!("failed to parse corporation assets page {page}: {e}"))?;
            assets.append(&mut rows);
            let pages = headers
                .get("x-pages")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1)
                .max(1);
            if page >= pages {
                break pages;
            }
            page += 1;
        };
        Ok(CorporationSnapshot {
            corporation_id,
            corporation_name: corporation.name,
            divisions,
            assets,
            page_count: total_pages,
        })
    }
}

pub fn division_for_flag(flag: &str, divisions: &[Division]) -> (Option<i64>, Option<String>) {
    let number = flag
        .strip_prefix("CorpSAG")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| (1..=7).contains(value));
    match number {
        Some(number) => {
            let name = divisions
                .iter()
                .find(|division| division.number == number)
                .map(|division| division.name.clone())
                .unwrap_or_else(|| format!("Division {number}"));
            (Some(number), Some(name))
        }
        None if flag == "CorpDeliveries" || flag == "Deliveries" => {
            (None, Some("Deliveries".into()))
        }
        None => (None, None),
    }
}

/// Finds the corporation hangar division for an asset, walking through
/// containing ships and containers until ESI's root CorpSAG flag is reached.
pub fn division_for_asset(
    asset: &AssetRecord,
    assets: &[AssetRecord],
    divisions: &[Division],
) -> (Option<i64>, Option<String>) {
    let by_item = assets
        .iter()
        .map(|candidate| (candidate.item_id, candidate))
        .collect::<HashMap<_, _>>();
    let mut current = asset;
    let mut visited = HashSet::new();
    for _ in 0..32 {
        let division = division_for_flag(&current.location_flag, divisions);
        if division.0.is_some() || division.1.is_some() {
            return division;
        }
        if !visited.insert(current.item_id) {
            break;
        }
        let Some(parent) = by_item.get(&current.location_id) else {
            break;
        };
        current = parent;
    }
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn division_flags_use_custom_name_or_safe_default() {
        let d = vec![Division {
            number: 2,
            name: "Minerals".into(),
        }];
        assert_eq!(
            division_for_flag("CorpSAG2", &d),
            (Some(2), Some("Minerals".into()))
        );
        assert_eq!(
            division_for_flag("CorpSAG7", &d),
            (Some(7), Some("Division 7".into()))
        );
        assert_eq!(division_for_flag("Cargo", &d), (None, None));
    }

    #[test]
    fn nested_assets_inherit_the_root_corporation_division() {
        let divisions = vec![Division {
            number: 2,
            name: "Minerals".into(),
        }];
        let container = AssetRecord {
            item_id: 100,
            type_id: 1,
            quantity: 1,
            location_id: 60003760,
            location_type: "station".into(),
            location_flag: "CorpSAG2".into(),
            is_singleton: true,
        };
        let nested = AssetRecord {
            item_id: 101,
            type_id: 34,
            quantity: 10,
            location_id: 100,
            location_type: "item".into(),
            location_flag: "Cargo".into(),
            is_singleton: false,
        };
        let assets = vec![container, nested.clone()];
        assert_eq!(
            division_for_asset(&nested, &assets, &divisions),
            (Some(2), Some("Minerals".into()))
        );
    }
}

//! RNI-159 — read-only personal commerce synchronization and queries.
//! Market orders, contract lists, contract items, and bids remain isolated
//! from corporation data and every production/procurement calculation.

use chrono::{DateTime, Duration, Utc};
use reqwest::header::{ETAG, IF_NONE_MATCH, LAST_MODIFIED};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Number;

pub const MARKET_SCOPE: &str = "esi-markets.read_character_orders.v1";
pub const CONTRACT_SCOPE: &str = "esi-contracts.read_character_contracts.v1";
const ESI_BASE: &str = "https://esi.evetech.net";
const COMPATIBILITY_DATE: &str = "2026-07-18";
const SOURCE_ORDERS: &str = "ESI Character Market Orders";
const SOURCE_CONTRACTS: &str = "ESI Character Contracts";

#[derive(Debug, Clone, Deserialize)]
pub struct EsiMarketOrder {
    pub duration: i64,
    pub escrow: Option<Number>,
    #[serde(default)]
    pub is_buy_order: bool,
    pub is_corporation: bool,
    pub issued: String,
    pub location_id: i64,
    pub min_volume: Option<i64>,
    pub order_id: i64,
    pub price: Number,
    pub range: String,
    pub region_id: i64,
    pub type_id: i64,
    pub volume_remain: i64,
    pub volume_total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EsiContract {
    pub acceptor_id: i64,
    pub assignee_id: i64,
    pub availability: String,
    pub buyout: Option<Number>,
    pub collateral: Option<Number>,
    pub contract_id: i64,
    pub date_accepted: Option<String>,
    pub date_completed: Option<String>,
    pub date_expired: String,
    pub date_issued: String,
    pub days_to_complete: Option<i64>,
    pub end_location_id: Option<i64>,
    pub for_corporation: bool,
    pub issuer_corporation_id: i64,
    pub issuer_id: i64,
    pub price: Option<Number>,
    pub reward: Option<Number>,
    pub start_location_id: Option<i64>,
    pub status: String,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub contract_type: String,
    pub volume: Option<Number>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EsiContractItem {
    pub is_included: bool,
    pub is_singleton: bool,
    pub quantity: i64,
    pub raw_quantity: Option<i64>,
    pub record_id: i64,
    pub type_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EsiContractBid {
    pub amount: Number,
    pub bid_id: i64,
    pub bidder_id: i64,
    pub date_bid: String,
}

#[derive(Debug, Clone)]
pub enum MarketFetch {
    Modified {
        rows: Vec<EsiMarketOrder>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    NotModified,
}

pub trait CommerceSource {
    fn market_orders(
        &self,
        client_id: &str,
        character_id: i64,
        etag: Option<&str>,
    ) -> Result<MarketFetch, String>;
    fn contracts(
        &self,
        client_id: &str,
        character_id: i64,
    ) -> Result<(Vec<EsiContract>, u32), String>;
}

pub struct LiveCommerceSource;

fn refreshed(client_id: &str, character_id: i64) -> Result<(reqwest::Client, String), String> {
    let refresh = crate::esi::token_store::get_refresh_token(character_id)?;
    let http = reqwest::Client::new();
    let tokens = tauri::async_runtime::block_on(crate::esi::client::refresh_access_token(
        &http, client_id, &refresh,
    ))?;
    if tokens.expires_in <= 0 {
        return Err("token refresh returned a non-positive expiry".into());
    }
    crate::esi::token_store::store_refresh_token(character_id, &tokens.refresh_token)?;
    Ok((http, tokens.access_token))
}

fn send(
    http: &reqwest::Client,
    url: String,
    token: &str,
    etag: Option<&str>,
    context: &str,
) -> Result<(reqwest::StatusCode, reqwest::header::HeaderMap, String), String> {
    let mut request = http
        .get(url)
        .bearer_auth(token)
        .header("X-Compatibility-Date", COMPATIBILITY_DATE)
        .header(
            "User-Agent",
            concat!("RenderNorthIndustrial/", env!("CARGO_PKG_VERSION")),
        );
    if let Some(value) = etag {
        request = request.header(IF_NONE_MATCH, value)
    }
    let response = tauri::async_runtime::block_on(request.send())
        .map_err(|e| format!("{context} request failed: {e}"))?;
    let status = response.status();
    let headers = response.headers().clone();
    let body = tauri::async_runtime::block_on(response.text())
        .map_err(|e| format!("failed to read {context}: {e}"))?;
    if status != reqwest::StatusCode::NOT_MODIFIED && !status.is_success() {
        return Err(format!("{context} returned {status}: {body}"));
    }
    Ok((status, headers, body))
}

impl CommerceSource for LiveCommerceSource {
    fn market_orders(
        &self,
        client_id: &str,
        character_id: i64,
        etag: Option<&str>,
    ) -> Result<MarketFetch, String> {
        let (http, token) = refreshed(client_id, character_id)?;
        let (status, headers, body) = send(
            &http,
            format!("{ESI_BASE}/characters/{character_id}/orders"),
            &token,
            etag,
            "character market orders",
        )?;
        if status == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(MarketFetch::NotModified);
        }
        let rows = serde_json::from_str(&body)
            .map_err(|e| format!("failed to parse character market orders: {e}"))?;
        Ok(MarketFetch::Modified {
            rows,
            etag: headers
                .get(ETAG)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
            last_modified: headers
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
        })
    }
    fn contracts(
        &self,
        client_id: &str,
        character_id: i64,
    ) -> Result<(Vec<EsiContract>, u32), String> {
        let (http, token) = refreshed(client_id, character_id)?;
        let mut rows = Vec::new();
        let mut page = 1;
        let pages = loop {
            let (_, headers, body) = send(
                &http,
                format!("{ESI_BASE}/characters/{character_id}/contracts?page={page}"),
                &token,
                None,
                &format!("character contracts page {page}"),
            )?;
            let mut batch: Vec<EsiContract> = serde_json::from_str(&body)
                .map_err(|e| format!("failed to parse character contracts page {page}: {e}"))?;
            rows.append(&mut batch);
            let total = headers
                .get("x-pages")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1)
                .max(1);
            if page >= total {
                break total;
            }
            page += 1
        };
        Ok((rows, pages))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub character_id: i64,
    pub status: String,
    pub count: i64,
    pub page_count: i64,
    pub error: Option<String>,
}

fn has_scope(conn: &Connection, character_id: i64, scope: &str) -> Result<bool, String> {
    let value: String = conn
        .query_row(
            "SELECT scopes_granted FROM characters WHERE character_id=?1 AND is_demo=0",
            [character_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("character {character_id} is not connected"))?;
    Ok(value.split_whitespace().any(|v| v == scope))
}
fn decimal(value: Option<&Number>) -> Option<String> {
    value.map(ToString::to_string)
}

pub fn sync_market_one(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
) -> Result<SyncResult, String> {
    sync_market_one_with(conn, client_id, character_id, &LiveCommerceSource)
}
pub fn sync_contracts_one(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
) -> Result<SyncResult, String> {
    sync_contracts_one_with(conn, client_id, character_id, &LiveCommerceSource)
}
pub fn sync_market_all(conn: &Connection, client_id: &str) -> Result<Vec<SyncResult>, String> {
    sync_all(conn, client_id, true, &LiveCommerceSource)
}
pub fn sync_contracts_all(conn: &Connection, client_id: &str) -> Result<Vec<SyncResult>, String> {
    sync_all(conn, client_id, false, &LiveCommerceSource)
}
pub fn sync_all_commerce(conn: &Connection, client_id: &str) -> Result<Vec<SyncResult>, String> {
    let mut out = sync_market_all(conn, client_id)?;
    out.extend(sync_contracts_all(conn, client_id)?);
    Ok(out)
}

pub fn sync_market_one_with<S: CommerceSource>(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
    source: &S,
) -> Result<SyncResult, String> {
    if !has_scope(conn, character_id, MARKET_SCOPE)? {
        return state_error(
            conn,
            "character_market_order_sync_state",
            character_id,
            format!(
                "reauthorization required: Add / Reauthorize Character must grant {MARKET_SCOPE}"
            ),
        );
    }
    conn.execute("INSERT INTO character_market_order_sync_state(character_id,status,last_attempt_at,last_error) VALUES(?1,'syncing',strftime('%Y-%m-%dT%H:%M:%SZ','now'),NULL) ON CONFLICT(character_id) DO UPDATE SET status='syncing',last_attempt_at=excluded.last_attempt_at,last_error=NULL",[character_id]).map_err(|e|e.to_string())?;
    let etag: Option<String> = conn
        .query_row(
            "SELECT etag FROM character_market_order_sync_state WHERE character_id=?1",
            [character_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let fetched = match source.market_orders(client_id, character_id, etag.as_deref()) {
        Ok(v) => v,
        Err(e) => return state_error(conn, "character_market_order_sync_state", character_id, e),
    };
    if matches!(fetched, MarketFetch::NotModified) {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM character_market_orders WHERE character_id=?1",
                [character_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        conn.execute("UPDATE character_market_order_sync_state SET status=?2,last_success_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'),order_count=?3,page_count=1,last_error=NULL WHERE character_id=?1",params![character_id,if count==0{"empty"}else{"success"},count]).map_err(|e|e.to_string())?;
        return Ok(SyncResult {
            character_id,
            status: if count == 0 { "empty" } else { "success" }.into(),
            count,
            page_count: 1,
            error: None,
        });
    }
    let MarketFetch::Modified {
        rows,
        etag,
        last_modified,
    } = fetched
    else {
        unreachable!()
    };
    let rows = rows
        .into_iter()
        .filter(|row| !row.is_corporation)
        .collect::<Vec<_>>();
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "DELETE FROM character_market_orders WHERE character_id=?1",
            [character_id],
        )
        .map_err(|e| e.to_string())?;
        for row in &rows {
            conn.execute("INSERT INTO character_market_orders(character_id,order_id,type_id,is_buy_order,is_corporation,location_id,region_id,price_isk,volume_total,volume_remain,min_volume,issued_at,duration_days,order_range,escrow_isk,source,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",params![character_id,row.order_id,row.type_id,row.is_buy_order as i64,row.is_corporation as i64,row.location_id,row.region_id,row.price.to_string(),row.volume_total,row.volume_remain,row.min_volume,&row.issued,row.duration,&row.range,decimal(row.escrow.as_ref()),SOURCE_ORDERS]).map_err(|e|format!("failed to persist market order {}: {e}",row.order_id))?;
        }
        conn.execute("UPDATE character_market_order_sync_state SET status=?2,last_success_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'),order_count=?3,page_count=1,etag=?4,last_modified=?5,last_error=NULL WHERE character_id=?1",params![character_id,if rows.is_empty(){"empty"}else{"success"},rows.len() as i64,etag,last_modified]).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = conn.execute_batch("ROLLBACK;");
        return state_error(conn, "character_market_order_sync_state", character_id, e);
    }
    conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
    Ok(SyncResult {
        character_id,
        status: if rows.is_empty() { "empty" } else { "success" }.into(),
        count: rows.len() as i64,
        page_count: 1,
        error: None,
    })
}

pub fn sync_contracts_one_with<S: CommerceSource>(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
    source: &S,
) -> Result<SyncResult, String> {
    if !has_scope(conn, character_id, CONTRACT_SCOPE)? {
        return state_error(
            conn,
            "character_contract_sync_state",
            character_id,
            format!(
                "reauthorization required: Add / Reauthorize Character must grant {CONTRACT_SCOPE}"
            ),
        );
    }
    conn.execute("INSERT INTO character_contract_sync_state(character_id,status,last_attempt_at,last_error) VALUES(?1,'syncing',strftime('%Y-%m-%dT%H:%M:%SZ','now'),NULL) ON CONFLICT(character_id) DO UPDATE SET status='syncing',last_attempt_at=excluded.last_attempt_at,last_error=NULL",[character_id]).map_err(|e|e.to_string())?;
    let (rows, pages) = match source.contracts(client_id, character_id) {
        Ok(v) => v,
        Err(e) => return state_error(conn, "character_contract_sync_state", character_id, e),
    };
    let rows = rows
        .into_iter()
        .filter(|row| !row.for_corporation)
        .collect::<Vec<_>>();
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "DELETE FROM character_contracts WHERE character_id=?1",
            [character_id],
        )
        .map_err(|e| e.to_string())?;
        for row in &rows {
            conn.execute("INSERT INTO character_contracts(character_id,contract_id,issuer_id,issuer_corporation_id,assignee_id,acceptor_id,contract_type,availability,status,title,date_issued,date_expired,date_accepted,date_completed,start_location_id,end_location_id,price_isk,reward_isk,collateral_isk,buyout_isk,volume_m3,days_to_complete,for_corporation,source,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",params![character_id,row.contract_id,row.issuer_id,row.issuer_corporation_id,row.assignee_id,row.acceptor_id,&row.contract_type,&row.availability,&row.status,&row.title,&row.date_issued,&row.date_expired,&row.date_accepted,&row.date_completed,row.start_location_id,row.end_location_id,decimal(row.price.as_ref()),decimal(row.reward.as_ref()),decimal(row.collateral.as_ref()),decimal(row.buyout.as_ref()),decimal(row.volume.as_ref()),row.days_to_complete,row.for_corporation as i64,SOURCE_CONTRACTS]).map_err(|e|format!("failed to persist contract {}: {e}",row.contract_id))?;
        }
        conn.execute("UPDATE character_contract_sync_state SET status=?2,last_success_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'),contract_count=?3,page_count=?4,last_error=NULL WHERE character_id=?1",params![character_id,if rows.is_empty(){"empty"}else{"success"},rows.len() as i64,pages as i64]).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = conn.execute_batch("ROLLBACK;");
        return state_error(conn, "character_contract_sync_state", character_id, e);
    }
    conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
    Ok(SyncResult {
        character_id,
        status: if rows.is_empty() { "empty" } else { "success" }.into(),
        count: rows.len() as i64,
        page_count: pages as i64,
        error: None,
    })
}

fn state_error(
    conn: &Connection,
    table: &str,
    character_id: i64,
    error: String,
) -> Result<SyncResult, String> {
    let count_column = if table == "character_market_order_sync_state" {
        "order_count"
    } else {
        "contract_count"
    };
    let sql=format!("INSERT INTO {table}(character_id,status,last_attempt_at,last_error) VALUES(?1,'error',strftime('%Y-%m-%dT%H:%M:%SZ','now'),?2) ON CONFLICT(character_id) DO UPDATE SET status='error',last_attempt_at=excluded.last_attempt_at,last_error=excluded.last_error");
    conn.execute(&sql, params![character_id, &error])
        .map_err(|e| e.to_string())?;
    let count: i64 = conn
        .query_row(
            &format!("SELECT {count_column} FROM {table} WHERE character_id=?1"),
            [character_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(SyncResult {
        character_id,
        status: "error".into(),
        count,
        page_count: 0,
        error: Some(error),
    })
}
fn sync_all<S: CommerceSource>(
    conn: &Connection,
    client_id: &str,
    market: bool,
    source: &S,
) -> Result<Vec<SyncResult>, String> {
    let mut stmt=conn.prepare("SELECT character_id FROM characters WHERE is_demo=0 AND enabled=1 ORDER BY character_id").map_err(|e|e.to_string())?;
    let ids = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(ids
        .into_iter()
        .map(|id| {
            let result = if market {
                sync_market_one_with(conn, client_id, id, source)
            } else {
                sync_contracts_one_with(conn, client_id, id, source)
            };
            result.unwrap_or_else(|e| SyncResult {
                character_id: id,
                status: "error".into(),
                count: 0,
                page_count: 0,
                error: Some(e),
            })
        })
        .collect())
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|v| v.with_timezone(&Utc))
}
fn expires(issued: &str, duration: i64) -> Option<String> {
    parse_time(issued).map(|v| (v + Duration::days(duration)).to_rfc3339())
}
fn remaining_seconds(expiration: &str, now: DateTime<Utc>) -> i64 {
    parse_time(expiration)
        .map(|v| (v - now).num_seconds())
        .unwrap_or(0)
}
fn filled(total: i64, remain: i64) -> i64 {
    total.saturating_sub(remain).max(0)
}
fn fill_percent(total: i64, remain: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        filled(total, remain) as f64 * 100.0 / total as f64
    }
}
fn decimal_to_cents(value: &str) -> i64 {
    let negative = value.starts_with('-');
    let clean = value.trim_start_matches('-');
    let mut parts = clean.split('.');
    let whole = parts.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
    let fraction = parts.next().unwrap_or("");
    let cents = format!("{fraction:0<2}")
        .chars()
        .take(2)
        .collect::<String>()
        .parse::<i64>()
        .unwrap_or(0);
    let result = whole.saturating_mul(100).saturating_add(cents);
    if negative {
        -result
    } else {
        result
    }
}
fn cents_string(cents: i128) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let value = cents.abs();
    format!("{sign}{}.{:02}", value / 100, value % 100)
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrderFilter {
    pub character_id: Option<i64>,
    pub side: Option<String>,
    pub expiring_days: Option<i64>,
    pub location: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketOrderRow {
    pub order_id: i64,
    pub character_id: i64,
    pub character_name: String,
    pub type_id: i64,
    pub item_name: String,
    pub side: String,
    pub location_id: i64,
    pub location_name: String,
    pub solar_system_name: Option<String>,
    pub region_name: Option<String>,
    pub price_isk: String,
    pub volume_total: i64,
    pub volume_remain: i64,
    pub quantity_filled: i64,
    pub fill_percentage: f64,
    pub min_volume: Option<i64>,
    pub issued_at: String,
    pub expires_at: String,
    pub remaining_seconds: i64,
    pub duration_days: i64,
    pub order_range: String,
    pub escrow_isk: Option<String>,
    pub source: String,
    pub last_synced: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSummary {
    pub active_orders: i64,
    pub sell_orders: i64,
    pub buy_orders: i64,
    pub remaining_sell_value_isk: String,
    pub remaining_buy_commitment_isk: String,
    pub total_escrow_isk: String,
    pub expiring_soon: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDashboard {
    pub rows: Vec<MarketOrderRow>,
    pub summary: MarketSummary,
}

pub fn market_dashboard(
    conn: &Connection,
    filter: MarketOrderFilter,
) -> Result<MarketDashboard, String> {
    let now = Utc::now();
    let mut stmt=conn.prepare("SELECT o.order_id,o.character_id,c.name,o.type_id,COALESCE(t.name,'Type '||o.type_id),o.is_buy_order,o.location_id,COALESCE(l.display_name,'Unresolved Location (ID '||o.location_id||')'),l.solar_system_name,COALESCE(l.region_name,r.display_name),o.price_isk,o.volume_total,o.volume_remain,o.min_volume,o.issued_at,o.duration_days,o.order_range,o.escrow_isk,o.source,o.synced_at FROM character_market_orders o JOIN characters c ON c.character_id=o.character_id LEFT JOIN eve_types t ON t.type_id=o.type_id LEFT JOIN location_cache l ON l.location_id=o.location_id LEFT JOIN location_cache r ON r.location_id=o.region_id WHERE c.enabled=1 ORDER BY o.order_id").map_err(|e|e.to_string())?;
    let mapped = stmt
        .query_map([], |r| {
            let issued: String = r.get(14)?;
            let duration: i64 = r.get(15)?;
            let expiration = expires(&issued, duration).unwrap_or_else(|| issued.clone());
            let total: i64 = r.get(11)?;
            let remain: i64 = r.get(12)?;
            Ok(MarketOrderRow {
                order_id: r.get(0)?,
                character_id: r.get(1)?,
                character_name: r.get(2)?,
                type_id: r.get(3)?,
                item_name: r.get(4)?,
                side: if r.get::<_, i64>(5)? != 0 {
                    "Buy"
                } else {
                    "Sell"
                }
                .into(),
                location_id: r.get(6)?,
                location_name: r.get(7)?,
                solar_system_name: r.get(8)?,
                region_name: r.get(9)?,
                price_isk: r.get(10)?,
                volume_total: total,
                volume_remain: remain,
                quantity_filled: filled(total, remain),
                fill_percentage: fill_percent(total, remain),
                min_volume: r.get(13)?,
                issued_at: issued,
                expires_at: expiration.clone(),
                remaining_seconds: remaining_seconds(&expiration, now),
                duration_days: duration,
                order_range: r.get(16)?,
                escrow_isk: r.get(17)?,
                source: r.get(18)?,
                last_synced: r.get(19)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let threshold = filter.expiring_days.unwrap_or(7) * 86400;
    let query = filter.search.as_deref().unwrap_or("").to_ascii_lowercase();
    let location = filter
        .location
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    rows.retain(|r| {
        filter
            .character_id
            .map(|id| id == r.character_id)
            .unwrap_or(true)
            && filter
                .side
                .as_deref()
                .map(|s| s == "all" || s == "expiring" || s.eq_ignore_ascii_case(&r.side))
                .unwrap_or(true)
            && (query.is_empty()
                || r.item_name.to_ascii_lowercase().contains(&query)
                || r.character_name.to_ascii_lowercase().contains(&query))
            && (location.is_empty() || r.location_name.to_ascii_lowercase().contains(&location))
    });
    if filter.side.as_deref() == Some("expiring") {
        rows.retain(|r| r.remaining_seconds >= 0 && r.remaining_seconds <= threshold)
    }
    match filter.sort.as_deref().unwrap_or("expiring") {
        "value" => rows.sort_by_key(|r| {
            std::cmp::Reverse(decimal_to_cents(&r.price_isk).saturating_mul(r.volume_remain))
        }),
        "fill_high" => rows.sort_by(|a, b| b.fill_percentage.total_cmp(&a.fill_percentage)),
        "fill_low" => rows.sort_by(|a, b| a.fill_percentage.total_cmp(&b.fill_percentage)),
        "newest" => rows.sort_by(|a, b| b.issued_at.cmp(&a.issued_at)),
        "oldest" => rows.sort_by(|a, b| a.issued_at.cmp(&b.issued_at)),
        "item" => rows.sort_by(|a, b| a.item_name.cmp(&b.item_name)),
        "character" => rows.sort_by(|a, b| a.character_name.cmp(&b.character_name)),
        _ => rows.sort_by_key(|r| r.remaining_seconds),
    }
    let summary = market_summary(&rows, threshold);
    Ok(MarketDashboard { rows, summary })
}
fn market_summary(rows: &[MarketOrderRow], threshold: i64) -> MarketSummary {
    let mut sell = 0i128;
    let mut buy = 0i128;
    let mut escrow = 0i128;
    let (mut sells, mut buys, mut soon) = (0, 0, 0);
    for r in rows {
        let value = decimal_to_cents(&r.price_isk) as i128 * r.volume_remain as i128;
        if r.side == "Buy" {
            buys += 1;
            buy += value;
            escrow += r.escrow_isk.as_deref().map(decimal_to_cents).unwrap_or(0) as i128
        } else {
            sells += 1;
            sell += value
        }
        if r.remaining_seconds >= 0 && r.remaining_seconds <= threshold {
            soon += 1
        }
    }
    MarketSummary {
        active_orders: rows.len() as i64,
        sell_orders: sells,
        buy_orders: buys,
        remaining_sell_value_isk: cents_string(sell),
        remaining_buy_commitment_isk: cents_string(buy),
        total_escrow_isk: cents_string(escrow),
        expiring_soon: soon,
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContractFilter {
    pub character_id: Option<i64>,
    pub status: Option<String>,
    pub contract_type: Option<String>,
    pub availability: Option<String>,
    pub direction: Option<String>,
    pub start_location: Option<String>,
    pub end_location: Option<String>,
    pub search: Option<String>,
    pub expiring_days: Option<i64>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractRow {
    pub contract_id: i64,
    pub character_id: i64,
    pub character_name: String,
    pub title: String,
    pub contract_type: String,
    pub issuer_id: i64,
    pub issuer_corporation_id: i64,
    pub assignee_id: i64,
    pub acceptor_id: i64,
    pub direction: String,
    pub availability: String,
    pub status: String,
    pub start_location_id: Option<i64>,
    pub start_location_name: Option<String>,
    pub end_location_id: Option<i64>,
    pub end_location_name: Option<String>,
    pub price_isk: Option<String>,
    pub reward_isk: Option<String>,
    pub collateral_isk: Option<String>,
    pub buyout_isk: Option<String>,
    pub date_issued: String,
    pub date_expired: String,
    pub remaining_seconds: i64,
    pub source: String,
    pub last_synced: String,
    #[serde(skip)]
    pub search_text: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSummary {
    pub outstanding_contracts: i64,
    pub assigned_to_me: i64,
    pub issued_by_me: i64,
    pub in_progress: i64,
    pub expiring_soon: i64,
    pub completed_or_finished: i64,
    pub total_collateral_exposure_isk: String,
    pub outstanding_rewards_isk: String,
    pub outstanding_contract_value_isk: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDashboard {
    pub rows: Vec<ContractRow>,
    pub summary: ContractSummary,
}

pub fn contract_dashboard(
    conn: &Connection,
    filter: ContractFilter,
) -> Result<ContractDashboard, String> {
    let now = Utc::now();
    let mut stmt=conn.prepare("SELECT x.contract_id,x.character_id,c.name,x.title,x.contract_type,x.issuer_id,x.issuer_corporation_id,x.assignee_id,x.acceptor_id,x.availability,x.status,x.start_location_id,ls.display_name,x.end_location_id,le.display_name,x.price_isk,x.reward_isk,x.collateral_isk,x.buyout_isk,x.date_issued,x.date_expired,x.source,x.synced_at,COALESCE((SELECT group_concat(COALESCE(ti.name,'Type '||ci.type_id),' ') FROM character_contract_items ci LEFT JOIN eve_types ti ON ti.type_id=ci.type_id WHERE ci.character_id=x.character_id AND ci.contract_id=x.contract_id),'') FROM character_contracts x JOIN characters c ON c.character_id=x.character_id LEFT JOIN location_cache ls ON ls.location_id=x.start_location_id LEFT JOIN location_cache le ON le.location_id=x.end_location_id WHERE c.enabled=1 ORDER BY x.date_expired").map_err(|e|e.to_string())?;
    let mapped = stmt
        .query_map([], |r| {
            let cid: i64 = r.get(1)?;
            let issuer: i64 = r.get(5)?;
            let issuer_corporation_id: i64 = r.get(6)?;
            let assignee: i64 = r.get(7)?;
            let acceptor: i64 = r.get(8)?;
            let expires_at: String = r.get(20)?;
            let contract_id: i64 = r.get(0)?;
            Ok(ContractRow {
                contract_id,
                character_id: cid,
                character_name: r.get(2)?,
                title: r
                    .get::<_, Option<String>>(3)?
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| format!("Contract {contract_id}")),
                contract_type: r.get(4)?,
                issuer_id: issuer,
                issuer_corporation_id,
                assignee_id: assignee,
                acceptor_id: acceptor,
                direction: if issuer == cid {
                    "Issued"
                } else if acceptor == cid && acceptor != 0 {
                    "Accepted"
                } else if assignee == cid {
                    "Assigned"
                } else {
                    "Available"
                }
                .into(),
                availability: r.get(9)?,
                status: r.get(10)?,
                start_location_id: r.get(11)?,
                start_location_name: r.get(12)?,
                end_location_id: r.get(13)?,
                end_location_name: r.get(14)?,
                price_isk: r.get(15)?,
                reward_isk: r.get(16)?,
                collateral_isk: r.get(17)?,
                buyout_isk: r.get(18)?,
                date_issued: r.get(19)?,
                date_expired: expires_at.clone(),
                remaining_seconds: remaining_seconds(&expires_at, now),
                source: r.get(21)?,
                last_synced: r.get(22)?,
                search_text: r.get(23)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut rows = mapped
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let query = filter.search.as_deref().unwrap_or("").to_ascii_lowercase();
    let start_location = filter
        .start_location
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let end_location = filter
        .end_location
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let threshold = filter.expiring_days.unwrap_or(7) * 86400;
    rows.retain(|r| {
        filter
            .character_id
            .map(|id| id == r.character_id)
            .unwrap_or(true)
            && filter
                .status
                .as_deref()
                .map(|v| v == "all" || v == "expiring" || v == "expired" || v == r.status)
                .unwrap_or(true)
            && filter
                .contract_type
                .as_deref()
                .map(|v| v == "all" || v == r.contract_type)
                .unwrap_or(true)
            && filter
                .availability
                .as_deref()
                .map(|v| v == "all" || v == r.availability)
                .unwrap_or(true)
            && filter
                .direction
                .as_deref()
                .map(|v| v == "all" || v.eq_ignore_ascii_case(&r.direction))
                .unwrap_or(true)
            && (start_location.is_empty()
                || r.start_location_name
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&start_location)
                || r.start_location_id
                    .map(|id| id.to_string().contains(&start_location))
                    .unwrap_or(false))
            && (end_location.is_empty()
                || r.end_location_name
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&end_location)
                || r.end_location_id
                    .map(|id| id.to_string().contains(&end_location))
                    .unwrap_or(false))
            && (query.is_empty()
                || r.title.to_ascii_lowercase().contains(&query)
                || r.character_name.to_ascii_lowercase().contains(&query)
                || r.search_text.to_ascii_lowercase().contains(&query))
    });
    if filter.status.as_deref() == Some("expiring") {
        rows.retain(|r| r.remaining_seconds >= 0 && r.remaining_seconds <= threshold)
    } else if filter.status.as_deref() == Some("expired") {
        rows.retain(|r| r.remaining_seconds < 0)
    }
    let summary = contract_summary(&rows, threshold);
    Ok(ContractDashboard { rows, summary })
}
fn contract_summary(rows: &[ContractRow], threshold: i64) -> ContractSummary {
    let (mut outstanding, mut assigned, mut issued, mut progress, mut soon, mut finished) =
        (0, 0, 0, 0, 0, 0);
    let (mut collateral, mut rewards, mut value) = (0i128, 0i128, 0i128);
    for r in rows {
        let active = r.status == "outstanding" || r.status == "in_progress";
        if r.status == "outstanding" {
            outstanding += 1
        }
        if active {
            collateral += r
                .collateral_isk
                .as_deref()
                .map(decimal_to_cents)
                .unwrap_or(0) as i128;
            rewards += r.reward_isk.as_deref().map(decimal_to_cents).unwrap_or(0) as i128;
            value += r.price_isk.as_deref().map(decimal_to_cents).unwrap_or(0) as i128
        }
        if r.direction == "Assigned" {
            assigned += 1
        }
        if r.direction == "Issued" {
            issued += 1
        }
        if r.status == "in_progress" {
            progress += 1
        }
        if r.remaining_seconds >= 0 && r.remaining_seconds <= threshold {
            soon += 1
        }
        if r.status.starts_with("finished") {
            finished += 1
        }
    }
    ContractSummary {
        outstanding_contracts: outstanding,
        assigned_to_me: assigned,
        issued_by_me: issued,
        in_progress: progress,
        expiring_soon: soon,
        completed_or_finished: finished,
        total_collateral_exposure_isk: cents_string(collateral),
        outstanding_rewards_isk: cents_string(rewards),
        outstanding_contract_value_isk: cents_string(value),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommerceSyncOverview {
    pub status: String,
    pub oldest_success_at: Option<String>,
    pub relevant_characters: i64,
    pub current_characters: i64,
    pub error_characters: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommerceOverview {
    pub remaining_sell_value_isk: String,
    pub remaining_buy_commitment_isk: String,
    pub outstanding_contract_value_isk: String,
    pub collateral_exposure_isk: String,
    pub total_commerce_exposure_isk: String,
    pub market_orders_sync: CommerceSyncOverview,
    pub contracts_sync: CommerceSyncOverview,
}

fn total_commerce_exposure_cents(
    remaining_sell_value_isk: &str,
    remaining_buy_commitment_isk: &str,
    outstanding_contract_value_isk: &str,
    collateral_exposure_isk: &str,
) -> i128 {
    [
        remaining_sell_value_isk,
        remaining_buy_commitment_isk,
        outstanding_contract_value_isk,
        collateral_exposure_isk,
    ]
    .into_iter()
    .map(|value| decimal_to_cents(value) as i128)
    .sum()
}

fn aggregate_sync_rows(rows: Vec<(String, Option<String>, bool)>) -> CommerceSyncOverview {
    let relevant_characters = rows.len() as i64;
    let current_characters = rows
        .iter()
        .filter(|(_, last_success, _)| last_success.is_some())
        .count() as i64;
    let error_characters = rows.iter().filter(|(_, _, has_error)| *has_error).count() as i64;
    let syncing = rows.iter().any(|(status, _, _)| status == "syncing");
    let oldest_success_at = rows
        .iter()
        .filter_map(|(_, last_success, _)| last_success.as_ref())
        .min()
        .cloned();
    let status = if relevant_characters == 0 || (current_characters == 0 && error_characters == 0 && !syncing) {
        "never"
    } else if syncing {
        "syncing"
    } else if current_characters > 0 && (current_characters < relevant_characters || error_characters > 0) {
        "partial"
    } else if error_characters > 0 {
        "error"
    } else {
        "current"
    };
    CommerceSyncOverview {
        status: status.into(),
        oldest_success_at,
        relevant_characters,
        current_characters,
        error_characters,
    }
}

fn aggregate_sync_state(
    conn: &Connection,
    table: &str,
    character_id: Option<i64>,
) -> Result<CommerceSyncOverview, String> {
    let sql = format!(
        "SELECT COALESCE(s.status,'never'),s.last_success_at,s.last_error IS NOT NULL \
         FROM characters c LEFT JOIN {table} s ON s.character_id=c.character_id \
         WHERE c.enabled=1 AND c.is_demo=0 AND (?1 IS NULL OR c.character_id=?1) \
         ORDER BY c.character_id"
    );
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![character_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, i64>(2)? != 0,
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(aggregate_sync_rows(rows))
}

pub fn commerce_overview(
    conn: &Connection,
    character_id: Option<i64>,
    expiring_days: Option<i64>,
) -> Result<CommerceOverview, String> {
    let market = market_dashboard(
        conn,
        MarketOrderFilter {
            character_id,
            expiring_days,
            ..Default::default()
        },
    )?;
    let contracts = contract_dashboard(
        conn,
        ContractFilter {
            character_id,
            expiring_days,
            ..Default::default()
        },
    )?;
    let total = total_commerce_exposure_cents(
        &market.summary.remaining_sell_value_isk,
        &market.summary.remaining_buy_commitment_isk,
        &contracts.summary.outstanding_contract_value_isk,
        &contracts.summary.total_collateral_exposure_isk,
    );
    Ok(CommerceOverview {
        remaining_sell_value_isk: market.summary.remaining_sell_value_isk,
        remaining_buy_commitment_isk: market.summary.remaining_buy_commitment_isk,
        outstanding_contract_value_isk: contracts.summary.outstanding_contract_value_isk,
        collateral_exposure_isk: contracts.summary.total_collateral_exposure_isk,
        total_commerce_exposure_isk: cents_string(total),
        market_orders_sync: aggregate_sync_state(
            conn,
            "character_market_order_sync_state",
            character_id,
        )?,
        contracts_sync: aggregate_sync_state(
            conn,
            "character_contract_sync_state",
            character_id,
        )?,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractItemRow {
    pub record_id: i64,
    pub type_id: i64,
    pub item_name: String,
    pub quantity: i64,
    pub raw_quantity: Option<i64>,
    pub singleton: bool,
    pub included: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBidRow {
    pub bid_id: i64,
    pub bidder_id: i64,
    pub amount_isk: String,
    pub date_bid: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDetail {
    pub contract: ContractRow,
    pub items: Vec<ContractItemRow>,
    pub bids: Vec<ContractBidRow>,
    pub items_error: Option<String>,
    pub bids_error: Option<String>,
}

fn fetch_detail_live(
    client_id: &str,
    character_id: i64,
    contract_id: i64,
    kind: &str,
) -> Result<String, String> {
    let (http, token) = refreshed(client_id, character_id)?;
    let (_, _, body) = send(
        &http,
        format!("{ESI_BASE}/characters/{character_id}/contracts/{contract_id}/{kind}"),
        &token,
        None,
        &format!("contract {kind}"),
    )?;
    Ok(body)
}

fn replace_contract_items(
    conn: &Connection,
    character_id: i64,
    contract_id: i64,
    rows: Vec<EsiContractItem>,
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM character_contract_items WHERE character_id=?1 AND contract_id=?2",
        params![character_id, contract_id],
    )
    .map_err(|error| error.to_string())?;
    for row in rows {
        tx.execute(
            "INSERT INTO character_contract_items(character_id,contract_id,record_id,type_id,quantity,raw_quantity,is_singleton,is_included,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            params![character_id, contract_id, row.record_id, row.type_id, row.quantity, row.raw_quantity, row.is_singleton as i64, row.is_included as i64],
        ).map_err(|error| error.to_string())?;
    }
    tx.execute(
        "INSERT INTO character_contract_detail_state(character_id,contract_id,items_status,items_loaded_at,items_error) VALUES(?1,?2,'success',strftime('%Y-%m-%dT%H:%M:%SZ','now'),NULL) ON CONFLICT(character_id,contract_id) DO UPDATE SET items_status='success',items_loaded_at=excluded.items_loaded_at,items_error=NULL",
        params![character_id, contract_id],
    ).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

fn replace_contract_bids(
    conn: &Connection,
    character_id: i64,
    contract_id: i64,
    rows: Vec<EsiContractBid>,
) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    tx.execute(
        "DELETE FROM character_contract_bids WHERE character_id=?1 AND contract_id=?2",
        params![character_id, contract_id],
    )
    .map_err(|error| error.to_string())?;
    for row in rows {
        tx.execute(
            "INSERT INTO character_contract_bids(character_id,contract_id,bid_id,bidder_id,amount_isk,date_bid,synced_at) VALUES(?1,?2,?3,?4,?5,?6,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            params![character_id, contract_id, row.bid_id, row.bidder_id, row.amount.to_string(), row.date_bid],
        ).map_err(|error| error.to_string())?;
    }
    tx.execute(
        "INSERT INTO character_contract_detail_state(character_id,contract_id,bids_status,bids_loaded_at,bids_error) VALUES(?1,?2,'success',strftime('%Y-%m-%dT%H:%M:%SZ','now'),NULL) ON CONFLICT(character_id,contract_id) DO UPDATE SET bids_status='success',bids_loaded_at=excluded.bids_loaded_at,bids_error=NULL",
        params![character_id, contract_id],
    ).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

pub fn contract_detail(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
    contract_id: i64,
) -> Result<ContractDetail, String> {
    let contract = contract_dashboard(
        conn,
        ContractFilter {
            character_id: Some(character_id),
            ..Default::default()
        },
    )?
    .rows
    .into_iter()
    .find(|row| row.contract_id == contract_id)
    .ok_or_else(|| {
        format!("contract {contract_id} is not synchronized for character {character_id}")
    })?;
    let fresh = conn.query_row(
        "SELECT 1 FROM character_contract_detail_state WHERE character_id=?1 AND contract_id=?2 AND items_loaded_at>=datetime('now','-30 minutes')",
        params![character_id, contract_id],
        |row| row.get::<_, i64>(0),
    ).is_ok();

    let mut items_error = None;
    let mut bids_error = None;
    if !fresh {
        let items_result = fetch_detail_live(client_id, character_id, contract_id, "items")
            .and_then(|body| {
                serde_json::from_str::<Vec<EsiContractItem>>(&body)
                    .map_err(|error| format!("contract items parse failed: {error}"))
            })
            .and_then(|rows| replace_contract_items(conn, character_id, contract_id, rows));
        if let Err(error) = items_result {
            items_error = Some(error);
        }

        if contract.contract_type == "auction" {
            let bids_result = fetch_detail_live(client_id, character_id, contract_id, "bids")
                .and_then(|body| {
                    serde_json::from_str::<Vec<EsiContractBid>>(&body)
                        .map_err(|error| format!("contract bids parse failed: {error}"))
                })
                .and_then(|rows| replace_contract_bids(conn, character_id, contract_id, rows));
            if let Err(error) = bids_result {
                bids_error = Some(error);
            }
        }
    }

    if let Some(error) = &items_error {
        let _ = conn.execute(
            "INSERT INTO character_contract_detail_state(character_id,contract_id,items_status,items_error) VALUES(?1,?2,'error',?3) ON CONFLICT(character_id,contract_id) DO UPDATE SET items_status='error',items_error=excluded.items_error",
            params![character_id, contract_id, error],
        );
    }
    if let Some(error) = &bids_error {
        let _ = conn.execute(
            "INSERT INTO character_contract_detail_state(character_id,contract_id,bids_status,bids_error) VALUES(?1,?2,'error',?3) ON CONFLICT(character_id,contract_id) DO UPDATE SET bids_status='error',bids_error=excluded.bids_error",
            params![character_id, contract_id, error],
        );
    }

    let mut item_statement = conn.prepare("SELECT i.record_id,i.type_id,COALESCE(t.name,'Type '||i.type_id),i.quantity,i.raw_quantity,i.is_singleton,i.is_included FROM character_contract_items i LEFT JOIN eve_types t ON t.type_id=i.type_id WHERE i.character_id=?1 AND i.contract_id=?2 ORDER BY i.is_included DESC,t.name").map_err(|error| error.to_string())?;
    let items = item_statement
        .query_map(params![character_id, contract_id], |row| {
            Ok(ContractItemRow {
                record_id: row.get(0)?,
                type_id: row.get(1)?,
                item_name: row.get(2)?,
                quantity: row.get(3)?,
                raw_quantity: row.get(4)?,
                singleton: row.get::<_, i64>(5)? != 0,
                included: row.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let mut bid_statement = conn.prepare("SELECT bid_id,bidder_id,amount_isk,date_bid FROM character_contract_bids WHERE character_id=?1 AND contract_id=?2 ORDER BY amount_isk DESC").map_err(|error| error.to_string())?;
    let bids = bid_statement
        .query_map(params![character_id, contract_id], |row| {
            Ok(ContractBidRow {
                bid_id: row.get(0)?,
                bidder_id: row.get(1)?,
                amount_isk: row.get(2)?,
                date_bid: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(ContractDetail {
        contract,
        items,
        bids,
        items_error,
        bids_error,
    })
}

pub fn safe_diagnostics(conn: &Connection) -> Result<serde_json::Value, String> {
    let rows=conn.prepare("SELECT c.character_id,ms.status,ms.order_count,ms.page_count,ms.last_success_at,ms.last_error IS NOT NULL,cs.status,cs.contract_count,cs.page_count,cs.last_success_at,cs.last_error IS NOT NULL FROM characters c LEFT JOIN character_market_order_sync_state ms ON ms.character_id=c.character_id LEFT JOIN character_contract_sync_state cs ON cs.character_id=c.character_id WHERE c.is_demo=0 ORDER BY c.character_id").map_err(|e|e.to_string())?.query_map([],|r|Ok(serde_json::json!({"characterId":r.get::<_,i64>(0)?,"marketOrders":{"status":r.get::<_,Option<String>>(1)?,"count":r.get::<_,Option<i64>>(2)?.unwrap_or(0),"pages":r.get::<_,Option<i64>>(3)?.unwrap_or(0),"lastSync":r.get::<_,Option<String>>(4)?,"hasError":r.get::<_,i64>(5)?!=0},"contracts":{"status":r.get::<_,Option<String>>(6)?,"count":r.get::<_,Option<i64>>(7)?.unwrap_or(0),"pages":r.get::<_,Option<i64>>(8)?.unwrap_or(0),"lastSync":r.get::<_,Option<String>>(9)?,"hasError":r.get::<_,i64>(10)?!=0}}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(
        serde_json::json!({"capabilities":[MARKET_SCOPE,CONTRACT_SCOPE],"characters":rows,"privacy":"No order rows, contract contents, counterparties, items, prices, rewards, collateral, locations, raw errors, tokens, or credentials are exported."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE characters(character_id INTEGER PRIMARY KEY,name TEXT,is_demo INTEGER,scopes_granted TEXT,enabled INTEGER);CREATE TABLE eve_types(type_id INTEGER PRIMARY KEY,name TEXT);CREATE TABLE location_cache(location_id INTEGER PRIMARY KEY,display_name TEXT,solar_system_name TEXT,region_name TEXT);INSERT INTO eve_types VALUES(34,'Tritanium');INSERT INTO location_cache VALUES(600,'Jita 4-4','Jita','The Forge');INSERT INTO location_cache VALUES(601,'Perimeter Trade Hub','Perimeter','The Forge');").unwrap();
        c.execute_batch(include_str!("../migrations/0021_personal_commerce.sql"))
            .unwrap();
        c
    }
    fn add(c: &Connection, id: i64, enabled: bool, scopes: &str) {
        c.execute(
            "INSERT INTO characters VALUES(?1,?2,0,?3,?4)",
            params![id, format!("Pilot {id}"), scopes, enabled as i64],
        )
        .unwrap();
    }
    fn order(id: i64, buy: bool, total: i64, remain: i64) -> EsiMarketOrder {
        EsiMarketOrder {
            duration: 30,
            escrow: if buy { Some(Number::from(500)) } else { None },
            is_buy_order: buy,
            is_corporation: false,
            issued: "2026-07-01T00:00:00Z".into(),
            location_id: 600,
            min_volume: Some(1),
            order_id: id,
            price: Number::from(10),
            range: "station".into(),
            region_id: 10,
            type_id: 34,
            volume_remain: remain,
            volume_total: total,
        }
    }
    fn contract(id: i64, status: &str, typ: &str) -> EsiContract {
        EsiContract {
            acceptor_id: 0,
            assignee_id: 1,
            availability: "personal".into(),
            buyout: Some(Number::from(20)),
            collateral: Some(Number::from(30)),
            contract_id: id,
            date_accepted: None,
            date_completed: None,
            date_expired: "2026-08-01T00:00:00Z".into(),
            date_issued: "2026-07-01T00:00:00Z".into(),
            days_to_complete: Some(3),
            end_location_id: Some(601),
            for_corporation: false,
            issuer_corporation_id: 9,
            issuer_id: 2,
            price: Some(Number::from(40)),
            reward: Some(Number::from(50)),
            start_location_id: Some(600),
            status: status.into(),
            title: Some("Test".into()),
            contract_type: typ.into(),
            volume: Some(Number::from(5)),
        }
    }
    struct Fake {
        markets: HashMap<i64, Result<MarketFetch, String>>,
        contracts: HashMap<i64, Result<(Vec<EsiContract>, u32), String>>,
    }
    impl CommerceSource for Fake {
        fn market_orders(&self, _: &str, id: i64, _: Option<&str>) -> Result<MarketFetch, String> {
            self.markets.get(&id).unwrap().clone()
        }
        fn contracts(&self, _: &str, id: i64) -> Result<(Vec<EsiContract>, u32), String> {
            self.contracts.get(&id).unwrap().clone()
        }
    }
    #[test]
    fn market_orders_remain_separate_by_character_and_side() {
        let c = db();
        let scopes = format!("{MARKET_SCOPE} {CONTRACT_SCOPE}");
        add(&c, 1, true, &scopes);
        add(&c, 2, true, &scopes);
        let f = Fake {
            markets: HashMap::from([
                (
                    1,
                    Ok(MarketFetch::Modified {
                        rows: vec![order(10, false, 10, 4)],
                        etag: None,
                        last_modified: None,
                    }),
                ),
                (
                    2,
                    Ok(MarketFetch::Modified {
                        rows: vec![order(10, true, 20, 5)],
                        etag: None,
                        last_modified: None,
                    }),
                ),
            ]),
            contracts: HashMap::new(),
        };
        sync_market_one_with(&c, "x", 1, &f).unwrap();
        sync_market_one_with(&c, "x", 2, &f).unwrap();
        assert_eq!(
            c.query_row(
                "SELECT COUNT(DISTINCT character_id) FROM character_market_orders",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        let d = market_dashboard(&c, MarketOrderFilter::default()).unwrap();
        assert_eq!((d.summary.sell_orders, d.summary.buy_orders), (1, 1));
    }
    #[test]
    fn derived_order_math_is_safe_and_exact() {
        assert_eq!(filled(10, 4), 6);
        assert_eq!(fill_percent(10, 4), 60.0);
        assert_eq!(fill_percent(0, 0), 0.0);
        assert_eq!(decimal_to_cents("123.45"), 12345);
        assert_eq!(cents_string(12345), "123.45");
    }
    #[test]
    fn failed_market_sync_preserves_snapshot_and_empty_is_success() {
        let c = db();
        add(&c, 1, true, MARKET_SCOPE);
        let ok = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![order(1, false, 1, 1)],
                    etag: None,
                    last_modified: None,
                }),
            )]),
            contracts: HashMap::new(),
        };
        sync_market_one_with(&c, "x", 1, &ok).unwrap();
        let bad = Fake {
            markets: HashMap::from([(1, Err("offline".into()))]),
            contracts: HashMap::new(),
        };
        assert_eq!(
            sync_market_one_with(&c, "x", 1, &bad).unwrap().status,
            "error"
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_market_orders", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let empty = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![],
                    etag: None,
                    last_modified: None,
                }),
            )]),
            contracts: HashMap::new(),
        };
        assert_eq!(
            sync_market_one_with(&c, "x", 1, &empty).unwrap().status,
            "empty"
        )
    }
    #[test]
    fn corporation_orders_are_excluded() {
        let c = db();
        add(&c, 1, true, MARKET_SCOPE);
        let mut row = order(1, false, 1, 1);
        row.is_corporation = true;
        let f = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![row],
                    etag: None,
                    last_modified: None,
                }),
            )]),
            contracts: HashMap::new(),
        };
        sync_market_one_with(&c, "x", 1, &f).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_market_orders", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        )
    }
    #[test]
    fn corporation_contracts_are_excluded() {
        let c = db();
        add(&c, 1, true, CONTRACT_SCOPE);
        let mut row = contract(1, "outstanding", "courier");
        row.for_corporation = true;
        let f = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([(1, Ok((vec![row], 1)))]),
        };
        let result = sync_contracts_one_with(&c, "x", 1, &f).unwrap();
        assert_eq!((result.status.as_str(), result.count), ("empty", 0));
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_contracts", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn contracts_persist_status_types_amounts_and_multiple_characters() {
        let c = db();
        add(&c, 1, true, CONTRACT_SCOPE);
        add(&c, 2, true, CONTRACT_SCOPE);
        let f = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([
                (1, Ok((vec![contract(1, "outstanding", "courier")], 2))),
                (2, Ok((vec![contract(1, "finished", "auction")], 1))),
            ]),
        };
        sync_contracts_one_with(&c, "x", 1, &f).unwrap();
        sync_contracts_one_with(&c, "x", 2, &f).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_contracts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        let values:(String,String,String,String)=c.query_row("SELECT price_isk,reward_isk,collateral_isk,buyout_isk FROM character_contracts WHERE character_id=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
        assert_eq!(values, ("40".into(), "50".into(), "30".into(), "20".into()))
    }
    #[test]
    fn failed_contract_sync_preserves_snapshot_and_missing_scope_is_precise() {
        let c = db();
        add(&c, 1, true, CONTRACT_SCOPE);
        let ok = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([(
                1,
                Ok((vec![contract(1, "outstanding", "item_exchange")], 1)),
            )]),
        };
        sync_contracts_one_with(&c, "x", 1, &ok).unwrap();
        let bad = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([(1, Err("offline".into()))]),
        };
        assert_eq!(
            sync_contracts_one_with(&c, "x", 1, &bad).unwrap().status,
            "error"
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_contracts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        add(&c, 2, true, "");
        assert!(sync_contracts_one_with(&c, "x", 2, &bad)
            .unwrap()
            .error
            .unwrap()
            .contains(CONTRACT_SCOPE))
    }
    #[test]
    fn expiration_and_market_summary_calculations_are_deterministic() {
        assert_eq!(
            expires("2026-07-01T00:00:00Z", 7).as_deref(),
            Some("2026-07-08T00:00:00+00:00")
        );
        assert_eq!(
            remaining_seconds(
                "2026-07-08T00:00:00Z",
                parse_time("2026-07-07T12:00:00Z").unwrap()
            ),
            43_200
        );
        let c = db();
        add(&c, 1, true, MARKET_SCOPE);
        let f = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![order(1, false, 10, 4), order(2, true, 20, 5)],
                    etag: Some("etag-1".into()),
                    last_modified: Some("date-1".into()),
                }),
            )]),
            contracts: HashMap::new(),
        };
        sync_market_one_with(&c, "x", 1, &f).unwrap();
        let dashboard = market_dashboard(&c, MarketOrderFilter::default()).unwrap();
        assert_eq!(dashboard.summary.remaining_sell_value_isk, "40.00");
        assert_eq!(dashboard.summary.remaining_buy_commitment_isk, "50.00");
        assert_eq!(dashboard.summary.total_escrow_isk, "500.00");
        assert_eq!(
            dashboard.rows[0].quantity_filled + dashboard.rows[0].volume_remain,
            dashboard.rows[0].volume_total
        );
    }
    #[test]
    fn commerce_exposure_uses_exact_active_components_without_double_counting() {
        let c = db();
        let scopes = format!("{MARKET_SCOPE} {CONTRACT_SCOPE}");
        add(&c, 1, true, &scopes);
        let source = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![order(1, false, 10, 4), order(2, true, 20, 5)],
                    etag: None,
                    last_modified: None,
                }),
            )]),
            contracts: HashMap::from([(
                1,
                Ok((
                    vec![
                        contract(1, "outstanding", "courier"),
                        contract(2, "finished", "item_exchange"),
                    ],
                    1,
                )),
            )]),
        };
        sync_market_one_with(&c, "x", 1, &source).unwrap();
        sync_contracts_one_with(&c, "x", 1, &source).unwrap();
        let overview = commerce_overview(&c, None, Some(7)).unwrap();
        assert_eq!(overview.remaining_sell_value_isk, "40.00");
        assert_eq!(overview.remaining_buy_commitment_isk, "50.00");
        assert_eq!(overview.outstanding_contract_value_isk, "40.00");
        assert_eq!(overview.collateral_exposure_isk, "30.00");
        assert_eq!(overview.total_commerce_exposure_isk, "160.00");
        assert_ne!(overview.total_commerce_exposure_isk, "710.00");
    }
    #[test]
    fn aggregate_sync_state_uses_oldest_success_and_reports_partial() {
        let overview = aggregate_sync_rows(vec![
            ("success".into(), Some("2026-07-18T12:00:00Z".into()), false),
            ("error".into(), Some("2026-07-17T11:00:00Z".into()), true),
            ("never".into(), None, false),
        ]);
        assert_eq!(overview.status, "partial");
        assert_eq!(overview.oldest_success_at.as_deref(), Some("2026-07-17T11:00:00Z"));
        assert_eq!(overview.relevant_characters, 3);
        assert_eq!(overview.current_characters, 2);
        assert_eq!(overview.error_characters, 1);
    }
    #[test]
    fn market_filters_and_sorts_include_expiring_and_character() {
        let c = db();
        add(&c, 2, true, MARKET_SCOPE);
        add(&c, 1, true, MARKET_SCOPE);
        let f = Fake {
            markets: HashMap::from([
                (
                    1,
                    Ok(MarketFetch::Modified {
                        rows: vec![order(1, false, 10, 4)],
                        etag: None,
                        last_modified: None,
                    }),
                ),
                (
                    2,
                    Ok(MarketFetch::Modified {
                        rows: vec![order(2, true, 10, 4)],
                        etag: None,
                        last_modified: None,
                    }),
                ),
            ]),
            contracts: HashMap::new(),
        };
        sync_market_one_with(&c, "x", 1, &f).unwrap();
        sync_market_one_with(&c, "x", 2, &f).unwrap();
        let buys = market_dashboard(
            &c,
            MarketOrderFilter {
                side: Some("Buy".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(buys.rows.len(), 1);
        let sorted = market_dashboard(
            &c,
            MarketOrderFilter {
                sort: Some("character".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(sorted.rows[0].character_name, "Pilot 1");
    }
    #[test]
    fn sync_all_excludes_disabled_characters_for_both_features() {
        let c = db();
        let scopes = format!("{MARKET_SCOPE} {CONTRACT_SCOPE}");
        add(&c, 1, true, &scopes);
        add(&c, 2, false, &scopes);
        let f = Fake {
            markets: HashMap::from([(
                1,
                Ok(MarketFetch::Modified {
                    rows: vec![],
                    etag: None,
                    last_modified: None,
                }),
            )]),
            contracts: HashMap::from([(1, Ok((vec![], 1)))]),
        };
        assert_eq!(sync_all(&c, "x", true, &f).unwrap().len(), 1);
        assert_eq!(sync_all(&c, "x", false, &f).unwrap().len(), 1);
    }
    #[test]
    fn successful_empty_contract_snapshot_is_not_an_error() {
        let c = db();
        add(&c, 1, true, CONTRACT_SCOPE);
        let f = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([(1, Ok((vec![], 1)))]),
        };
        let result = sync_contracts_one_with(&c, "x", 1, &f).unwrap();
        assert_eq!(
            (result.status.as_str(), result.count, result.page_count),
            ("empty", 0, 1)
        );
    }
    #[test]
    fn contract_direction_filters_items_and_bids_remain_separate() {
        let c = db();
        add(&c, 1, true, CONTRACT_SCOPE);
        let mut issued = contract(1, "outstanding", "item_exchange");
        issued.issuer_id = 1;
        issued.assignee_id = 0;
        issued.availability = "public".into();
        let mut accepted = contract(2, "in_progress", "auction");
        accepted.acceptor_id = 1;
        accepted.assignee_id = 0;
        accepted.availability = "corporation".into();
        let f = Fake {
            markets: HashMap::new(),
            contracts: HashMap::from([(1, Ok((vec![issued, accepted], 3)))]),
        };
        sync_contracts_one_with(&c, "x", 1, &f).unwrap();
        replace_contract_items(
            &c,
            1,
            1,
            vec![
                EsiContractItem {
                    is_included: true,
                    is_singleton: false,
                    quantity: 5,
                    raw_quantity: None,
                    record_id: 10,
                    type_id: 34,
                },
                EsiContractItem {
                    is_included: false,
                    is_singleton: true,
                    quantity: 1,
                    raw_quantity: Some(-1),
                    record_id: 11,
                    type_id: 34,
                },
            ],
        )
        .unwrap();
        replace_contract_bids(
            &c,
            1,
            2,
            vec![EsiContractBid {
                amount: Number::from(99),
                bid_id: 20,
                bidder_id: 88,
                date_bid: "2026-07-18T00:00:00Z".into(),
            }],
        )
        .unwrap();
        let issued_rows = contract_dashboard(
            &c,
            ContractFilter {
                direction: Some("Issued".into()),
                search: Some("Tritanium".into()),
                start_location: Some("Jita".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(issued_rows.rows.len(), 1);
        assert_eq!(issued_rows.rows[0].availability, "public");
        let offered: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM character_contract_items WHERE is_included=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let requested: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM character_contract_items WHERE is_included=0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let bids: i64 = c
            .query_row("SELECT COUNT(*) FROM character_contract_bids", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!((offered, requested, bids), (1, 1, 1));
    }
    #[test]
    fn transport_is_read_only_and_has_no_commerce_write_actions() {
        let source = include_str!("commerce.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for forbidden in ["http.post(", "http.put(", "http.patch(", "http.delete("] {
            assert!(!source.contains(forbidden));
        }
    }
    #[test]
    fn diagnostics_exclude_private_commerce_content() {
        let c = db();
        add(&c, 1, true, &format!("{MARKET_SCOPE} {CONTRACT_SCOPE}"));
        let text = safe_diagnostics(&c).unwrap().to_string();
        for forbidden in [
            "price_isk",
            "collateral_isk",
            "assignee_id",
            "access_token",
            "refresh_token",
        ] {
            assert!(!text.contains(forbidden))
        }
    }
    #[test]
    fn migration_is_additive_and_has_no_integration_tables() {
        let sql = include_str!("../migrations/0021_personal_commerce.sql");
        for forbidden in [
            "production_",
            "quartermaster",
            "procurement",
            "corporation_market",
            "corporation_contract",
        ] {
            assert!(!sql.contains(forbidden))
        }
        assert!(sql.contains("character_market_orders"));
        assert!(sql.contains("character_contracts"))
    }
}

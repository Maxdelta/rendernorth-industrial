use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
struct EsiOrder {
    order_id: i64,
    type_id: i64,
    is_buy_order: bool,
    price: f64,
    volume_remain: i64,
    #[serde(default = "one")]
    min_volume: i64,
    range: String,
    location_id: i64,
    system_id: i64,
}
fn one() -> i64 {
    1
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderLevel {
    pub price: f64,
    pub volume: i64,
}
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeightedFill {
    pub unit_price: Option<f64>,
    pub total_price: Option<f64>,
    pub requested: i64,
    pub filled: i64,
    pub available_volume: i64,
    pub sufficient: bool,
}

pub fn weighted_fill(mut orders: Vec<OrderLevel>, quantity: i64, ascending: bool) -> WeightedFill {
    orders.retain(|o| o.price > 0.0 && o.volume > 0);
    orders.sort_by(|a, b| {
        if ascending {
            a.price.total_cmp(&b.price)
        } else {
            b.price.total_cmp(&a.price)
        }
    });
    let available = orders.iter().map(|o| o.volume).sum();
    let mut remaining = quantity.max(0);
    let mut total = 0.0;
    let mut filled = 0;
    for o in orders {
        if remaining == 0 {
            break;
        }
        let take = remaining.min(o.volume);
        total += take as f64 * o.price;
        filled += take;
        remaining -= take;
    }
    let sufficient = quantity > 0 && filled == quantity;
    WeightedFill {
        unit_price: if filled > 0 {
            Some(total / filled as f64)
        } else {
            None
        },
        total_price: if filled > 0 { Some(total) } else { None },
        requested: quantity,
        filled,
        available_volume: available,
        sufficient,
    }
}
pub fn weighted_sell_fill(orders: Vec<OrderLevel>, quantity: i64) -> WeightedFill {
    weighted_fill(orders, quantity, true)
}
pub fn weighted_buy_fill(orders: Vec<OrderLevel>, quantity: i64) -> WeightedFill {
    weighted_fill(orders, quantity, false)
}

fn complete_total(fill: &WeightedFill) -> Option<f64> {
    if fill.sufficient {
        fill.total_price
    } else {
        None
    }
}

fn complete_unit_price(fill: &WeightedFill) -> Option<f64> {
    if fill.sufficient {
        fill.unit_price
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketProfile {
    pub profile_id: i64,
    pub display_name: String,
    pub region_id: i64,
    pub location_id: i64,
    pub refresh_interval_seconds: i64,
    pub status: String,
    pub fetched_at: Option<String>,
    pub expires_at: Option<String>,
    pub order_count: i64,
    pub page_count: i64,
    pub last_error: Option<String>,
    pub stale: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketQuote {
    pub type_id: i64,
    pub quantity: i64,
    pub unit_volume_m3: Option<f64>,
    pub requested_volume_m3: Option<f64>,
    pub acquisition: WeightedFill,
    pub liquidation: WeightedFill,
    pub market_profile: String,
    pub fetched_at: Option<String>,
    pub expires_at: Option<String>,
    pub stale: bool,
    pub source: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub order_count: i64,
    pub page_count: i64,
    pub fetched_at: String,
    pub expires_at: String,
}

pub fn selected_profile(conn: &Connection) -> Result<MarketProfile, String> {
    conn.query_row("SELECT p.profile_id,p.display_name,p.region_id,p.location_id,p.refresh_interval_seconds,COALESCE(s.status,'never'),s.fetched_at,s.expires_at,COALESCE(s.order_count,0),COALESCE(s.page_count,0),s.last_error,CASE WHEN s.expires_at IS NULL OR s.expires_at<=strftime('%Y-%m-%dT%H:%M:%SZ','now') THEN 1 ELSE 0 END FROM market_profiles p LEFT JOIN market_refresh_state s ON s.profile_id=p.profile_id WHERE p.is_selected=1 LIMIT 1",[],|r|Ok(MarketProfile{profile_id:r.get(0)?,display_name:r.get(1)?,region_id:r.get(2)?,location_id:r.get(3)?,refresh_interval_seconds:r.get(4)?,status:r.get(5)?,fetched_at:r.get(6)?,expires_at:r.get(7)?,order_count:r.get(8)?,page_count:r.get(9)?,last_error:r.get(10)?,stale:r.get::<_,i64>(11)?!=0})).map_err(|e|format!("selected market profile unavailable: {e}"))
}

pub fn quote(conn: &Connection, type_id: i64, quantity: i64) -> Result<MarketQuote, String> {
    let p = selected_profile(conn)?;
    let unit_volume_m3 = conn
        .query_row(
            "SELECT volume_m3 FROM eve_types WHERE type_id=?1",
            [type_id],
            |row| row.get::<_, Option<f64>>(0),
        )
        .optional()
        .map_err(|error| format!("failed to load CCP type volume for {type_id}: {error}"))?
        .flatten();
    let mut sell = Vec::new();
    let mut buy = Vec::new();
    let mut s=conn.prepare("SELECT is_buy_order,price,volume_remain,location_id,order_range FROM market_order_cache WHERE profile_id=?1 AND type_id=?2").map_err(|e|e.to_string())?;
    let rows = s
        .query_map((p.profile_id, type_id), |r| {
            Ok((
                r.get::<_, i64>(0)? != 0,
                r.get::<_, f64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (is_buy, price, volume, location, range) = row.map_err(|e| e.to_string())?;
        if !is_buy && location == p.location_id {
            sell.push(OrderLevel { price, volume })
        } else if is_buy && (location == p.location_id || range == "region") {
            buy.push(OrderLevel { price, volume })
        }
    }
    Ok(MarketQuote {
        type_id,
        quantity,
        unit_volume_m3,
        requested_volume_m3: crate::volume::volume_for_quantity(unit_volume_m3, quantity),
        acquisition: weighted_sell_fill(sell, quantity),
        liquidation: weighted_buy_fill(buy, quantity),
        market_profile: p.display_name,
        fetched_at: p.fetched_at,
        expires_at: p.expires_at,
        stale: p.stale,
        source: "CCP ESI Market Orders".into(),
    })
}
pub fn bulk_quote(conn: &Connection, requests: &[(i64, i64)]) -> Result<Vec<MarketQuote>, String> {
    requests.iter().map(|(t, q)| quote(conn, *t, *q)).collect()
}

fn max_age(headers: &reqwest::header::HeaderMap, default: i64) -> i64 {
    headers
        .get(reqwest::header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split(',')
                .find_map(|p| p.trim().strip_prefix("max-age=")?.parse().ok())
        })
        .unwrap_or(default)
}
pub fn refresh(conn: &Connection) -> Result<RefreshResult, String> {
    let p = selected_profile(conn)?;
    let http = reqwest::Client::new();
    let mut all = Vec::new();
    let mut page = 1_u32;
    let mut pages = 1_u32;
    let mut ttl = p.refresh_interval_seconds.max(300);
    while page <= pages {
        let url = format!(
            "https://esi.evetech.net/markets/{}/orders?order_type=all&page={page}",
            p.region_id
        );
        let resp = tauri::async_runtime::block_on(
            http.get(url)
                .header("X-Compatibility-Date", "2026-07-14")
                .header("User-Agent", "RenderNorthIndustrial/0.1")
                .send(),
        )
        .map_err(|e| format!("market page {page} request failed: {e}"))?;
        let status = resp.status();
        if page == 1 {
            pages = resp
                .headers()
                .get("x-pages")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            ttl = max_age(resp.headers(), ttl);
        }
        let body = tauri::async_runtime::block_on(resp.text()).map_err(|e| e.to_string())?;
        if !status.is_success() {
            let error = format!("market page {page} returned {status}: {body}");
            let _ = conn.execute("INSERT INTO market_refresh_state(profile_id,status,last_error) VALUES(?1,'error',?2) ON CONFLICT(profile_id) DO UPDATE SET status='error',last_error=excluded.last_error", (p.profile_id, &error));
            return Err(error);
        }
        all.extend(
            serde_json::from_str::<Vec<EsiOrder>>(&body)
                .map_err(|e| format!("market page {page} parse failed: {e}"))?,
        );
        page += 1;
    }
    let now = chrono::Utc::now();
    let fetched = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let expires = (now + chrono::Duration::seconds(ttl))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        conn.execute(
            "DELETE FROM market_order_cache WHERE profile_id=?1",
            [p.profile_id],
        )
        .map_err(|e| e.to_string())?;
        for o in &all {
            conn.execute("INSERT INTO market_order_cache(profile_id,order_id,type_id,is_buy_order,price,volume_remain,min_volume,order_range,location_id,system_id,fetched_at,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)", (p.profile_id,o.order_id,o.type_id,o.is_buy_order as i64,o.price,o.volume_remain,o.min_volume,&o.range,o.location_id,o.system_id,&fetched,&expires)).map_err(|e|e.to_string())?;
        }
        conn.execute("INSERT INTO market_refresh_state(profile_id,status,fetched_at,expires_at,order_count,page_count,last_error) VALUES(?1,'success',?2,?3,?4,?5,NULL) ON CONFLICT(profile_id) DO UPDATE SET status='success',fetched_at=excluded.fetched_at,expires_at=excluded.expires_at,order_count=excluded.order_count,page_count=excluded.page_count,last_error=NULL", (p.profile_id,&fetched,&expires,all.len() as i64,pages as i64)).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(e);
    }
    conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
    Ok(RefreshResult {
        order_count: all.len() as i64,
        page_count: pages as i64,
        fetched_at: fetched,
        expires_at: expires,
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySearchInput {
    pub text: Option<String>,
    pub character: Option<String>,
    pub source: Option<String>,
    pub category: Option<String>,
    pub group: Option<String>,
    pub region: Option<String>,
    pub system: Option<String>,
    pub location: Option<String>,
    pub resolved: Option<bool>,
    pub positive_only: Option<bool>,
    pub sort: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySearchRow {
    pub stack_key: String,
    pub type_id: i64,
    pub type_name: String,
    pub quantity: i64,
    pub unit_volume_m3: Option<f64>,
    pub stack_volume_m3: Option<f64>,
    pub owner: String,
    pub source: String,
    pub group_name: Option<String>,
    pub category_name: Option<String>,
    pub location_name: String,
    pub system_name: Option<String>,
    pub constellation_name: Option<String>,
    pub region_name: Option<String>,
    pub container_path: Vec<String>,
    pub location_flag: Option<String>,
    pub resolved: bool,
    pub last_synced: String,
    pub quote: MarketQuote,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySearchResult {
    pub rows: Vec<InventorySearchRow>,
    pub matching_stacks: i64,
    pub matching_units: i64,
    pub total_volume_m3: Option<f64>,
    pub total_replacement_value: Option<f64>,
    pub total_liquidation_value: Option<f64>,
}

fn contains(hay: &str, needle: &str) -> bool {
    hay.to_lowercase().contains(&needle.to_lowercase())
}
fn matches(row: &InventorySearchRow, input: &InventorySearchInput) -> bool {
    if input.positive_only.unwrap_or(false) && row.quantity <= 0 {
        return false;
    }
    if let Some(v) = &input.character {
        if !v.is_empty() && row.owner != *v {
            return false;
        }
    }
    if let Some(v) = &input.source {
        if !v.is_empty() && row.source != *v {
            return false;
        }
    }
    if let Some(v) = &input.category {
        if !v.is_empty() && row.category_name.as_deref() != Some(v) {
            return false;
        }
    }
    if let Some(v) = &input.group {
        if !v.is_empty() && row.group_name.as_deref() != Some(v) {
            return false;
        }
    }
    if let Some(v) = &input.region {
        if !v.is_empty() && row.region_name.as_deref() != Some(v) {
            return false;
        }
    }
    if let Some(v) = &input.system {
        if !v.is_empty() && row.system_name.as_deref() != Some(v) {
            return false;
        }
    }
    if let Some(v) = &input.location {
        if !v.is_empty() && row.location_name != *v {
            return false;
        }
    }
    if let Some(v) = input.resolved {
        if row.resolved != v {
            return false;
        }
    }
    if let Some(q) = input.text.as_ref().filter(|q| !q.trim().is_empty()) {
        let joined = format!(
            "{} {} {} {} {} {} {} {} {}",
            row.type_name,
            row.owner,
            row.location_name,
            row.system_name.as_deref().unwrap_or(""),
            row.constellation_name.as_deref().unwrap_or(""),
            row.region_name.as_deref().unwrap_or(""),
            row.container_path.join(" "),
            row.location_flag.as_deref().unwrap_or(""),
            row.source
        );
        if !contains(&joined, q) {
            return false;
        }
    }
    true
}
pub fn search_inventory(
    conn: &Connection,
    input: &InventorySearchInput,
) -> Result<InventorySearchResult, String> {
    let mut rows = Vec::new();
    let mut stmt=conn.prepare("SELECT a.character_id,a.item_id,a.type_id,COALESCE(t.name,'Unknown Type '||a.type_id),a.quantity,c.name,COALESCE(g.name,''),COALESCE(cat.name,''),a.location_id,a.location_flag,a.synced_at,t.volume_m3 FROM character_assets a JOIN characters c ON c.character_id=a.character_id LEFT JOIN eve_types t ON t.type_id=a.type_id LEFT JOIN eve_groups g ON g.group_id=t.group_id LEFT JOIN eve_categories cat ON cat.category_id=g.category_id WHERE c.is_demo=0").map_err(|e|format!("inventory search query failed: {e}"))?;
    let assets = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, i64>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, Option<f64>>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for (cid, item, type_id, name, quantity, owner, group, category, location_id, flag, synced, unit_volume_m3) in
        assets
    {
        let loc = crate::location::resolve(conn, cid, location_id)?;
        let quote = quote(conn, type_id, quantity)?;
        rows.push(InventorySearchRow {
            stack_key: format!("esi-{cid}-{item}"),
            type_id,
            type_name: name,
            quantity,
            unit_volume_m3,
            stack_volume_m3: crate::volume::volume_for_quantity(unit_volume_m3, quantity),
            owner,
            source: "ESI Character Assets".into(),
            group_name: if group.is_empty() { None } else { Some(group) },
            category_name: if category.is_empty() {
                None
            } else {
                Some(category)
            },
            location_name: loc.display_name,
            system_name: loc.solar_system_name,
            constellation_name: loc.constellation_name,
            region_name: loc.region_name,
            container_path: loc.container_path,
            location_flag: Some(flag),
            resolved: loc.resolution_status == "resolved",
            last_synced: synced,
            quote,
        })
    }
    let mut manual=conn.prepare("SELECT m.id,m.type_id,t.name,m.quantity,m.location_name,m.updated_at,COALESCE(g.name,''),COALESCE(c.name,''),t.volume_m3 FROM manual_inventory_entries m JOIN eve_types t ON t.type_id=m.type_id LEFT JOIN eve_groups g ON g.group_id=t.group_id LEFT JOIN eve_categories c ON c.category_id=g.category_id").map_err(|e|e.to_string())?;
    let entries = manual
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, Option<f64>>(8)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    for (id, type_id, name, quantity, location, synced, group, category, unit_volume_m3) in entries {
        let quote = quote(conn, type_id, quantity)?;
        rows.push(InventorySearchRow {
            stack_key: format!("manual-{id}"),
            type_id,
            type_name: name,
            quantity,
            unit_volume_m3,
            stack_volume_m3: crate::volume::volume_for_quantity(unit_volume_m3, quantity),
            owner: "Manual".into(),
            source: "Manual Inventory".into(),
            group_name: if group.is_empty() { None } else { Some(group) },
            category_name: if category.is_empty() {
                None
            } else {
                Some(category)
            },
            location_name: location,
            system_name: None,
            constellation_name: None,
            region_name: None,
            container_path: vec![],
            location_flag: None,
            resolved: false,
            last_synced: synced,
            quote,
        })
    }
    rows.retain(|r| matches(r, input));
    match input.sort.as_deref() {
        Some("quantity") => rows.sort_by(|a, b| b.quantity.cmp(&a.quantity)),
        Some("owner") => rows.sort_by(|a, b| a.owner.cmp(&b.owner)),
        Some("location") => rows.sort_by(|a, b| a.location_name.cmp(&b.location_name)),
        Some("replacement") => rows.sort_by(|a, b| {
            b.quote
                .acquisition
                .total_price
                .unwrap_or(-1.0)
                .total_cmp(&a.quote.acquisition.total_price.unwrap_or(-1.0))
        }),
        Some("unit_acquisition") => rows.sort_by(|a, b| {
            b.quote
                .acquisition
                .unit_price
                .unwrap_or(-1.0)
                .total_cmp(&a.quote.acquisition.unit_price.unwrap_or(-1.0))
        }),
        Some("liquidation") => rows.sort_by(|a, b| {
            b.quote
                .liquidation
                .total_price
                .unwrap_or(-1.0)
                .total_cmp(&a.quote.liquidation.total_price.unwrap_or(-1.0))
        }),
        Some("unit_liquidation") => rows.sort_by(|a, b| {
            b.quote
                .liquidation
                .unit_price
                .unwrap_or(-1.0)
                .total_cmp(&a.quote.liquidation.unit_price.unwrap_or(-1.0))
        }),
        Some("synced") => rows.sort_by(|a, b| b.last_synced.cmp(&a.last_synced)),
        _ => rows.sort_by(|a, b| a.type_name.cmp(&b.type_name)),
    }
    let units = rows.iter().map(|r| r.quantity).sum();
    let total_volume_m3 =
        crate::volume::aggregate_volume(rows.iter().map(|row| row.stack_volume_m3));
    let replacement = (!rows.is_empty())
        .then(|| {
            rows.iter()
                .try_fold(0.0, |a, r| Some(a + complete_total(&r.quote.acquisition)?))
        })
        .flatten();
    let liquidation = (!rows.is_empty())
        .then(|| {
            rows.iter()
                .try_fold(0.0, |a, r| Some(a + complete_total(&r.quote.liquidation)?))
        })
        .flatten();
    Ok(InventorySearchResult {
        matching_stacks: rows.len() as i64,
        matching_units: units,
        total_volume_m3,
        total_replacement_value: replacement,
        total_liquidation_value: liquidation,
        rows,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CostAssumptions {
    pub sales_tax_percent: f64,
    pub broker_fee_percent: f64,
    pub manufacturing_job_cost: f64,
    pub hauling_cost: f64,
    pub other_cost: f64,
}

impl CostAssumptions {
    fn validate(&self) -> Result<(), String> {
        let percentages = [
            ("sales tax percentage", self.sales_tax_percent),
            ("broker fee percentage", self.broker_fee_percent),
        ];
        for (name, value) in percentages {
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(format!("{name} must be a number from 0 through 100"));
            }
        }
        let costs = [
            ("manufacturing job cost", self.manufacturing_job_cost),
            ("hauling cost", self.hauling_cost),
            ("other cost", self.other_cost),
        ];
        for (name, value) in costs {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{name} must be a non-negative number"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CostAssumptionState {
    pub assumptions: CostAssumptions,
    pub saved_at: Option<String>,
}

pub fn load_cost_assumptions(
    conn: &Connection,
    operation_id: i64,
) -> Result<CostAssumptionState, String> {
    conn.query_row(
        "SELECT sales_tax_percent,broker_fee_percent,manufacturing_job_cost,hauling_cost,other_cost,updated_at FROM operation_cost_assumptions WHERE operation_id=?1",
        [operation_id],
        |row| {
            Ok(CostAssumptionState {
                assumptions: CostAssumptions {
                    sales_tax_percent: row.get(0)?,
                    broker_fee_percent: row.get(1)?,
                    manufacturing_job_cost: row.get(2)?,
                    hauling_cost: row.get(3)?,
                    other_cost: row.get(4)?,
                },
                saved_at: Some(row.get(5)?),
            })
        },
    )
    .optional()
    .map_err(|error| format!("failed to load cost assumptions: {error}"))
    .map(|state| {
        state.unwrap_or(CostAssumptionState {
            assumptions: CostAssumptions::default(),
            saved_at: None,
        })
    })
}

pub fn save_cost_assumptions(
    conn: &Connection,
    operation_id: i64,
    assumptions: &CostAssumptions,
) -> Result<CostAssumptionState, String> {
    assumptions.validate()?;
    conn.execute(
        "INSERT INTO operation_cost_assumptions(operation_id,sales_tax_percent,broker_fee_percent,manufacturing_job_cost,hauling_cost,other_cost) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(operation_id) DO UPDATE SET sales_tax_percent=excluded.sales_tax_percent,broker_fee_percent=excluded.broker_fee_percent,manufacturing_job_cost=excluded.manufacturing_job_cost,hauling_cost=excluded.hauling_cost,other_cost=excluded.other_cost,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        (
            operation_id,
            assumptions.sales_tax_percent,
            assumptions.broker_fee_percent,
            assumptions.manufacturing_job_cost,
            assumptions.hauling_cost,
            assumptions.other_cost,
        ),
    )
    .map_err(|error| format!("failed to save cost assumptions: {error}"))?;
    load_cost_assumptions(conn, operation_id)
}

pub fn reset_cost_assumptions(
    conn: &Connection,
    operation_id: i64,
) -> Result<CostAssumptionState, String> {
    save_cost_assumptions(conn, operation_id, &CostAssumptions::default())
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialCostLine {
    pub type_id: i64,
    pub type_name: String,
    pub required_quantity: i64,
    pub owned_quantity: i64,
    pub shortage_quantity: i64,
    pub unit_volume_m3: Option<f64>,
    pub required_volume_m3: Option<f64>,
    pub owned_volume_m3: Option<f64>,
    pub purchase_volume_m3: Option<f64>,
    pub unit_acquisition_price: Option<f64>,
    pub replacement_value: Option<f64>,
    pub owned_opportunity_cost: Option<f64>,
    pub shortage_cash_cost: Option<f64>,
    pub available_volume: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEconomics {
    pub operation_id: i64,
    pub market_profile: String,
    pub materials: Vec<MaterialCostLine>,
    pub total_material_replacement_cost: Option<f64>,
    pub owned_material_opportunity_cost: Option<f64>,
    pub cash_required: Option<f64>,
    pub total_material_volume_m3: Option<f64>,
    pub total_purchase_volume_m3: Option<f64>,
    pub output_immediate_sale_value: Option<f64>,
    pub output_listed_sale_value: Option<f64>,
    pub revenue_basis: String,
    pub revenue_basis_message: Option<String>,
    pub gross_profit: Option<f64>,
    pub sales_tax: Option<f64>,
    pub broker_fee: Option<f64>,
    pub manufacturing_job_cost: f64,
    pub hauling_cost: f64,
    pub other_cost: f64,
    pub estimated_net_profit: Option<f64>,
    pub margin: Option<f64>,
    pub roi: Option<f64>,
    pub stale: bool,
    pub price_timestamp: Option<String>,
}
struct ProfitTotals {
    replacement: Option<f64>,
    owned: Option<f64>,
    cash: Option<f64>,
    gross: Option<f64>,
    tax: Option<f64>,
    broker: Option<f64>,
    net: Option<f64>,
    margin: Option<f64>,
    roi: Option<f64>,
}
fn profit_totals(
    materials: &[MaterialCostLine],
    revenue: Option<f64>,
    a: &CostAssumptions,
) -> ProfitTotals {
    let sum = |f: fn(&MaterialCostLine) -> Option<f64>| {
        materials.iter().try_fold(0.0, |n, l| Some(n + f(l)?))
    };
    let replacement = sum(|l| l.replacement_value);
    let owned = sum(|l| l.owned_opportunity_cost);
    let cash = sum(|l| l.shortage_cash_cost);
    let gross = revenue.zip(replacement).map(|(r, c)| r - c);
    let tax = revenue.map(|v| v * a.sales_tax_percent / 100.0);
    let broker = revenue.map(|v| v * a.broker_fee_percent / 100.0);
    let net = gross
        .zip(tax)
        .zip(broker)
        .map(|((g, t), b)| g - t - b - a.manufacturing_job_cost - a.hauling_cost - a.other_cost);
    let margin = gross
        .zip(revenue)
        .and_then(|(g, v)| if v > 0.0 { Some(g / v) } else { None });
    let roi = gross
        .zip(replacement)
        .and_then(|(g, c)| if c > 0.0 { Some(g / c) } else { None });
    ProfitTotals {
        replacement,
        owned,
        cash,
        gross,
        tax,
        broker,
        net,
        margin,
        roi,
    }
}

fn select_revenue(
    immediate: Option<f64>,
    listed: Option<f64>,
    requested: Option<&str>,
) -> (String, Option<f64>, Option<String>) {
    match requested {
        Some("listed") if listed.is_some() => ("listed".into(), listed, None),
        Some("listed") if immediate.is_some() => (
            "immediate".into(),
            immediate,
            Some(
                "Listed-sale valuation unavailable at the selected market; using Immediate Sale"
                    .into(),
            ),
        ),
        Some("immediate") if immediate.is_some() => ("immediate".into(), immediate, None),
        Some("immediate") if listed.is_some() => (
            "listed".into(),
            listed,
            Some(
                "Immediate-sale valuation unavailable at the selected market; using Listed Sale"
                    .into(),
            ),
        ),
        Some(value) if value != "immediate" && value != "listed" => (
            "immediate".into(),
            None,
            Some(format!("invalid revenue basis: {value}")),
        ),
        _ if immediate.is_some() => ("immediate".into(), immediate, None),
        _ if listed.is_some() => ("listed".into(), listed, None),
        Some("listed") => (
            "listed".into(),
            None,
            Some("Listed-sale valuation unavailable at the selected market".into()),
        ),
        Some("immediate") => (
            "immediate".into(),
            None,
            Some("Immediate-sale valuation unavailable at the selected market".into()),
        ),
        _ => (
            "immediate".into(),
            None,
            Some(
                "Immediate-sale and listed-sale valuations are unavailable at the selected market"
                    .into(),
            ),
        ),
    }
}

pub fn operation_economics(
    conn: &Connection,
    operation_id: i64,
    requested_revenue_basis: Option<&str>,
) -> Result<OperationEconomics, String> {
    let assumptions = load_cost_assumptions(conn, operation_id)?.assumptions;
    let plan = crate::production::repository::ProductionRepository::new(conn)
        .calculate_plan(operation_id)?;
    let mut materials = Vec::new();
    for l in &plan.leaf_totals {
        let q = quote(conn, l.type_id, l.required_quantity)?;
        let unit = complete_unit_price(&q.acquisition);
        materials.push(MaterialCostLine {
            type_id: l.type_id,
            type_name: l.type_name.clone(),
            required_quantity: l.required_quantity,
            owned_quantity: l.available_quantity.min(l.required_quantity),
            shortage_quantity: l.missing_quantity,
            unit_volume_m3: l.unit_volume_m3,
            required_volume_m3: l.required_volume_m3,
            owned_volume_m3: l.owned_volume_m3,
            purchase_volume_m3: l.missing_volume_m3,
            unit_acquisition_price: unit,
            replacement_value: unit.map(|p| p * l.required_quantity as f64),
            owned_opportunity_cost: unit
                .map(|p| p * l.available_quantity.min(l.required_quantity) as f64),
            shortage_cash_cost: unit.map(|p| p * l.missing_quantity as f64),
            available_volume: q.acquisition.available_volume,
        })
    }
    let output = quote(conn, plan.build_target_type_id, plan.produced_quantity)?;
    let immediate = complete_total(&output.liquidation);
    let listed = complete_total(&output.acquisition);
    let (revenue_basis, revenue, revenue_basis_message) =
        select_revenue(immediate, listed, requested_revenue_basis);
    let totals = profit_totals(&materials, revenue, &assumptions);
    let total_material_volume_m3 =
        crate::volume::aggregate_volume(materials.iter().map(|line| line.required_volume_m3));
    let total_purchase_volume_m3 =
        crate::volume::aggregate_volume(materials.iter().map(|line| line.purchase_volume_m3));
    Ok(OperationEconomics {
        operation_id,
        market_profile: output.market_profile,
        total_material_replacement_cost: totals.replacement,
        owned_material_opportunity_cost: totals.owned,
        cash_required: totals.cash,
        total_material_volume_m3,
        total_purchase_volume_m3,
        output_immediate_sale_value: immediate,
        output_listed_sale_value: listed,
        revenue_basis,
        revenue_basis_message,
        gross_profit: totals.gross,
        sales_tax: totals.tax,
        broker_fee: totals.broker,
        manufacturing_job_cost: assumptions.manufacturing_job_cost,
        hauling_cost: assumptions.hauling_cost,
        other_cost: assumptions.other_cost,
        estimated_net_profit: totals.net,
        margin: totals.margin,
        roi: totals.roi,
        stale: output.stale,
        price_timestamp: output.fetched_at,
        materials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn levels() -> Vec<OrderLevel> {
        vec![
            OrderLevel {
                price: 10.0,
                volume: 2,
            },
            OrderLevel {
                price: 12.0,
                volume: 3,
            },
        ]
    }
    #[test]
    fn weighted_sell_crosses_levels() {
        let f = weighted_sell_fill(levels(), 5);
        assert_eq!(f.total_price, Some(56.0));
        assert_eq!(f.unit_price, Some(11.2));
        assert!(f.sufficient)
    }
    #[test]
    fn weighted_buy_crosses_levels() {
        let f = weighted_buy_fill(
            vec![
                OrderLevel {
                    price: 9.0,
                    volume: 3,
                },
                OrderLevel {
                    price: 10.0,
                    volume: 2,
                },
            ],
            5,
        );
        assert_eq!(f.total_price, Some(47.0));
    }
    #[test]
    fn insufficient_volume_is_explicit() {
        let f = weighted_sell_fill(
            vec![OrderLevel {
                price: 1.0,
                volume: 2,
            }],
            5,
        );
        assert!(!f.sufficient);
        assert_eq!(f.filled, 2)
    }
    #[test]
    fn no_orders_is_not_zero() {
        let f = weighted_sell_fill(vec![], 5);
        assert_eq!(f.total_price, None);
        assert_eq!(f.unit_price, None)
    }
    fn row() -> InventorySearchRow {
        InventorySearchRow {
            stack_key: "x".into(),
            type_id: 34,
            type_name: "Tritanium".into(),
            quantity: 5,
            unit_volume_m3: Some(0.01),
            stack_volume_m3: Some(0.05),
            owner: "Maxdelta".into(),
            source: "ESI Character Assets".into(),
            group_name: Some("Mineral".into()),
            category_name: Some("Material".into()),
            location_name: "Jita 4-4".into(),
            system_name: Some("Jita".into()),
            constellation_name: Some("Kimotoro".into()),
            region_name: Some("The Forge".into()),
            container_path: vec!["Container".into()],
            location_flag: Some("Hangar".into()),
            resolved: true,
            last_synced: "now".into(),
            quote: MarketQuote {
                type_id: 34,
                quantity: 5,
                unit_volume_m3: Some(0.01),
                requested_volume_m3: Some(0.05),
                acquisition: weighted_sell_fill(levels(), 5),
                liquidation: weighted_buy_fill(levels(), 5),
                market_profile: "Jita".into(),
                fetched_at: Some("now".into()),
                expires_at: Some("later".into()),
                stale: false,
                source: "ESI".into(),
            },
        }
    }
    #[test]
    fn inventory_text_search_covers_name_owner_and_hierarchy() {
        for q in [
            "tritanium",
            "maxdelta",
            "kimotoro",
            "container",
            "hangar",
            "character assets",
        ] {
            assert!(
                matches(
                    &row(),
                    &InventorySearchInput {
                        text: Some(q.into()),
                        ..Default::default()
                    }
                ),
                "{q}"
            )
        }
    }
    #[test]
    fn character_location_category_and_group_filters_are_enforced() {
        let good = InventorySearchInput {
            character: Some("Maxdelta".into()),
            location: Some("Jita 4-4".into()),
            category: Some("Material".into()),
            group: Some("Mineral".into()),
            ..Default::default()
        };
        assert!(matches(&row(), &good));
        let bad = InventorySearchInput {
            character: Some("Saber Side".into()),
            ..Default::default()
        };
        assert!(!matches(&row(), &bad))
    }
    #[test]
    fn no_result_search_is_false() {
        assert!(!matches(
            &row(),
            &InventorySearchInput {
                text: Some("Definitely absent".into()),
                ..Default::default()
            }
        ))
    }
    #[test]
    fn inventory_stack_values_use_weighted_totals() {
        let r = row();
        assert_eq!(r.quote.acquisition.total_price, Some(56.0));
        assert_eq!(r.quote.liquidation.total_price, Some(56.0))
    }
    #[test]
    fn profit_includes_owned_opportunity_cost_fees_margin_and_roi() {
        let lines = vec![MaterialCostLine {
            type_id: 1,
            type_name: "A".into(),
            required_quantity: 10,
            owned_quantity: 4,
            shortage_quantity: 6,
            unit_volume_m3: Some(1.0),
            required_volume_m3: Some(10.0),
            owned_volume_m3: Some(4.0),
            purchase_volume_m3: Some(6.0),
            unit_acquisition_price: Some(2.0),
            replacement_value: Some(20.0),
            owned_opportunity_cost: Some(8.0),
            shortage_cash_cost: Some(12.0),
            available_volume: 100,
        }];
        let a = CostAssumptions {
            sales_tax_percent: 10.0,
            broker_fee_percent: 5.0,
            manufacturing_job_cost: 1.0,
            hauling_cost: 2.0,
            other_cost: 3.0,
        };
        let t = profit_totals(&lines, Some(40.0), &a);
        assert_eq!(t.replacement, Some(20.0));
        assert_eq!(t.owned, Some(8.0));
        assert_eq!(t.cash, Some(12.0));
        assert_eq!(t.gross, Some(20.0));
        assert_eq!(t.net, Some(8.0));
        assert_eq!(t.margin, Some(0.5));
        assert_eq!(t.roi, Some(1.0))
    }
    #[test]
    fn missing_price_propagates_instead_of_becoming_zero() {
        let lines = vec![MaterialCostLine {
            type_id: 1,
            type_name: "A".into(),
            required_quantity: 1,
            owned_quantity: 1,
            shortage_quantity: 0,
            unit_volume_m3: Some(1.0),
            required_volume_m3: Some(1.0),
            owned_volume_m3: Some(1.0),
            purchase_volume_m3: Some(0.0),
            unit_acquisition_price: None,
            replacement_value: None,
            owned_opportunity_cost: None,
            shortage_cash_cost: None,
            available_volume: 0,
        }];
        let t = profit_totals(&lines, Some(10.0), &CostAssumptions::default());
        assert_eq!(t.replacement, None);
        assert_eq!(t.net, None)
    }
    #[test]
    fn stale_cache_is_labeled_and_fresh_cache_is_reused() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE market_profiles(profile_id INTEGER,display_name TEXT,region_id INTEGER,location_id INTEGER,refresh_interval_seconds INTEGER,is_selected INTEGER);CREATE TABLE market_refresh_state(profile_id INTEGER,status TEXT,fetched_at TEXT,expires_at TEXT,order_count INTEGER,page_count INTEGER,last_error TEXT);INSERT INTO market_profiles VALUES(1,'Jita',10000002,60003760,300,1);INSERT INTO market_refresh_state VALUES(1,'success','old','2000-01-01T00:00:00Z',1,1,NULL);").unwrap();
        assert!(selected_profile(&c).unwrap().stale);
        c.execute(
            "UPDATE market_refresh_state SET expires_at='2999-01-01T00:00:00Z'",
            [],
        )
        .unwrap();
        assert!(!selected_profile(&c).unwrap().stale)
    }
    #[test]
    fn failed_snapshot_transaction_preserves_previous_orders() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE orders(id INTEGER PRIMARY KEY,price REAL CHECK(price>0));INSERT INTO orders VALUES(1,10);BEGIN;DELETE FROM orders;INSERT INTO orders VALUES(2,-1);").unwrap_err();
        c.execute_batch("ROLLBACK").unwrap();
        assert_eq!(
            c.query_row("SELECT price FROM orders WHERE id=1", [], |r| r
                .get::<_, f64>(0))
                .unwrap(),
            10.0
        )
    }

    fn assumptions_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE operation_cost_assumptions(
                operation_id INTEGER PRIMARY KEY,
                sales_tax_percent REAL NOT NULL DEFAULT 0,
                broker_fee_percent REAL NOT NULL DEFAULT 0,
                manufacturing_job_cost REAL NOT NULL DEFAULT 0,
                hauling_cost REAL NOT NULL DEFAULT 0,
                other_cost REAL NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
            );",
        )
        .unwrap();
        conn
    }

    fn nonzero_assumptions() -> CostAssumptions {
        CostAssumptions {
            sales_tax_percent: 3.6,
            broker_fee_percent: 1.0,
            manufacturing_job_cost: 97_000_000.0,
            hauling_cost: 100_000_000.0,
            other_cost: 25.5,
        }
    }

    #[test]
    fn cost_assumptions_save_and_reload_exact_values() {
        let conn = assumptions_db();
        let expected = nonzero_assumptions();
        let saved = save_cost_assumptions(&conn, 11, &expected).unwrap();
        assert_eq!(saved.assumptions, expected);
        assert!(saved.saved_at.is_some());
        assert_eq!(
            load_cost_assumptions(&conn, 11).unwrap().assumptions,
            expected
        );
    }

    #[test]
    fn cost_assumptions_are_operation_specific_and_resettable() {
        let conn = assumptions_db();
        let expected = nonzero_assumptions();
        save_cost_assumptions(&conn, 11, &expected).unwrap();
        save_cost_assumptions(
            &conn,
            12,
            &CostAssumptions {
                hauling_cost: 42.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            load_cost_assumptions(&conn, 11).unwrap().assumptions,
            expected
        );
        assert_eq!(
            load_cost_assumptions(&conn, 12)
                .unwrap()
                .assumptions
                .hauling_cost,
            42.0
        );
        assert_eq!(
            reset_cost_assumptions(&conn, 11).unwrap().assumptions,
            CostAssumptions::default()
        );
        assert_eq!(
            load_cost_assumptions(&conn, 12)
                .unwrap()
                .assumptions
                .hauling_cost,
            42.0
        );
    }

    #[test]
    fn invalid_cost_assumptions_are_rejected_before_persistence() {
        let conn = assumptions_db();
        let invalid_percent = CostAssumptions {
            sales_tax_percent: 101.0,
            ..Default::default()
        };
        assert!(save_cost_assumptions(&conn, 1, &invalid_percent).is_err());
        let invalid_cost = CostAssumptions {
            hauling_cost: -1.0,
            ..Default::default()
        };
        assert!(save_cost_assumptions(&conn, 1, &invalid_cost).is_err());
        assert_eq!(load_cost_assumptions(&conn, 1).unwrap().saved_at, None);
    }

    #[test]
    fn persistence_failure_is_returned_as_an_error() {
        let conn = Connection::open_in_memory().unwrap();
        let error = save_cost_assumptions(&conn, 1, &CostAssumptions::default()).unwrap_err();
        assert!(error.contains("failed to save cost assumptions"));
    }

    #[test]
    fn recalculation_after_apply_uses_reloaded_costs() {
        let conn = assumptions_db();
        let materials = vec![MaterialCostLine {
            type_id: 1,
            type_name: "A".into(),
            required_quantity: 10,
            owned_quantity: 4,
            shortage_quantity: 6,
            unit_volume_m3: Some(1.0),
            required_volume_m3: Some(10.0),
            owned_volume_m3: Some(4.0),
            purchase_volume_m3: Some(6.0),
            unit_acquisition_price: Some(2.0),
            replacement_value: Some(20.0),
            owned_opportunity_cost: Some(8.0),
            shortage_cash_cost: Some(12.0),
            available_volume: 100,
        }];
        let costs = CostAssumptions {
            manufacturing_job_cost: 7.0,
            hauling_cost: 3.0,
            ..Default::default()
        };
        save_cost_assumptions(&conn, 1, &costs).unwrap();
        let reloaded = load_cost_assumptions(&conn, 1).unwrap().assumptions;
        let totals = profit_totals(&materials, Some(40.0), &reloaded);
        assert_eq!(totals.gross, Some(20.0));
        assert_eq!(totals.net, Some(10.0));
    }

    #[test]
    fn revenue_basis_falls_back_to_the_available_value() {
        assert_eq!(select_revenue(Some(30.0), None, None).0, "immediate");
        let listed_missing = select_revenue(Some(30.0), None, Some("listed"));
        assert_eq!(listed_missing.0, "immediate");
        assert_eq!(listed_missing.1, Some(30.0));
        assert!(listed_missing
            .2
            .unwrap()
            .contains("Listed-sale valuation unavailable"));

        assert_eq!(select_revenue(None, Some(40.0), None).0, "listed");
        let immediate_missing = select_revenue(None, Some(40.0), Some("immediate"));
        assert_eq!(immediate_missing.0, "listed");
        assert_eq!(immediate_missing.1, Some(40.0));
        assert!(immediate_missing
            .2
            .unwrap()
            .contains("Immediate-sale valuation unavailable"));
    }

    #[test]
    fn revenue_basis_supports_switching_both_available_and_neither_available() {
        assert_eq!(select_revenue(Some(30.0), Some(40.0), None).0, "immediate");
        assert_eq!(
            select_revenue(Some(30.0), Some(40.0), Some("listed")).1,
            Some(40.0)
        );
        assert_eq!(
            select_revenue(Some(30.0), Some(40.0), Some("immediate")).1,
            Some(30.0)
        );
        let neither = select_revenue(None, None, None);
        assert_eq!(neither.1, None);
        assert!(neither.2.is_some());
    }

    #[test]
    fn selected_revenue_drives_fees_while_manual_costs_leave_gross_unchanged() {
        let materials = vec![MaterialCostLine {
            type_id: 1,
            type_name: "A".into(),
            required_quantity: 1,
            owned_quantity: 1,
            shortage_quantity: 0,
            unit_volume_m3: Some(1.0),
            required_volume_m3: Some(1.0),
            owned_volume_m3: Some(1.0),
            purchase_volume_m3: Some(0.0),
            unit_acquisition_price: Some(20.0),
            replacement_value: Some(20.0),
            owned_opportunity_cost: Some(20.0),
            shortage_cash_cost: Some(0.0),
            available_volume: 100,
        }];
        let no_manual = profit_totals(&materials, Some(100.0), &CostAssumptions::default());
        let with_manual = profit_totals(
            &materials,
            Some(100.0),
            &CostAssumptions {
                sales_tax_percent: 10.0,
                broker_fee_percent: 5.0,
                manufacturing_job_cost: 7.0,
                hauling_cost: 3.0,
                other_cost: 0.0,
            },
        );
        assert_eq!(no_manual.gross, Some(80.0));
        assert_eq!(with_manual.gross, Some(80.0));
        assert_eq!(with_manual.tax, Some(10.0));
        assert_eq!(with_manual.broker, Some(5.0));
        assert_eq!(with_manual.net, Some(55.0));
    }

    fn market_search_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE market_profiles(profile_id INTEGER PRIMARY KEY,display_name TEXT,region_id INTEGER,location_id INTEGER,refresh_interval_seconds INTEGER,is_selected INTEGER);
             CREATE TABLE market_refresh_state(profile_id INTEGER PRIMARY KEY,status TEXT,fetched_at TEXT,expires_at TEXT,order_count INTEGER,page_count INTEGER,last_error TEXT);
             CREATE TABLE market_order_cache(profile_id INTEGER,order_id INTEGER,type_id INTEGER,is_buy_order INTEGER,price REAL,volume_remain INTEGER,min_volume INTEGER,order_range TEXT,location_id INTEGER,system_id INTEGER,fetched_at TEXT,expires_at TEXT,source TEXT);
             CREATE TABLE eve_categories(category_id INTEGER PRIMARY KEY,name TEXT);
             CREATE TABLE eve_groups(group_id INTEGER PRIMARY KEY,category_id INTEGER,name TEXT);
             CREATE TABLE eve_types(type_id INTEGER PRIMARY KEY,name TEXT,group_id INTEGER,published INTEGER,is_manufacturable INTEGER,volume_m3 REAL);
             CREATE TABLE blueprint_products(blueprint_type_id INTEGER,product_type_id INTEGER,quantity INTEGER);
             CREATE TABLE characters(character_id INTEGER PRIMARY KEY,name TEXT,is_demo INTEGER);
             CREATE TABLE character_assets(character_id INTEGER,item_id INTEGER,type_id INTEGER,quantity INTEGER,location_id INTEGER,location_flag TEXT,synced_at TEXT);
             CREATE TABLE manual_inventory_entries(id INTEGER PRIMARY KEY,type_id INTEGER,quantity INTEGER,location_name TEXT,updated_at TEXT);
             INSERT INTO market_profiles VALUES(1,'Jita 4-4',10000002,60003760,300,1);
             INSERT INTO market_refresh_state VALUES(1,'success','2026-01-01T00:00:00Z','2999-01-01T00:00:00Z',4,1,NULL);
             INSERT INTO eve_categories VALUES(1,'Commodity');
             INSERT INTO eve_groups VALUES(1,1,'Test Group');
             INSERT INTO eve_types VALUES
                (42,'Owned Widget',1,1,0,2.5),
                (43,'Radar-FTL Interlink Communicator',1,1,0,6.0),
                (44,'Known Type Without Orders',1,1,0,4.0);
             INSERT INTO manual_inventory_entries VALUES(1,42,3,'Owned Hangar','2026-01-01T00:00:00Z');
             INSERT INTO market_order_cache VALUES
                (1,1,43,0,10,1,1,'station',60003760,30000142,'now','later','CCP ESI Market Orders'),
                (1,2,43,0,12,9,1,'station',60003760,30000142,'now','later','CCP ESI Market Orders'),
                (1,3,43,1,8,2,1,'station',60003760,30000142,'now','later','CCP ESI Market Orders'),
                (1,4,43,1,7,8,1,'region',60000000,30000142,'now','later','CCP ESI Market Orders');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn my_inventory_finds_owned_item_and_unowned_item_has_no_owned_state() {
        let conn = market_search_db();
        let owned = search_inventory(
            &conn,
            &InventorySearchInput {
                text: Some("Owned Widget".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(owned.matching_stacks, 1);
        assert_eq!(owned.matching_units, 3);
        assert_eq!(owned.total_volume_m3, Some(7.5));

        let unowned = search_inventory(
            &conn,
            &InventorySearchInput {
                text: Some("Radar-FTL".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(unowned.matching_stacks, 0);
        assert!(unowned.rows.is_empty());
        assert_eq!(unowned.total_replacement_value, None);
        assert_eq!(unowned.total_liquidation_value, None);
    }

    #[test]
    fn market_search_finds_and_prices_an_unowned_known_type() {
        let conn = market_search_db();
        let found = crate::staticdata::search_types(&conn, "Radar-FTL", 25).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].type_id, 43);
        let owned_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM manual_inventory_entries WHERE type_id=43",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(owned_count, 0);
        let quote = quote(&conn, 43, 1).unwrap();
        assert_eq!(quote.acquisition.unit_price, Some(10.0));
        assert_eq!(quote.acquisition.total_price, Some(10.0));
        assert_eq!(quote.liquidation.unit_price, Some(8.0));
        assert_eq!(quote.liquidation.total_price, Some(8.0));
        assert_eq!(quote.unit_volume_m3, Some(6.0));
        assert_eq!(quote.requested_volume_m3, Some(6.0));
    }

    #[test]
    fn market_search_quantity_uses_weighted_multi_order_depth() {
        let conn = market_search_db();
        let quote = quote(&conn, 43, 5).unwrap();
        assert_eq!(quote.acquisition.total_price, Some(58.0));
        assert_eq!(quote.acquisition.unit_price, Some(11.6));
        assert_eq!(quote.liquidation.total_price, Some(37.0));
        assert_eq!(quote.liquidation.unit_price, Some(7.4));
        assert_eq!(quote.requested_volume_m3, Some(30.0));
    }

    #[test]
    fn market_search_distinguishes_no_type_no_orders_and_insufficient_depth() {
        let conn = market_search_db();
        assert!(
            crate::staticdata::search_types(&conn, "Definitely Missing", 25)
                .unwrap()
                .is_empty()
        );
        let no_orders = quote(&conn, 44, 1).unwrap();
        assert_eq!(no_orders.acquisition.filled, 0);
        assert_eq!(no_orders.liquidation.filled, 0);
        assert_eq!(no_orders.acquisition.total_price, None);
        let insufficient = quote(&conn, 43, 20).unwrap();
        assert!(!insufficient.acquisition.sufficient);
        assert!(!insufficient.liquidation.sufficient);
        assert_eq!(insufficient.acquisition.filled, 10);
        assert_eq!(insufficient.liquidation.filled, 10);
    }

    #[test]
    fn inventory_filters_do_not_affect_static_market_search() {
        let conn = market_search_db();
        let inventory_filter = InventorySearchInput {
            character: Some("Maxdelta".into()),
            location: Some("Some owned location".into()),
            source: Some("Manual Inventory".into()),
            ..Default::default()
        };
        assert!(!matches(&row(), &inventory_filter));
        let found = crate::staticdata::search_types(&conn, "Radar-FTL", 25).unwrap();
        assert_eq!(found[0].type_id, 43);
    }

    #[test]
    fn direct_market_quote_reuses_cached_rows_without_refreshing_esi() {
        let conn = market_search_db();
        let before: i64 = conn
            .query_row("SELECT COUNT(*) FROM market_order_cache", [], |row| {
                row.get(0)
            })
            .unwrap();
        let first = quote(&conn, 43, 5).unwrap();
        let second = quote(&conn, 43, 5).unwrap();
        let after: i64 = conn
            .query_row("SELECT COUNT(*) FROM market_order_cache", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            first.acquisition.total_price,
            second.acquisition.total_price
        );
        assert_eq!(
            first.liquidation.total_price,
            second.liquidation.total_price
        );
        assert_eq!(before, after);
    }
}

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineInput {
    pub name: String,
    pub description: String,
    pub category: String,
    pub fleet_notes: String,
    pub is_active: bool,
    pub version: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Doctrine {
    pub doctrine_id: i64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub fleet_notes: String,
    pub is_active: bool,
    pub version: i64,
    pub fit_count: i64,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitItem {
    pub type_id: i64,
    pub item_name: String,
    pub item_kind: String,
    pub quantity: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineFit {
    pub fit_id: i64,
    pub doctrine_id: i64,
    pub name: String,
    pub hull_type_id: i64,
    pub hull_name: String,
    pub desired_quantity: i64,
    pub original_eft_text: String,
    pub source_name: String,
    pub items: Vec<FitItem>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitReadiness {
    pub fit_id: i64,
    pub name: String,
    pub hull_name: String,
    pub desired: i64,
    pub ready_now: i64,
    pub ready_after_build: i64,
    pub still_missing: i64,
    pub coverage_percent: f64,
    pub status: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineItem {
    pub type_id: i64,
    pub item_name: String,
    pub item_kind: String,
    pub required: i64,
    pub owned: i64,
    pub reserved: i64,
    pub available: i64,
    pub missing: i64,
    pub manufacturable: bool,
    pub owned_blueprint: bool,
    pub action: String,
    pub unit_volume_m3: Option<f64>,
    pub total_missing_volume_m3: Option<f64>,
    pub purchase_unit_price: Option<f64>,
    pub purchase_cost: Option<f64>,
    pub market_status: String,
    pub blocked_reason: Option<String>,
    pub blocked_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildMaterialDetail {
    pub type_id: i64,
    pub item_name: String,
    pub required_quantity: i64,
    pub owned_quantity: i64,
    pub missing_quantity: i64,
    pub unit_volume_m3: Option<f64>,
    pub missing_volume_m3: Option<f64>,
    pub acquisition_unit_price: Option<f64>,
    pub total_acquisition_cost: Option<f64>,
    pub market_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildDetail {
    pub type_id: i64,
    pub item_name: String,
    pub missing_quantity: i64,
    pub manufacturing_runs: i64,
    pub output_quantity: i64,
    pub blueprint_source: Option<String>,
    pub blueprint_owner: Option<String>,
    pub me: Option<i64>,
    pub te: Option<i64>,
    pub raw_material_cost: Option<f64>,
    pub raw_material_volume_m3: Option<f64>,
    pub missing_blueprint_warning: Option<String>,
    pub unmanufacturable_warning: Option<String>,
    pub warnings: Vec<String>,
    pub raw_materials: Vec<BuildMaterialDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasonCount {
    pub reason: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionSummary {
    pub ready_item_types: i64,
    pub build_item_types: i64,
    pub buy_item_types: i64,
    pub blocked_item_types: i64,
    pub missing_blueprints: i64,
    pub unpriced_lines: i64,
    pub insufficient_volume_lines: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuartermasterExports {
    pub buy_finished_goods: crate::procurement::ShoppingExports,
    pub buy_manufacturing_inputs: crate::procurement::ShoppingExports,
    pub combined_purchase_list: crate::procurement::ShoppingExports,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuartermasterSummary {
    pub buy_cost: f64,
    pub buy_volume_m3: f64,
    pub build_input_cost: f64,
    pub build_input_volume_m3: f64,
    pub total_completion_cost: f64,
    pub total_hauling_volume_m3: f64,
    pub totals_complete: bool,
    pub excluded_lines: i64,
    pub ready_now: i64,
    pub ready_after_build: i64,
    pub still_missing_after_build: i64,
    pub target: i64,
    pub coverage_now_percent: f64,
    pub coverage_after_build_percent: f64,
    pub overall_status: String,
    pub can_fully_field: bool,
    pub blocking_components: Vec<String>,
    pub shopping_ready: bool,
    pub production_ready: bool,
    pub action_summary: ActionSummary,
    pub blocked_reasons: Vec<ReasonCount>,
    pub partial_reasons: Vec<ReasonCount>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoctrineAnalysis {
    pub doctrine: Doctrine,
    pub fits: Vec<FitReadiness>,
    pub items: Vec<DoctrineItem>,
    pub build_details: Vec<BuildDetail>,
    pub buy_lines: Vec<crate::procurement::ShoppingLine>,
    pub build_input_lines: Vec<crate::procurement::ShoppingLine>,
    pub summary: QuartermasterSummary,
    pub shopping: QuartermasterExports,
}

fn validate(input: &DoctrineInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("doctrine name is required".into());
    }
    if input.version < 1 {
        return Err("doctrine version must be at least 1".into());
    }
    Ok(())
}
pub fn create(conn: &Connection, input: &DoctrineInput) -> Result<Doctrine, String> {
    validate(input)?;
    conn.execute("INSERT INTO doctrine_groups(name,description,category,fleet_notes,is_active,version) VALUES(?1,?2,?3,?4,?5,?6)",(&input.name,&input.description,&input.category,&input.fleet_notes,input.is_active as i64,input.version)).map_err(|e|format!("failed to create doctrine: {e}"))?;
    get(conn, conn.last_insert_rowid())
}
pub fn update(conn: &Connection, id: i64, input: &DoctrineInput) -> Result<Doctrine, String> {
    validate(input)?;
    let n=conn.execute("UPDATE doctrine_groups SET name=?2,description=?3,category=?4,fleet_notes=?5,is_active=?6,version=?7,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE doctrine_id=?1",(id,&input.name,&input.description,&input.category,&input.fleet_notes,input.is_active as i64,input.version)).map_err(|e|format!("failed to update doctrine: {e}"))?;
    if n == 0 {
        return Err(format!("doctrine {id} not found"));
    }
    get(conn, id)
}
pub fn list(conn: &Connection) -> Result<Vec<Doctrine>, String> {
    let mut s=conn.prepare("SELECT d.doctrine_id,d.name,d.description,d.category,d.fleet_notes,d.is_active,d.version,COUNT(f.fit_id),d.updated_at FROM doctrine_groups d LEFT JOIN doctrine_fits f ON f.doctrine_id=d.doctrine_id GROUP BY d.doctrine_id ORDER BY d.is_active DESC,d.name,d.doctrine_id").map_err(|e|e.to_string())?;
    let rows = s
        .query_map([], map_doctrine)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}
fn map_doctrine(r: &rusqlite::Row) -> rusqlite::Result<Doctrine> {
    Ok(Doctrine {
        doctrine_id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        category: r.get(3)?,
        fleet_notes: r.get(4)?,
        is_active: r.get::<_, i64>(5)? != 0,
        version: r.get(6)?,
        fit_count: r.get(7)?,
        updated_at: r.get(8)?,
    })
}
pub fn get(conn: &Connection, id: i64) -> Result<Doctrine, String> {
    conn.query_row("SELECT d.doctrine_id,d.name,d.description,d.category,d.fleet_notes,d.is_active,d.version,COUNT(f.fit_id),d.updated_at FROM doctrine_groups d LEFT JOIN doctrine_fits f ON f.doctrine_id=d.doctrine_id WHERE d.doctrine_id=?1 GROUP BY d.doctrine_id",[id],map_doctrine).map_err(|e|format!("doctrine {id} not found: {e}"))
}

#[derive(Clone)]
struct TypeMeta {
    i: i64,
    n: String,
    g: String,
    c: String,
    v: Option<f64>,
}
fn resolve(conn: &Connection, name: &str) -> Result<TypeMeta, String> {
    conn.query_row("SELECT t.type_id,t.name,COALESCE(g.name,''),COALESCE(c.name,''),t.volume_m3 FROM eve_types t LEFT JOIN eve_groups g ON g.group_id=t.group_id LEFT JOIN eve_categories c ON c.category_id=g.category_id WHERE lower(t.name)=lower(?1) LIMIT 1",[name.trim()],|r|Ok(TypeMeta{i:r.get(0)?,n:r.get(1)?,g:r.get(2)?,c:r.get(3)?,v:r.get(4)?})).map_err(|_|format!("EFT type not found in imported CCP static data: {name}"))
}
fn kind(m: &TypeMeta) -> String {
    let s = format!("{} {}", m.g, m.c).to_lowercase();
    if s.contains("script") {
        "Script"
    } else if s.contains("rig") {
        "Rig"
    } else if s.contains("subsystem") {
        "Subsystem"
    } else if s.contains("drone") || s.contains("fighter") {
        "Drone"
    } else if s.contains("charge") || s.contains("ammo") || s.contains("missile") {
        "Charge"
    } else if s.contains("module") {
        "Module"
    } else {
        "Cargo"
    }
    .into()
}
fn uncomment(line: &str) -> &str {
    let mut end = line.len();
    for token in ["//", "#", ";"] {
        if let Some(i) = line.find(token) {
            end = end.min(i)
        }
    }
    line[..end].trim()
}
fn named_quantity(text: &str) -> (String, i64) {
    let t = text.trim();
    if let Some((name, q)) = t.rsplit_once(" x") {
        if let Ok(n) = q.trim().parse::<i64>() {
            if n > 0 {
                return (name.trim().into(), n);
            }
        }
    }
    (t.into(), 1)
}
fn parse(conn: &Connection, text: &str) -> Result<(String, TypeMeta, Vec<FitItem>), String> {
    let mut lines = text.lines().map(uncomment).filter(|l| !l.is_empty());
    let h = lines.next().ok_or("EFT text is empty")?;
    if !h.starts_with('[') || !h.ends_with(']') {
        return Err("invalid EFT header; expected [Hull, Fit Name]".into());
    }
    let inside = &h[1..h.len() - 1];
    let mut p = inside.splitn(2, ',');
    let hull = p.next().unwrap().trim();
    if hull.is_empty() {
        return Err("EFT hull is missing".into());
    }
    let hm = resolve(conn, hull)?;
    let name = p
        .next()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(hull)
        .to_string();
    let mut agg: BTreeMap<i64, FitItem> = BTreeMap::new();
    for line in lines {
        if line.starts_with('[') && line.to_lowercase().contains("empty") {
            continue;
        }
        if line.starts_with('[') {
            return Err(format!("malformed EFT line: {line}"));
        }
        for part in line.split(',') {
            let (n, q) = named_quantity(part);
            let m = resolve(conn, &n)?;
            let k = kind(&m);
            agg.entry(m.i)
                .and_modify(|x| x.quantity += q)
                .or_insert(FitItem {
                    type_id: m.i,
                    item_name: m.n,
                    item_kind: k,
                    quantity: q,
                });
        }
    }
    Ok((name, hm, agg.into_values().collect()))
}

pub fn import_text(
    conn: &Connection,
    doctrine_id: i64,
    text: &str,
    source: &str,
) -> Result<DoctrineFit, String> {
    get(conn, doctrine_id)?;
    let (name, hull, items) = parse(conn, text)?;
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<i64, String> {
        conn.execute("INSERT INTO doctrine_fits(doctrine_id,name,hull_type_id,hull_name,original_eft_text,source_name) VALUES(?1,?2,?3,?4,?5,?6)",(doctrine_id,&name,hull.i,&hull.n,text,source)).map_err(|e|e.to_string())?;
        let id = conn.last_insert_rowid();
        for i in items {
            conn.execute("INSERT INTO doctrine_fit_items(fit_id,type_id,item_name,item_kind,quantity) VALUES(?1,?2,?3,?4,?5)",(id,i.type_id,&i.item_name,&i.item_kind,i.quantity)).map_err(|e|e.to_string())?;
        }
        Ok(id)
    })();
    match result {
        Ok(id) => {
            conn.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            fit(conn, id)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(format!("failed to import EFT: {e}"))
        }
    }
}
pub fn import_file(conn: &Connection, id: i64, path: &str) -> Result<DoctrineFit, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("failed to read EFT file '{path}': {e}"))?;
    import_text(conn, id, &text, path)
}
pub fn import_folder(conn: &Connection, id: i64, path: &str) -> Result<Vec<DoctrineFit>, String> {
    let mut files = fs::read_dir(path)
        .map_err(|e| format!("failed to read EFT folder '{path}': {e}"))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && matches!(
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_ascii_lowercase())
                        .as_deref(),
                    Some("eft" | "txt")
                )
        })
        .collect::<Vec<_>>();
    files.sort();
    if files.is_empty() {
        return Err("EFT folder contains no .eft or .txt files".into());
    }
    files
        .into_iter()
        .map(|p| import_file(conn, id, p.to_string_lossy().as_ref()))
        .collect()
}
pub fn fits(conn: &Connection, id: i64) -> Result<Vec<DoctrineFit>, String> {
    let mut s = conn
        .prepare("SELECT fit_id FROM doctrine_fits WHERE doctrine_id=?1 ORDER BY name,fit_id")
        .map_err(|e| e.to_string())?;
    let ids = s
        .query_map([id], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    ids.into_iter().map(|x| fit(conn, x)).collect()
}
fn fit(conn: &Connection, id: i64) -> Result<DoctrineFit, String> {
    let mut f=conn.query_row("SELECT fit_id,doctrine_id,name,hull_type_id,hull_name,desired_quantity,original_eft_text,source_name FROM doctrine_fits WHERE fit_id=?1",[id],|r|Ok(DoctrineFit{fit_id:r.get(0)?,doctrine_id:r.get(1)?,name:r.get(2)?,hull_type_id:r.get(3)?,hull_name:r.get(4)?,desired_quantity:r.get(5)?,original_eft_text:r.get(6)?,source_name:r.get(7)?,items:vec![]})).map_err(|e|format!("fit {id} not found: {e}"))?;
    let mut s=conn.prepare("SELECT type_id,item_name,item_kind,quantity FROM doctrine_fit_items WHERE fit_id=?1 ORDER BY item_kind,item_name,type_id").map_err(|e|e.to_string())?;
    f.items = s
        .query_map([id], |r| {
            Ok(FitItem {
                type_id: r.get(0)?,
                item_name: r.get(1)?,
                item_kind: r.get(2)?,
                quantity: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(f)
}
pub fn set_quantity(conn: &Connection, id: i64, q: i64) -> Result<DoctrineFit, String> {
    if q < 0 {
        return Err("desired quantity cannot be negative".into());
    }
    let n=conn.execute("UPDATE doctrine_fits SET desired_quantity=?2,updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE fit_id=?1",(id,q)).map_err(|e|e.to_string())?;
    if n == 0 {
        return Err(format!("fit {id} not found"));
    }
    fit(conn, id)
}
pub fn delete_fit(conn: &Connection, id: i64) -> Result<(), String> {
    let n = conn
        .execute("DELETE FROM doctrine_fits WHERE fit_id=?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        Err(format!("fit {id} not found"))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
struct BlueprintChoice {
    source: String,
    owner: String,
    me: i64,
    te: i64,
}

fn blueprint_choice(conn: &Connection, product: i64) -> Result<Option<BlueprintChoice>, String> {
    let manual = conn
        .query_row(
            "SELECT 'Manual Ownership',COALESCE(c.name,'Manual Ownership'),b.me_level,b.te_level
             FROM blueprints b
             LEFT JOIN characters c ON c.character_id=b.character_id
             WHERE b.is_demo=0 AND (
               lower(b.type_name)=lower((SELECT name FROM eve_types WHERE type_id=?1)) OR
               lower(b.type_name) IN (
                 SELECT lower(t.name) FROM blueprint_products p
                 JOIN eve_types t ON t.type_id=p.blueprint_type_id
                 WHERE p.product_type_id=?1
               )
             )
             ORDER BY b.is_copy,b.me_level DESC,b.te_level DESC,b.blueprint_id LIMIT 1",
            [product],
            |r| {
                Ok(BlueprintChoice {
                    source: r.get(0)?,
                    owner: r.get(1)?,
                    me: r.get(2)?,
                    te: r.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("manual blueprint lookup failed: {e}"))?;
    if manual.is_some() {
        return Ok(manual);
    }
    conn.query_row(
        "SELECT 'ESI Character Blueprints',c.name,cb.material_efficiency,cb.time_efficiency
         FROM character_blueprints cb
         JOIN characters c ON c.character_id=cb.character_id
         WHERE c.enabled=1 AND c.is_demo=0 AND cb.type_id IN (
           SELECT blueprint_type_id FROM blueprint_products WHERE product_type_id=?1
         )
         ORDER BY CASE WHEN cb.runs=-1 THEN 0 ELSE 1 END,
                  cb.material_efficiency DESC,cb.time_efficiency DESC,c.name,cb.item_id LIMIT 1",
        [product],
        |r| {
            Ok(BlueprintChoice {
                source: r.get(0)?,
                owner: r.get(1)?,
                me: r.get(2)?,
                te: r.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("synchronized blueprint lookup failed: {e}"))
}
fn reserved(conn: &Connection, name: &str) -> Result<i64, String> {
    conn.query_row("SELECT COALESCE(SUM(r.quantity),0) FROM inventory_reservations r JOIN inventory_items i ON i.item_id=r.item_id WHERE r.released_at IS NULL AND i.type_name=?1 AND i.source!='demo'",[name],|r|r.get(0)).map_err(|e|e.to_string())
}

fn usable(conn: &Connection, type_id: i64, name: &str) -> Result<i64, String> {
    let owned = crate::character::assets::global_owned_quantity(conn, type_id)?.0;
    Ok((owned - reserved(conn, name)?).max(0))
}

fn allocate_fits(fits: &[DoctrineFit], mut pool: BTreeMap<i64, i64>) -> (i64, BTreeMap<i64, i64>) {
    let mut total = 0;
    let mut by_fit = BTreeMap::new();
    for fit in fits {
        let mut requirements = BTreeMap::from([(fit.hull_type_id, 1_i64)]);
        for item in &fit.items {
            *requirements.entry(item.type_id).or_default() += item.quantity;
        }
        let mut ready = 0;
        for _ in 0..fit.desired_quantity {
            if requirements
                .iter()
                .all(|(type_id, quantity)| pool.get(type_id).copied().unwrap_or(0) >= *quantity)
            {
                for (type_id, quantity) in &requirements {
                    *pool.entry(*type_id).or_default() -= quantity;
                }
                ready += 1;
                total += 1;
            } else {
                break;
            }
        }
        by_fit.insert(fit.fit_id, ready);
    }
    (total, by_fit)
}

fn shopping_line(
    conn: &Connection,
    type_id: i64,
    item_name: String,
    required: i64,
    owned: i64,
    shortage: i64,
    unit_volume_m3: Option<f64>,
) -> Result<crate::procurement::ShoppingLine, String> {
    let quote = crate::market::quote(conn, type_id, shortage)?;
    let complete = quote.acquisition.sufficient;
    let status = if quote.acquisition.available_volume == 0 {
        "No market orders"
    } else if !complete {
        "Insufficient market volume"
    } else if quote.stale {
        "Stale price"
    } else {
        "Current"
    };
    Ok(crate::procurement::ShoppingLine {
        type_id,
        item_name,
        required_quantity: required,
        owned_quantity: owned,
        shortage_quantity: shortage,
        unit_volume_m3,
        total_volume_m3: crate::volume::volume_for_quantity(unit_volume_m3, shortage),
        acquisition_unit_price: complete.then_some(quote.acquisition.unit_price).flatten(),
        total_acquisition_cost: complete.then_some(quote.acquisition.total_price).flatten(),
        available_market_volume: quote.acquisition.available_volume,
        market_status: status.into(),
        price_timestamp: quote.fetched_at,
        procurement_status: "Needed".into(),
        notes: String::new(),
        last_updated: None,
    })
}

fn known_cost(lines: &[crate::procurement::ShoppingLine]) -> f64 {
    lines
        .iter()
        .filter_map(|line| line.total_acquisition_cost)
        .sum()
}

fn known_volume(lines: &[crate::procurement::ShoppingLine]) -> f64 {
    lines.iter().filter_map(|line| line.total_volume_m3).sum()
}

pub fn analyze(conn: &Connection, id: i64) -> Result<DoctrineAnalysis, String> {
    let doctrine = get(conn, id)?;
    let fs = fits(conn, id)?;
    if fs.is_empty() {
        return Err("doctrine has no fits to analyze".into());
    }
    let mut demand: BTreeMap<i64, (String, String, i64, Option<f64>)> = BTreeMap::new();
    for f in &fs {
        let d = f.desired_quantity;
        demand
            .entry(f.hull_type_id)
            .and_modify(|x| x.2 += d)
            .or_insert((
                f.hull_name.clone(),
                "Hull".into(),
                d,
                resolve(conn, &f.hull_name)?.v,
            ));
        for i in &f.items {
            let q = i.quantity * d;
            demand.entry(i.type_id).and_modify(|x| x.2 += q).or_insert((
                i.item_name.clone(),
                i.item_kind.clone(),
                q,
                resolve(conn, &i.item_name)?.v,
            ));
        }
    }
    let mut inventory_pool = BTreeMap::new();
    let mut items = vec![];
    let mut choices = BTreeMap::new();
    for (type_id, (name, k, required, vol)) in demand {
        let owned = crate::character::assets::global_owned_quantity(conn, type_id)?.0;
        let res = reserved(conn, &name)?;
        let available = (owned - res).max(0);
        inventory_pool.insert(type_id, available);
        let missing = (required - available).max(0);
        let manufacturable: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM blueprint_products WHERE product_type_id=?1)",
                [type_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let choice = blueprint_choice(conn, type_id)?;
        if let Some(value) = choice.clone() {
            choices.insert(type_id, value);
        }
        let ob = choice.is_some();
        items.push(DoctrineItem {
            type_id,
            item_name: name,
            item_kind: k,
            required,
            owned,
            reserved: res,
            available,
            missing,
            manufacturable,
            owned_blueprint: ob,
            action: if missing == 0 {
                "READY"
            } else if manufacturable && ob {
                "BUILD"
            } else if manufacturable {
                "BLOCKED"
            } else {
                "BUY"
            }
            .into(),
            unit_volume_m3: vol,
            total_missing_volume_m3: crate::volume::volume_for_quantity(vol, missing),
            purchase_unit_price: None,
            purchase_cost: None,
            market_status: if missing == 0 {
                "Not required"
            } else if manufacturable && ob {
                "Production path"
            } else if manufacturable {
                "Required blueprint unavailable"
            } else {
                "Pending price"
            }
            .into(),
            blocked_reason: if missing > 0 && manufacturable && !ob {
                Some("Missing blueprint".into())
            } else {
                None
            },
            blocked_detail: if missing > 0 && manufacturable && !ob {
                Some("Required blueprint unavailable".into())
            } else {
                None
            },
        });
    }
    items.sort_by(|a, b| {
        a.item_name
            .cmp(&b.item_name)
            .then(a.type_id.cmp(&b.type_id))
    });
    let mut excluded = BTreeSet::new();
    let mut buy_lines = Vec::new();
    for item in items.iter_mut().filter(|item| item.action == "BUY") {
        let line = shopping_line(
            conn,
            item.type_id,
            item.item_name.clone(),
            item.required,
            item.available.min(item.required),
            item.missing,
            item.unit_volume_m3,
        )?;
        item.purchase_unit_price = line.acquisition_unit_price;
        item.purchase_cost = line.total_acquisition_cost;
        item.market_status = line.market_status.clone();
        if line.total_acquisition_cost.is_none() || line.total_volume_m3.is_none() {
            excluded.insert(format!("buy:{}", item.type_id));
        }
        if matches!(
            line.market_status.as_str(),
            "No market orders" | "Insufficient market volume"
        ) {
            item.action = "BLOCKED".into();
            item.blocked_reason = Some(
                if line.market_status == "No market orders" {
                    "No market orders"
                } else {
                    "Insufficient market depth"
                }
                .into(),
            );
            item.blocked_detail = Some(line.market_status.clone());
        } else if line.total_volume_m3.is_none() {
            item.action = "BLOCKED".into();
            item.blocked_reason = Some("Missing static volume".into());
            item.blocked_detail = Some("CCP static type volume is unavailable".into());
        }
        buy_lines.push(line);
    }

    // Finished doctrine items consume usable inventory before manufacturing
    // inputs are allocated, preventing the same stack from satisfying both.
    let mut remaining_pool = inventory_pool.clone();
    for item in &items {
        let used = item.available.min(item.required);
        *remaining_pool.entry(item.type_id).or_default() -= used;
    }

    let production = crate::production::repository::ProductionRepository::new(conn);
    let mut input_totals: BTreeMap<i64, (String, i64, i64, i64, Option<f64>)> = BTreeMap::new();
    let mut build_details = Vec::new();
    let mut build_allocations: Vec<BTreeMap<i64, i64>> = Vec::new();
    for item in items.iter_mut().filter(|item| item.action == "BUILD") {
        let choice = choices.get(&item.type_id).expect("BUILD has a blueprint");
        match production.calculate_target_plan(item.type_id, item.missing, choice.me, choice.te) {
            Ok(plan) => {
                if !plan.warnings.is_empty() {
                    item.action = "BLOCKED".into();
                    item.market_status = plan.warnings.join("; ");
                    item.blocked_reason = Some("No production path".into());
                    item.blocked_detail = Some(plan.warnings.join("; "));
                    excluded.insert(format!("build:{}", item.type_id));
                }
                let mut allocation = BTreeMap::new();
                let mut raw_materials = Vec::new();
                let mut raw_volume = 0.0;
                let mut volume_complete = true;
                for leaf in &plan.leaf_requirements {
                    let available = if let Some(value) = remaining_pool.get_mut(&leaf.type_id) {
                        value
                    } else {
                        let value = usable(conn, leaf.type_id, &leaf.type_name)?;
                        remaining_pool.entry(leaf.type_id).or_insert(value)
                    };
                    let used = (*available).min(leaf.required_quantity);
                    *available -= used;
                    let missing = leaf.required_quantity - used;
                    *allocation.entry(leaf.type_id).or_default() += missing;
                    let entry = input_totals.entry(leaf.type_id).or_insert((
                        leaf.type_name.clone(),
                        0,
                        0,
                        0,
                        leaf.unit_volume_m3,
                    ));
                    entry.1 += leaf.required_quantity;
                    entry.2 += used;
                    entry.3 += missing;
                    match crate::volume::volume_for_quantity(leaf.unit_volume_m3, missing) {
                        Some(value) => raw_volume += value,
                        None => volume_complete = false,
                    }
                    raw_materials.push(BuildMaterialDetail {
                        type_id: leaf.type_id,
                        item_name: leaf.type_name.clone(),
                        required_quantity: leaf.required_quantity,
                        owned_quantity: used,
                        missing_quantity: missing,
                        unit_volume_m3: leaf.unit_volume_m3,
                        missing_volume_m3: crate::volume::volume_for_quantity(
                            leaf.unit_volume_m3,
                            missing,
                        ),
                        acquisition_unit_price: None,
                        total_acquisition_cost: None,
                        market_status: if missing == 0 {
                            "Ready"
                        } else {
                            "Pending price"
                        }
                        .into(),
                    });
                }
                build_allocations.push(allocation);
                build_details.push(BuildDetail {
                    type_id: item.type_id,
                    item_name: item.item_name.clone(),
                    missing_quantity: item.missing,
                    manufacturing_runs: plan.total_runs,
                    output_quantity: plan.produced_quantity,
                    blueprint_source: Some(choice.source.clone()),
                    blueprint_owner: Some(choice.owner.clone()),
                    me: Some(choice.me),
                    te: Some(choice.te),
                    raw_material_cost: None,
                    raw_material_volume_m3: volume_complete.then_some(raw_volume),
                    missing_blueprint_warning: None,
                    unmanufacturable_warning: None,
                    warnings: plan.warnings,
                    raw_materials,
                });
            }
            Err(error) => {
                item.action = "BLOCKED".into();
                item.market_status = error.clone();
                item.blocked_reason = Some("No production path".into());
                item.blocked_detail = Some(error.clone());
                excluded.insert(format!("build:{}", item.type_id));
                build_allocations.push(BTreeMap::new());
                build_details.push(BuildDetail {
                    type_id: item.type_id,
                    item_name: item.item_name.clone(),
                    missing_quantity: item.missing,
                    manufacturing_runs: 0,
                    output_quantity: 0,
                    blueprint_source: Some(choice.source.clone()),
                    blueprint_owner: Some(choice.owner.clone()),
                    me: Some(choice.me),
                    te: Some(choice.te),
                    raw_material_cost: None,
                    raw_material_volume_m3: None,
                    missing_blueprint_warning: None,
                    unmanufacturable_warning: Some(error),
                    warnings: Vec::new(),
                    raw_materials: Vec::new(),
                });
            }
        }
    }
    for item in items
        .iter()
        .filter(|item| item.action == "BLOCKED" && item.manufacturable && !item.owned_blueprint)
    {
        excluded.insert(format!("build:{}", item.type_id));
        build_allocations.push(BTreeMap::new());
        build_details.push(BuildDetail {
            type_id: item.type_id,
            item_name: item.item_name.clone(),
            missing_quantity: item.missing,
            manufacturing_runs: 0,
            output_quantity: 0,
            blueprint_source: None,
            blueprint_owner: None,
            me: None,
            te: None,
            raw_material_cost: None,
            raw_material_volume_m3: None,
            missing_blueprint_warning: Some("Required owned blueprint is unavailable".into()),
            unmanufacturable_warning: None,
            warnings: Vec::new(),
            raw_materials: Vec::new(),
        });
    }

    let mut build_input_lines = Vec::new();
    for (type_id, (name, required, owned, shortage, volume)) in input_totals {
        if shortage == 0 {
            continue;
        }
        let line = shopping_line(conn, type_id, name, required, owned, shortage, volume)?;
        if line.total_acquisition_cost.is_none() || line.total_volume_m3.is_none() {
            excluded.insert(format!("input:{type_id}"));
        }
        build_input_lines.push(line);
    }
    let input_by_type = build_input_lines
        .iter()
        .map(|line| (line.type_id, line))
        .collect::<BTreeMap<_, _>>();
    for (detail, allocation) in build_details.iter_mut().zip(build_allocations.iter()) {
        let mut cost = 0.0;
        let mut complete = true;
        for (type_id, quantity) in allocation {
            if *quantity == 0 {
                continue;
            }
            match input_by_type
                .get(type_id)
                .and_then(|line| line.acquisition_unit_price)
            {
                Some(unit) => cost += unit * *quantity as f64,
                None => complete = false,
            }
        }
        detail.raw_material_cost = complete.then_some(cost);
        for material in &mut detail.raw_materials {
            if material.missing_quantity == 0 {
                continue;
            }
            if let Some(line) = input_by_type.get(&material.type_id) {
                material.acquisition_unit_price = line.acquisition_unit_price;
                material.total_acquisition_cost = line
                    .acquisition_unit_price
                    .map(|unit| unit * material.missing_quantity as f64);
                material.market_status = line.market_status.clone();
            }
        }
    }

    let mut combined: BTreeMap<i64, (String, i64, i64, i64, Option<f64>)> = BTreeMap::new();
    for line in buy_lines.iter().chain(build_input_lines.iter()) {
        let entry = combined.entry(line.type_id).or_insert((
            line.item_name.clone(),
            0,
            0,
            0,
            line.unit_volume_m3,
        ));
        entry.1 += line.required_quantity;
        entry.2 += line.owned_quantity;
        entry.3 += line.shortage_quantity;
    }
    let mut combined_lines = Vec::new();
    for (type_id, (name, required, owned, shortage, volume)) in combined {
        combined_lines.push(shopping_line(
            conn, type_id, name, required, owned, shortage, volume,
        )?);
    }

    let buy_summary = crate::procurement::summary_for_lines(&buy_lines);
    let input_summary = crate::procurement::summary_for_lines(&build_input_lines);
    let combined_summary = crate::procurement::summary_for_lines(&combined_lines);
    let profile = crate::market::selected_profile(conn)?.display_name;
    let exports = QuartermasterExports {
        buy_finished_goods: crate::procurement::exports_for_lines(
            &format!("{} — Buy Finished Goods", doctrine.name),
            &profile,
            &buy_summary,
            &buy_lines,
        ),
        buy_manufacturing_inputs: crate::procurement::exports_for_lines(
            &format!("{} — Buy Manufacturing Inputs", doctrine.name),
            &profile,
            &input_summary,
            &build_input_lines,
        ),
        combined_purchase_list: crate::procurement::exports_for_lines(
            &format!("{} — Combined Purchase List", doctrine.name),
            &profile,
            &combined_summary,
            &combined_lines,
        ),
    };

    let target = fs.iter().map(|fit| fit.desired_quantity).sum::<i64>();
    let (ready_now, now_by_fit) = allocate_fits(&fs, inventory_pool.clone());
    let mut after_build_pool = inventory_pool;
    for item in &items {
        if item.action == "BUILD" {
            *after_build_pool.entry(item.type_id).or_default() += item.missing;
        }
    }
    let (ready_after_build, after_by_fit) = allocate_fits(&fs, after_build_pool);
    let action_by_type = items
        .iter()
        .map(|item| (item.type_id, item.action.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut readiness = fs
        .iter()
        .map(|f| {
            let ready_now = now_by_fit.get(&f.fit_id).copied().unwrap_or(0);
            let ready_after_build = after_by_fit.get(&f.fit_id).copied().unwrap_or(0);
            let actions = std::iter::once(f.hull_type_id)
                .chain(f.items.iter().map(|item| item.type_id))
                .filter_map(|type_id| action_by_type.get(&type_id).copied())
                .collect::<Vec<_>>();
            let status = if ready_now >= f.desired_quantity {
                "READY"
            } else if actions.contains(&"BLOCKED") {
                "BLOCKED"
            } else if actions.contains(&"BUY") {
                "BUY"
            } else if actions.contains(&"BUILD") {
                "BUILD"
            } else {
                "BLOCKED"
            };
            FitReadiness {
                fit_id: f.fit_id,
                name: f.name.clone(),
                hull_name: f.hull_name.clone(),
                desired: f.desired_quantity,
                ready_now,
                ready_after_build,
                still_missing: (f.desired_quantity - ready_after_build).max(0),
                coverage_percent: if f.desired_quantity == 0 {
                    100.0
                } else {
                    ready_after_build as f64 / f.desired_quantity as f64 * 100.0
                },
                status: status.into(),
            }
        })
        .collect::<Vec<_>>();
    let priority = |status: &str| match status {
        "BLOCKED" => 0,
        "BUY" => 1,
        "BUILD" => 2,
        _ => 3,
    };
    readiness.sort_by(|a, b| {
        priority(&a.status)
            .cmp(&priority(&b.status))
            .then(a.name.cmp(&b.name))
            .then(a.fit_id.cmp(&b.fit_id))
    });
    let blocking = items
        .iter()
        .filter(|i| i.missing > 0)
        .map(|i| i.item_name.clone())
        .collect::<Vec<_>>();
    let production_ready = build_details.iter().all(|detail| {
        detail.missing_blueprint_warning.is_none()
            && detail.unmanufacturable_warning.is_none()
            && detail.warnings.is_empty()
    });
    let buy_cost = known_cost(&buy_lines);
    let build_input_cost = known_cost(&build_input_lines);
    let buy_volume_m3 = known_volume(&buy_lines);
    let build_input_volume_m3 = known_volume(&build_input_lines);
    let shopping_ready = excluded.is_empty();
    let count_action =
        |action: &str| items.iter().filter(|item| item.action == action).count() as i64;
    let missing_blueprints = build_details
        .iter()
        .filter(|detail| detail.missing_blueprint_warning.is_some())
        .count() as i64;
    let purchase_lines = buy_lines.iter().chain(build_input_lines.iter());
    let unpriced_lines = purchase_lines
        .clone()
        .filter(|line| line.market_status == "No market orders")
        .count() as i64;
    let insufficient_volume_lines = purchase_lines
        .clone()
        .filter(|line| line.market_status == "Insufficient market volume")
        .count() as i64;
    let missing_volume_lines = purchase_lines
        .filter(|line| line.total_volume_m3.is_none())
        .count() as i64;
    let missing_production_paths = build_details
        .iter()
        .filter(|detail| detail.unmanufacturable_warning.is_some() || !detail.warnings.is_empty())
        .count() as i64;
    let mut blocked_reason_map = BTreeMap::new();
    for reason in items
        .iter()
        .filter_map(|item| item.blocked_reason.as_deref())
    {
        *blocked_reason_map
            .entry(reason.to_string())
            .or_insert(0_i64) += 1;
    }
    let blocked_reasons = blocked_reason_map
        .into_iter()
        .map(|(reason, count)| ReasonCount { reason, count })
        .collect::<Vec<_>>();
    let mut partial_reasons = Vec::new();
    for (reason, count) in [
        ("Unpriced lines", unpriced_lines),
        ("Insufficient market volume", insufficient_volume_lines),
        ("Missing static volume", missing_volume_lines),
        ("Missing production path", missing_production_paths),
        ("Unavailable required blueprint", missing_blueprints),
    ] {
        if count > 0 {
            partial_reasons.push(ReasonCount {
                reason: reason.into(),
                count,
            });
        }
    }
    let ready_count = count_action("READY");
    let build_count = count_action("BUILD");
    let buy_count = count_action("BUY");
    let blocked_count = count_action("BLOCKED");
    let overall_status = if ready_now == target {
        "Ready"
    } else if blocked_count > 0 {
        "Blocked"
    } else if !excluded.is_empty() {
        "Partial Data"
    } else if buy_count > 0 {
        "Purchase Required"
    } else if build_count > 0 {
        "Build Required"
    } else {
        "Blocked"
    };
    Ok(DoctrineAnalysis {
        doctrine,
        fits: readiness,
        build_details,
        buy_lines,
        build_input_lines,
        summary: QuartermasterSummary {
            buy_cost,
            buy_volume_m3,
            build_input_cost,
            build_input_volume_m3,
            total_completion_cost: buy_cost + build_input_cost,
            total_hauling_volume_m3: buy_volume_m3 + build_input_volume_m3,
            totals_complete: excluded.is_empty(),
            excluded_lines: excluded.len() as i64,
            ready_now,
            ready_after_build,
            still_missing_after_build: (target - ready_after_build).max(0),
            target,
            coverage_now_percent: if target == 0 {
                100.0
            } else {
                ready_now as f64 / target as f64 * 100.0
            },
            coverage_after_build_percent: if target == 0 {
                100.0
            } else {
                ready_after_build as f64 / target as f64 * 100.0
            },
            overall_status: overall_status.into(),
            can_fully_field: ready_now == target,
            blocking_components: blocking,
            shopping_ready,
            production_ready,
            action_summary: ActionSummary {
                ready_item_types: ready_count,
                build_item_types: build_count,
                buy_item_types: buy_count,
                blocked_item_types: blocked_count,
                missing_blueprints,
                unpriced_lines,
                insufficient_volume_lines,
            },
            blocked_reasons,
            partial_reasons,
        },
        items,
        shopping: exports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    const M: &[&str] = &[
        include_str!("../../migrations/0001_init.sql"),
        include_str!("../../migrations/0002_build_targets.sql"),
        include_str!("../../migrations/0003_inventory_foundation.sql"),
        include_str!("../../migrations/0004_operation_foundation.sql"),
        include_str!("../../migrations/0005_reservation_foundation.sql"),
        include_str!("../../migrations/0006_blueprint_foundation.sql"),
        include_str!("../../migrations/0007_production_requirement_foundation.sql"),
        include_str!("../../migrations/0008_real_production_planner.sql"),
        include_str!("../../migrations/0009_inventory_scope.sql"),
        include_str!("../../migrations/0010_demo_category_correction.sql"),
        include_str!("../../migrations/0011_esi_character_auth.sql"),
        include_str!("../../migrations/0012_character_asset_sync.sql"),
        include_str!("../../migrations/0013_character_blueprint_sync.sql"),
        include_str!("../../migrations/0014_location_resolution.sql"),
        include_str!("../../migrations/0015_market_valuation.sql"),
        include_str!("../../migrations/0016_type_volume.sql"),
        include_str!("../../migrations/0017_operation_procurement.sql"),
        include_str!("../../migrations/0018_quartermaster.sql"),
    ];
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        for m in M {
            c.execute_batch(m).unwrap()
        }
        c.execute_batch("INSERT INTO eve_categories(category_id,name) VALUES(80,'Ship'),(81,'Module'),(82,'Charge'),(83,'Drone'),(84,'Commodity');INSERT INTO eve_groups(group_id,category_id,name) VALUES(80,80,'Cruiser'),(81,81,'Energy Weapon'),(82,82,'Charge'),(83,83,'Combat Drone'),(84,84,'Material');INSERT INTO eve_types(type_id,name,group_id,is_manufacturable,volume_m3) VALUES(100,'Test Ship',80,0,100),(10,'Test Laser',81,1,5),(20,'Test Ammo',82,0,.1),(30,'Test Drone',83,0,10),(40,'Test Cargo',84,0,5),(110,'Test Laser Blueprint',81,0,.01);INSERT INTO blueprint_products(blueprint_type_id,product_type_id,quantity) VALUES(110,10,1);INSERT INTO characters(character_id,name,is_demo,enabled) VALUES(900,'Pilot',0,1);INSERT INTO character_blueprints(character_id,item_id,type_id,location_id,location_flag,quantity,material_efficiency,time_efficiency,runs,source,synced_at) VALUES(900,1,110,600,'Hangar',-1,10,20,-1,'test','now');INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES(100,1,'Home'),(20,10,'Home'),(30,4,'Home');INSERT INTO market_refresh_state(profile_id,status,fetched_at,expires_at,order_count,page_count) VALUES(1,'success','2026-07-14T00:00:00Z','2999-01-01T00:00:00Z',2,1);INSERT INTO market_order_cache(profile_id,order_id,type_id,is_buy_order,price,volume_remain,min_volume,order_range,location_id,system_id,fetched_at,expires_at,source) VALUES(1,800,20,0,2,1000,1,'station',60003760,30000142,'now','later','test'),(1,801,40,0,3,1000,1,'station',60003760,30000142,'now','later','test');").unwrap();
        c.execute_batch("INSERT INTO eve_types(type_id,name,group_id,is_manufacturable,volume_m3) VALUES(50,'Test Raw',84,0,2);INSERT INTO blueprint_materials(blueprint_type_id,material_type_id,quantity) VALUES(110,50,10);INSERT INTO market_order_cache(profile_id,order_id,type_id,is_buy_order,price,volume_remain,min_volume,order_range,location_id,system_id,fetched_at,expires_at,source) VALUES(1,802,50,0,5,1000,1,'station',60003760,30000142,'now','later','test');").unwrap();
        c
    }
    fn input() -> DoctrineInput {
        DoctrineInput {
            name: "SLYCE Test".into(),
            description: "D".into(),
            category: "Fleet".into(),
            fleet_notes: "Anchor".into(),
            is_active: true,
            version: 1,
        }
    }
    const EFT:&str="[Test Ship, Test Line]\nTest Laser\nTest Ammo x10\nTest Ammo x5\nTest Drone x2\nTest Cargo x3\n# ignored";
    fn seeded() -> (Connection, i64, i64) {
        let c = db();
        let d = create(&c, &input()).unwrap();
        let f = import_text(&c, d.doctrine_id, EFT, "Clipboard").unwrap();
        set_quantity(&c, f.fit_id, 2).unwrap();
        (c, d.doctrine_id, f.fit_id)
    }
    #[test]
    fn single_eft_import_normalizes_hull_and_items() {
        let (c, id, _) = seeded();
        let f = &fits(&c, id).unwrap()[0];
        assert_eq!(f.hull_name, "Test Ship");
        assert_eq!(f.items.len(), 4)
    }
    #[test]
    fn clipboard_import_preserves_source_and_original() {
        let (c, id, _) = seeded();
        let f = &fits(&c, id).unwrap()[0];
        assert_eq!(f.source_name, "Clipboard");
        assert_eq!(f.original_eft_text, EFT)
    }
    #[test]
    fn duplicate_fits_are_preserved_and_aggregate() {
        let (c, id, _) = seeded();
        import_text(&c, id, EFT, "Second").unwrap();
        assert_eq!(fits(&c, id).unwrap().len(), 2)
    }
    #[test]
    fn module_charge_and_drone_aggregation_is_deterministic() {
        let (c, id, _) = seeded();
        let f = &fits(&c, id).unwrap()[0];
        assert_eq!(
            f.items.iter().find(|i| i.type_id == 20).unwrap().quantity,
            15
        );
        assert_eq!(
            f.items.iter().find(|i| i.type_id == 30).unwrap().item_kind,
            "Drone"
        );
        assert_eq!(
            f.items.iter().find(|i| i.type_id == 10).unwrap().item_kind,
            "Module"
        )
    }
    #[test]
    fn doctrine_update_persists_metadata() {
        let (c, id, _) = seeded();
        let mut x = input();
        x.version = 2;
        x.name = "Updated".into();
        assert_eq!(update(&c, id, &x).unwrap().version, 2)
    }
    #[test]
    fn deleted_fit_cascades_items() {
        let (c, _, fid) = seeded();
        delete_fit(&c, fid).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM doctrine_fit_items", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        )
    }
    #[test]
    fn invalid_and_malformed_eft_return_errors() {
        let c = db();
        let d = create(&c, &input()).unwrap();
        assert!(import_text(&c, d.doctrine_id, "bad", "x").is_err());
        assert!(import_text(&c, d.doctrine_id, "[Test Ship, X]\n[bad]", "x").is_err())
    }
    #[test]
    fn invalid_eft_header_is_rejected() {
        let c = db();
        let d = create(&c, &input()).unwrap();
        assert!(import_text(&c, d.doctrine_id, "not eft", "x")
            .unwrap_err()
            .contains("header"));
    }
    #[test]
    fn malformed_unknown_type_is_rejected() {
        let c = db();
        let d = create(&c, &input()).unwrap();
        assert!(
            import_text(&c, d.doctrine_id, "[Test Ship, X]\nNot A Real Type", "x")
                .unwrap_err()
                .contains("not found")
        );
    }
    #[test]
    fn charge_aggregation_combines_duplicate_lines() {
        let (c, id, _) = seeded();
        let f = &fits(&c, id).unwrap()[0];
        assert_eq!(
            f.items
                .iter()
                .find(|i| i.item_kind == "Charge")
                .unwrap()
                .quantity,
            15
        );
    }
    #[test]
    fn drone_aggregation_preserves_per_fit_quantity() {
        let (c, id, _) = seeded();
        let f = &fits(&c, id).unwrap()[0];
        assert_eq!(
            f.items
                .iter()
                .find(|i| i.item_kind == "Drone")
                .unwrap()
                .quantity,
            2
        );
    }
    #[test]
    fn analysis_coverage_and_missing_use_owned_inventory() {
        let (c, id, _) = seeded();
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.fits[0].desired, 2);
        assert_eq!(a.fits[0].ready_now, 0);
        assert_eq!(
            a.items.iter().find(|i| i.type_id == 20).unwrap().missing,
            20
        );
        assert!(!a.summary.can_fully_field)
    }
    #[test]
    fn analysis_volume_and_isk_totals_are_exact() {
        let (c, id, _) = seeded();
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.summary.buy_cost, 58.0);
        assert_eq!(a.summary.buy_volume_m3, 132.0);
        assert_eq!(a.summary.build_input_cost, 90.0);
        assert_eq!(a.summary.build_input_volume_m3, 36.0);
        assert_eq!(a.summary.total_completion_cost, 148.0);
        assert!(!a.summary.totals_complete)
    }
    #[test]
    fn manufacturing_and_blueprint_path_show_build() {
        let (c, id, _) = seeded();
        let a = analyze(&c, id).unwrap();
        let x = a.items.iter().find(|i| i.type_id == 10).unwrap();
        assert_eq!(x.action, "BUILD");
        assert!(x.owned_blueprint)
    }
    #[test]
    fn shopping_reuses_procurement_exports() {
        let (c, id, _) = seeded();
        let a = analyze(&c, id).unwrap();
        assert_eq!(
            a.shopping.combined_purchase_list.eve_multi_buy,
            "Test Ammo 20\nTest Cargo 6\nTest Raw 18\nTest Ship 1"
        );
        assert!(a
            .shopping
            .combined_purchase_list
            .discord_report
            .contains("148.00 ISK"));
        assert!(a
            .shopping
            .combined_purchase_list
            .csv
            .contains("market_status"))
    }

    fn custom(c: &Connection, eft: &str, desired: i64) -> i64 {
        let doctrine = create(c, &input()).unwrap();
        let fit = import_text(c, doctrine.doctrine_id, eft, "Test").unwrap();
        set_quantity(c, fit.fit_id, desired).unwrap();
        doctrine.doctrine_id
    }

    fn add_second_build_path(c: &Connection) {
        c.execute_batch("INSERT INTO eve_types(type_id,name,group_id,is_manufacturable,volume_m3) VALUES(11,'Test Utility',81,1,4),(111,'Test Utility Blueprint',81,0,.01);INSERT INTO blueprint_products(blueprint_type_id,product_type_id,quantity) VALUES(111,11,1);INSERT INTO blueprint_materials(blueprint_type_id,material_type_id,quantity) VALUES(111,50,4);INSERT INTO character_blueprints(character_id,item_id,type_id,location_id,location_flag,quantity,material_efficiency,time_efficiency,runs,source,synced_at) VALUES(900,2,111,600,'Hangar',-1,0,0,-1,'test','now');").unwrap();
    }

    #[test]
    fn buy_only_doctrine_separates_finished_goods() {
        let c = db();
        let id = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.summary.buy_cost, 3.0);
        assert_eq!(a.summary.build_input_cost, 0.0);
        assert_eq!(a.buy_lines.len(), 1);
    }

    #[test]
    fn build_only_doctrine_expands_inputs() {
        let c = db();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.summary.buy_cost, 0.0);
        assert_eq!(a.summary.build_input_cost, 45.0);
        assert_eq!(a.build_input_lines[0].shortage_quantity, 9);
    }

    #[test]
    fn mixed_buy_and_build_doctrine_has_both_subtotals() {
        let c = db();
        let id = custom(&c, "[Test Ship, Mixed]\nTest Laser\nTest Cargo", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(
            (a.summary.buy_cost, a.summary.build_input_cost),
            (3.0, 45.0)
        );
    }

    #[test]
    fn recursive_build_expansion_reaches_leaf_materials() {
        let c = db();
        c.execute_batch("DELETE FROM blueprint_materials WHERE blueprint_type_id=110;INSERT INTO eve_types(type_id,name,group_id,is_manufacturable,volume_m3) VALUES(60,'Test Intermediate',84,1,1),(160,'Test Intermediate Blueprint',81,0,.01);INSERT INTO blueprint_products(blueprint_type_id,product_type_id,quantity) VALUES(160,60,1);INSERT INTO blueprint_materials(blueprint_type_id,material_type_id,quantity) VALUES(110,60,10),(160,50,2);").unwrap();
        let id = custom(&c, "[Test Ship, Recursive]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_input_lines[0].item_name, "Test Raw");
        assert_eq!(a.build_input_lines[0].shortage_quantity, 18);
    }

    #[test]
    fn multiple_build_items_share_aggregated_materials() {
        let c = db();
        add_second_build_path(&c);
        let id = custom(&c, "[Test Ship, Shared]\nTest Laser\nTest Utility", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_input_lines.len(), 1);
        assert_eq!(a.build_input_lines[0].required_quantity, 13);
    }

    #[test]
    fn shared_inventory_is_subtracted_only_once() {
        let c = db();
        add_second_build_path(&c);
        c.execute("INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES(50,5,'Home')", []).unwrap();
        let id = custom(&c, "[Test Ship, Shared]\nTest Laser\nTest Utility", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_input_lines[0].shortage_quantity, 8);
    }

    #[test]
    fn manufacturing_runs_use_blueprint_output_rounding() {
        let c = db();
        c.execute(
            "UPDATE blueprint_products SET quantity=2 WHERE blueprint_type_id=110",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES(100,2,'Home')", []).unwrap();
        let id = custom(&c, "[Test Ship, Runs]\nTest Laser", 3);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_details[0].manufacturing_runs, 2);
    }

    #[test]
    fn owned_blueprint_me_is_used_for_inputs() {
        let c = db();
        let id = custom(&c, "[Test Ship, ME]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_details[0].me, Some(10));
        assert_eq!(a.build_input_lines[0].required_quantity, 9);
    }

    #[test]
    fn missing_blueprint_path_is_blocked() {
        let c = db();
        c.execute("DELETE FROM character_blueprints WHERE type_id=110", [])
            .unwrap();
        let id = custom(&c, "[Test Ship, Missing BP]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(
            a.items.iter().find(|i| i.type_id == 10).unwrap().action,
            "BLOCKED"
        );
        assert!(a.build_details[0].missing_blueprint_warning.is_some());
    }

    #[test]
    fn unpriced_buy_item_is_explicit_and_excluded() {
        let c = db();
        c.execute("DELETE FROM market_order_cache WHERE type_id=40", [])
            .unwrap();
        let id = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.buy_lines[0].market_status, "No market orders");
        assert!(!a.summary.totals_complete);
    }

    #[test]
    fn unpriced_build_input_is_explicit_and_excluded() {
        let c = db();
        c.execute("DELETE FROM market_order_cache WHERE type_id=50", [])
            .unwrap();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.build_input_lines[0].market_status, "No market orders");
        assert!(!a.summary.totals_complete);
    }

    #[test]
    fn insufficient_market_depth_is_explicit() {
        let c = db();
        c.execute(
            "UPDATE market_order_cache SET volume_remain=5 WHERE type_id=50",
            [],
        )
        .unwrap();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(
            a.build_input_lines[0].market_status,
            "Insufficient market volume"
        );
    }

    #[test]
    fn partial_total_reports_excluded_line_count() {
        let c = db();
        c.execute("DELETE FROM market_order_cache WHERE type_id=40", [])
            .unwrap();
        let id = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        let a = analyze(&c, id).unwrap();
        assert!(!a.summary.totals_complete);
        assert_eq!(a.summary.excluded_lines, 1);
    }

    #[test]
    fn buy_cost_is_weighted_direct_shortage_cost() {
        let c = db();
        let id = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        assert_eq!(analyze(&c, id).unwrap().summary.buy_cost, 3.0);
    }

    #[test]
    fn build_input_cost_is_weighted_leaf_shortage_cost() {
        let c = db();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        assert_eq!(analyze(&c, id).unwrap().summary.build_input_cost, 45.0);
    }

    #[test]
    fn total_completion_cost_sums_known_subtotals() {
        let c = db();
        let id = custom(&c, "[Test Ship, Mixed]\nTest Laser\nTest Cargo", 1);
        assert_eq!(analyze(&c, id).unwrap().summary.total_completion_cost, 48.0);
    }

    #[test]
    fn buy_volume_is_finished_good_shortage_volume() {
        let c = db();
        let id = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        assert_eq!(analyze(&c, id).unwrap().summary.buy_volume_m3, 5.0);
    }

    #[test]
    fn build_input_volume_is_leaf_shortage_volume() {
        let c = db();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        assert_eq!(analyze(&c, id).unwrap().summary.build_input_volume_m3, 18.0);
    }

    #[test]
    fn total_hauling_volume_sums_buy_and_build_inputs() {
        let c = db();
        let id = custom(&c, "[Test Ship, Mixed]\nTest Laser\nTest Cargo", 1);
        assert_eq!(
            analyze(&c, id).unwrap().summary.total_hauling_volume_m3,
            23.0
        );
    }

    #[test]
    fn coverage_now_and_after_build_count_complete_fits() {
        let c = db();
        let id = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(
            (
                a.summary.ready_now,
                a.summary.ready_after_build,
                a.summary.target
            ),
            (0, 1, 1)
        );
        assert_eq!(
            (
                a.summary.coverage_now_percent,
                a.summary.coverage_after_build_percent
            ),
            (0.0, 100.0)
        );
    }

    #[test]
    fn combined_export_avoids_finished_build_double_purchase() {
        let c = db();
        let id = custom(&c, "[Test Ship, Mixed]\nTest Laser\nTest Cargo", 1);
        let text = analyze(&c, id)
            .unwrap()
            .shopping
            .combined_purchase_list
            .eve_multi_buy;
        assert!(text.contains("Test Cargo 1"));
        assert!(text.contains("Test Raw 9"));
        assert!(!text.contains("Test Laser"));
    }

    #[test]
    fn fleet_overall_status_is_derived_from_completion_path() {
        let c = db();
        let build = custom(&c, "[Test Ship, Build]\nTest Laser", 1);
        assert_eq!(
            analyze(&c, build).unwrap().summary.overall_status,
            "Build Required"
        );
        let buy = custom(&c, "[Test Ship, Buy]\nTest Cargo", 1);
        assert_eq!(
            analyze(&c, buy).unwrap().summary.overall_status,
            "Purchase Required"
        );
        let ready = custom(&c, "[Test Ship, Ready]", 1);
        assert_eq!(analyze(&c, ready).unwrap().summary.overall_status, "Ready");
        c.execute("DELETE FROM character_blueprints WHERE type_id=110", [])
            .unwrap();
        let blocked = custom(&c, "[Test Ship, Blocked]\nTest Laser", 1);
        assert_eq!(
            analyze(&c, blocked).unwrap().summary.overall_status,
            "Blocked"
        );
    }

    #[test]
    fn still_missing_after_build_uses_complete_fit_allocation() {
        let c = db();
        let id = custom(&c, "[Test Ship, Mixed]\nTest Laser\nTest Cargo", 1);
        let summary = analyze(&c, id).unwrap().summary;
        assert_eq!(
            (summary.ready_after_build, summary.still_missing_after_build),
            (0, 1)
        );
    }

    fn status_doctrine(c: &Connection) -> i64 {
        add_second_build_path(c);
        c.execute("DELETE FROM character_blueprints WHERE type_id=111", [])
            .unwrap();
        c.execute("INSERT INTO manual_inventory_entries(type_id,quantity,location_name) VALUES(100,3,'Home')", []).unwrap();
        let doctrine = create(c, &input()).unwrap();
        for eft in [
            "[Test Ship, Z Ready]",
            "[Test Ship, A Buy]\nTest Cargo",
            "[Test Ship, B Build]\nTest Laser",
            "[Test Ship, C Blocked]\nTest Utility",
        ] {
            import_text(c, doctrine.doctrine_id, eft, "Test").unwrap();
        }
        doctrine.doctrine_id
    }

    #[test]
    fn fit_level_status_and_sort_priority_are_deterministic() {
        let c = db();
        let a = analyze(&c, status_doctrine(&c)).unwrap();
        assert_eq!(
            a.fits
                .iter()
                .map(|fit| fit.status.as_str())
                .collect::<Vec<_>>(),
            vec!["BLOCKED", "BUY", "BUILD", "READY"]
        );
        assert_eq!(
            a.fits
                .iter()
                .map(|fit| fit.name.as_str())
                .collect::<Vec<_>>(),
            vec!["C Blocked", "A Buy", "B Build", "Z Ready"]
        );
    }

    #[test]
    fn action_summary_counts_each_item_type_once() {
        let (c, id, _) = seeded();
        let counts = analyze(&c, id).unwrap().summary.action_summary;
        assert_eq!(
            (
                counts.ready_item_types,
                counts.build_item_types,
                counts.buy_item_types,
                counts.blocked_item_types
            ),
            (1, 1, 2, 1)
        );
        assert_eq!(
            (counts.unpriced_lines, counts.insufficient_volume_lines),
            (1, 0)
        );
    }

    #[test]
    fn blocked_reason_grouping_is_structured() {
        let (c, id, _) = seeded();
        let reasons = analyze(&c, id).unwrap().summary.blocked_reasons;
        assert!(reasons
            .iter()
            .any(|reason| reason.reason == "No market orders" && reason.count == 1));
    }

    #[test]
    fn partial_total_reasons_identify_excluded_data() {
        let c = db();
        c.execute("DELETE FROM market_order_cache WHERE type_id=50", [])
            .unwrap();
        let id = custom(&c, "[Test Ship, Partial]\nTest Laser", 1);
        let a = analyze(&c, id).unwrap();
        assert_eq!(a.summary.overall_status, "Partial Data");
        assert!(a
            .summary
            .partial_reasons
            .iter()
            .any(|reason| reason.reason == "Unpriced lines" && reason.count == 1));
    }
    static N: AtomicUsize = AtomicUsize::new(0);
    #[test]
    fn folder_import_reads_sorted_eft_files() {
        let c = db();
        let d = create(&c, &input()).unwrap();
        let p = std::env::temp_dir().join(format!(
            "rni154_{}_{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("b.eft"), EFT).unwrap();
        fs::write(p.join("a.txt"), EFT).unwrap();
        assert_eq!(
            import_folder(&c, d.doctrine_id, p.to_str().unwrap())
                .unwrap()
                .len(),
            2
        );
        let _ = fs::remove_dir_all(p);
    }
}

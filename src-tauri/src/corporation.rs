//! Corporation asset synchronization. Snapshots are transactionally isolated
//! from personal assets and never feed Production, Procurement, or Quartermaster.

use crate::esi::corporation_assets::{
    division_for_asset, CorporationSnapshot, CorporationSnapshotSource,
    LiveCorporationSnapshotSource,
};
use rusqlite::Connection;
use serde::Serialize;

const ASSET_SCOPE: &str = "esi-assets.read_corporation_assets.v1";
const ROLE_SCOPE: &str = "esi-characters.read_corporation_roles.v1";
const DIVISION_SCOPE: &str = "esi-corporations.read_divisions.v1";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporationSyncResult {
    pub character_id: i64,
    pub corporation_id: Option<i64>,
    pub corporation_name: Option<String>,
    pub status: String,
    pub role_verified: bool,
    pub asset_count: i64,
    pub page_count: i64,
    pub error: Option<String>,
}

fn scopes(conn: &Connection, character_id: i64) -> Result<String, String> {
    conn.query_row(
        "SELECT scopes_granted FROM characters WHERE character_id=?1 AND is_demo=0",
        [character_id],
        |r| r.get(0),
    )
    .map_err(|_| format!("character {character_id} is not connected"))
}
fn ensure_scopes(conn: &Connection, character_id: i64) -> Result<(), String> {
    let granted = scopes(conn, character_id)?;
    let missing = [ASSET_SCOPE, ROLE_SCOPE, DIVISION_SCOPE]
        .into_iter()
        .filter(|scope| !granted.split_whitespace().any(|value| value == *scope))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "reauthorization required: Add / Reauthorize Character must grant {}",
            missing.join(", ")
        ))
    }
}

fn mark_attempt(conn: &Connection, character_id: i64) -> Result<(), String> {
    conn.execute("INSERT INTO corporation_character_sync_state(character_id,status,last_attempt_at,last_error) VALUES(?1,'syncing',strftime('%Y-%m-%dT%H:%M:%SZ','now'),NULL) ON CONFLICT(character_id) DO UPDATE SET status='syncing',last_attempt_at=excluded.last_attempt_at,last_error=NULL",[character_id]).map_err(|e|e.to_string())?;
    Ok(())
}
fn mark_error(conn: &Connection, character_id: i64, error: &str) -> Result<(), String> {
    conn.execute("INSERT INTO corporation_character_sync_state(character_id,status,role_verified,last_attempt_at,last_error) VALUES(?1,'error',0,strftime('%Y-%m-%dT%H:%M:%SZ','now'),?2) ON CONFLICT(character_id) DO UPDATE SET status='error',role_verified=0,last_attempt_at=excluded.last_attempt_at,last_error=excluded.last_error",(character_id,error)).map_err(|e|e.to_string())?;
    Ok(())
}

pub fn sync_one(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
) -> Result<CorporationSyncResult, String> {
    sync_one_with(
        conn,
        client_id,
        character_id,
        &LiveCorporationSnapshotSource,
    )
}
pub fn sync_all(conn: &Connection, client_id: &str) -> Result<Vec<CorporationSyncResult>, String> {
    sync_all_with(conn, client_id, &LiveCorporationSnapshotSource)
}

pub fn sync_one_with<S: CorporationSnapshotSource>(
    conn: &Connection,
    client_id: &str,
    character_id: i64,
    source: &S,
) -> Result<CorporationSyncResult, String> {
    if let Err(error) = ensure_scopes(conn, character_id) {
        let _ = mark_error(conn, character_id, &error);
        return Ok(CorporationSyncResult {
            character_id,
            corporation_id: None,
            corporation_name: None,
            status: "error".into(),
            role_verified: false,
            asset_count: 0,
            page_count: 0,
            error: Some(error),
        });
    }
    mark_attempt(conn, character_id)?;
    let snapshot = match source.fetch(client_id, character_id) {
        Ok(value) => value,
        Err(error) => {
            mark_error(conn, character_id, &error)?;
            return Ok(CorporationSyncResult {
                character_id,
                corporation_id: None,
                corporation_name: None,
                status: "error".into(),
                role_verified: false,
                asset_count: 0,
                page_count: 0,
                error: Some(error),
            });
        }
    };
    persist(conn, character_id, &snapshot)?;
    Ok(CorporationSyncResult {
        character_id,
        corporation_id: Some(snapshot.corporation_id),
        corporation_name: Some(snapshot.corporation_name.clone()),
        status: "success".into(),
        role_verified: true,
        asset_count: snapshot.assets.len() as i64,
        page_count: snapshot.page_count as i64,
        error: None,
    })
}

fn persist(
    conn: &Connection,
    character_id: i64,
    snapshot: &CorporationSnapshot,
) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| format!("failed to begin corporation asset snapshot: {e}"))?;
    let result = (|| -> Result<(), String> {
        conn.execute("INSERT INTO corporations(corporation_id,name,authorizing_character_id,updated_at) VALUES(?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%SZ','now')) ON CONFLICT(corporation_id) DO UPDATE SET name=excluded.name,authorizing_character_id=excluded.authorizing_character_id,updated_at=excluded.updated_at",(snapshot.corporation_id,&snapshot.corporation_name,character_id)).map_err(|e|e.to_string())?;
        conn.execute(
            "DELETE FROM corporation_divisions WHERE corporation_id=?1",
            [snapshot.corporation_id],
        )
        .map_err(|e| e.to_string())?;
        for division in &snapshot.divisions {
            conn.execute("INSERT INTO corporation_divisions(corporation_id,division_number,name,synced_at) VALUES(?1,?2,?3,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",(snapshot.corporation_id,division.number,&division.name)).map_err(|e|e.to_string())?;
        }
        conn.execute(
            "DELETE FROM corporation_assets WHERE corporation_id=?1",
            [snapshot.corporation_id],
        )
        .map_err(|e| e.to_string())?;
        for asset in &snapshot.assets {
            let (number, name) = division_for_asset(asset, &snapshot.assets, &snapshot.divisions);
            conn.execute("INSERT INTO corporation_assets(corporation_id,item_id,type_id,quantity,location_id,location_type,location_flag,is_singleton,division_number,division_name,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",(snapshot.corporation_id,asset.item_id,asset.type_id,asset.quantity,asset.location_id,&asset.location_type,&asset.location_flag,asset.is_singleton as i64,number,name)).map_err(|e|format!("failed to persist corporation asset {}: {e}",asset.item_id))?;
        }
        conn.execute("INSERT INTO corporation_asset_sync_state(corporation_id,authorizing_character_id,status,required_role,role_verified,last_attempt_at,last_success_at,asset_count,page_count,last_error) VALUES(?1,?2,'success','Director',1,strftime('%Y-%m-%dT%H:%M:%SZ','now'),strftime('%Y-%m-%dT%H:%M:%SZ','now'),?3,?4,NULL) ON CONFLICT(corporation_id) DO UPDATE SET authorizing_character_id=excluded.authorizing_character_id,status='success',role_verified=1,last_attempt_at=excluded.last_attempt_at,last_success_at=excluded.last_success_at,asset_count=excluded.asset_count,page_count=excluded.page_count,last_error=NULL",(snapshot.corporation_id,character_id,snapshot.assets.len() as i64,snapshot.page_count as i64)).map_err(|e|e.to_string())?;
        conn.execute("INSERT INTO corporation_character_sync_state(character_id,corporation_id,corporation_name,status,required_role,role_verified,last_attempt_at,last_success_at,asset_count,page_count,last_error) VALUES(?1,?2,?3,'success','Director',1,strftime('%Y-%m-%dT%H:%M:%SZ','now'),strftime('%Y-%m-%dT%H:%M:%SZ','now'),?4,?5,NULL) ON CONFLICT(character_id) DO UPDATE SET corporation_id=excluded.corporation_id,corporation_name=excluded.corporation_name,status='success',role_verified=1,last_attempt_at=excluded.last_attempt_at,last_success_at=excluded.last_success_at,asset_count=excluded.asset_count,page_count=excluded.page_count,last_error=NULL",(character_id,snapshot.corporation_id,&snapshot.corporation_name,snapshot.assets.len() as i64,snapshot.page_count as i64)).map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(error);
    }
    conn.execute_batch("COMMIT;")
        .map_err(|e| format!("failed to commit corporation asset snapshot: {e}"))?;
    Ok(())
}

pub fn sync_all_with<S: CorporationSnapshotSource>(
    conn: &Connection,
    client_id: &str,
    source: &S,
) -> Result<Vec<CorporationSyncResult>, String> {
    let mut stmt=conn.prepare("SELECT character_id FROM characters WHERE is_demo=0 AND enabled=1 ORDER BY character_id").map_err(|e|e.to_string())?;
    let ids = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(ids
        .into_iter()
        .map(|id| {
            sync_one_with(conn, client_id, id, source).unwrap_or_else(|error| {
                CorporationSyncResult {
                    character_id: id,
                    corporation_id: None,
                    corporation_name: None,
                    status: "error".into(),
                    role_verified: false,
                    asset_count: 0,
                    page_count: 0,
                    error: Some(error),
                }
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esi::{
        assets::AssetRecord,
        corporation_assets::{CorporationSnapshot, Division},
    };
    use std::collections::HashMap;
    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE characters(character_id INTEGER PRIMARY KEY,name TEXT,is_demo INTEGER,scopes_granted TEXT,enabled INTEGER);CREATE TABLE corporations(corporation_id INTEGER PRIMARY KEY,name TEXT,authorizing_character_id INTEGER,updated_at TEXT);CREATE TABLE corporation_asset_sync_state(corporation_id INTEGER PRIMARY KEY,authorizing_character_id INTEGER,status TEXT,required_role TEXT,role_verified INTEGER,last_attempt_at TEXT,last_success_at TEXT,asset_count INTEGER,page_count INTEGER,last_error TEXT);CREATE TABLE corporation_character_sync_state(character_id INTEGER PRIMARY KEY,corporation_id INTEGER,corporation_name TEXT,status TEXT,required_role TEXT,role_verified INTEGER,last_attempt_at TEXT,last_success_at TEXT,asset_count INTEGER,page_count INTEGER,last_error TEXT);CREATE TABLE corporation_divisions(corporation_id INTEGER,division_number INTEGER,name TEXT,synced_at TEXT,PRIMARY KEY(corporation_id,division_number));CREATE TABLE corporation_assets(corporation_id INTEGER,item_id INTEGER,type_id INTEGER,quantity INTEGER,location_id INTEGER,location_type TEXT,location_flag TEXT,is_singleton INTEGER,division_number INTEGER,division_name TEXT,synced_at TEXT,PRIMARY KEY(corporation_id,item_id));CREATE TABLE character_assets(character_id INTEGER,item_id INTEGER,type_id INTEGER,quantity INTEGER);").unwrap();
        c
    }
    fn add(c: &Connection, id: i64, enabled: bool) {
        c.execute(
            "INSERT INTO characters VALUES(?1,'Pilot',0,?2,?3)",
            (
                id,
                format!("{ASSET_SCOPE} {ROLE_SCOPE} {DIVISION_SCOPE}"),
                enabled as i64,
            ),
        )
        .unwrap();
    }
    fn snap(corp: i64, item: i64) -> CorporationSnapshot {
        CorporationSnapshot {
            corporation_id: corp,
            corporation_name: format!("Corp {corp}"),
            divisions: vec![Division {
                number: 1,
                name: "Industry".into(),
            }],
            assets: vec![AssetRecord {
                item_id: item,
                type_id: 34,
                quantity: 10,
                location_id: 600,
                location_type: "station".into(),
                location_flag: "CorpSAG1".into(),
                is_singleton: false,
            }],
            page_count: 2,
        }
    }
    struct Fake(HashMap<i64, Result<CorporationSnapshot, String>>);
    impl CorporationSnapshotSource for Fake {
        fn fetch(&self, _: &str, id: i64) -> Result<CorporationSnapshot, String> {
            self.0.get(&id).unwrap().clone()
        }
    }
    #[test]
    fn multiple_corporations_sync_and_remain_separate() {
        let c = db();
        add(&c, 1, true);
        add(&c, 2, true);
        let f = Fake(HashMap::from([
            (1, Ok(snap(10, 100))),
            (2, Ok(snap(20, 200))),
        ]));
        let r = sync_all_with(&c, "client", &f).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(
            c.query_row(
                "SELECT COUNT(DISTINCT corporation_id) FROM corporation_assets",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM character_assets", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn disabled_characters_are_excluded() {
        let c = db();
        add(&c, 1, true);
        add(&c, 2, false);
        let f = Fake(HashMap::from([(1, Ok(snap(10, 100)))]));
        assert_eq!(sync_all_with(&c, "client", &f).unwrap().len(), 1)
    }
    #[test]
    fn failed_sync_preserves_prior_snapshot_and_surfaces_role_error() {
        let c = db();
        add(&c, 1, true);
        let ok = Fake(HashMap::from([(1, Ok(snap(10, 100)))]));
        sync_one_with(&c, "client", 1, &ok).unwrap();
        let bad = Fake(HashMap::from([(
            1,
            Err("character lacks the required Director role".into()),
        )]));
        let r = sync_one_with(&c, "client", 1, &bad).unwrap();
        assert_eq!(r.status, "error");
        assert!(r.error.unwrap().contains("Director"));
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM corporation_assets", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        )
    }
    #[test]
    fn missing_scope_requires_reauthorization_without_fetch() {
        let c = db();
        c.execute(
            "INSERT INTO characters VALUES(1,'Pilot',0,'esi-assets.read_assets.v1',1)",
            [],
        )
        .unwrap();
        let f = Fake(HashMap::new());
        let r = sync_one_with(&c, "client", 1, &f).unwrap();
        assert!(r.error.unwrap().contains("reauthorization required"));
    }
}

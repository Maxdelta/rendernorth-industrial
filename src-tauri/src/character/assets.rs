//! Character asset snapshot orchestration. Network fetch completes before
//! the replacement transaction begins, so failed syncs preserve the last
//! successful snapshot in full.

use crate::esi::assets::AssetRecord;
use rusqlite::Connection;
use serde::Serialize;

pub trait AssetSnapshotSource {
    fn fetch(&self, client_id: &str, character_id: i64) -> Result<(Vec<AssetRecord>, u32), String>;
}

pub struct LiveAssetSnapshotSource;
impl AssetSnapshotSource for LiveAssetSnapshotSource {
    fn fetch(&self, client_id: &str, character_id: i64) -> Result<(Vec<AssetRecord>, u32), String> {
        crate::esi::assets::fetch_character_assets(client_id, character_id)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub character_id: i64,
    pub status: String,
    pub asset_count: i64,
    pub page_count: i64,
    pub error: Option<String>,
}

pub fn global_owned_quantity(conn: &Connection, type_id: i64) -> Result<(i64, &'static str), String> {
    let manual: i64 = conn.query_row(
        "SELECT COALESCE(SUM(quantity),0) FROM manual_inventory_entries WHERE type_id=?1",
        [type_id], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let esi: i64 = conn.query_row(
        "SELECT COALESCE(SUM(a.quantity),0) FROM character_assets a
         JOIN characters c ON c.character_id=a.character_id
         WHERE a.type_id=?1 AND c.enabled=1 AND c.is_demo=0",
        [type_id], |r| r.get(0),
    ).map_err(|e| e.to_string())?;
    let source = match (manual > 0, esi > 0) {
        (true, true) => "Manual Inventory + ESI Character Assets",
        (false, true) => "ESI Character Assets",
        _ => "Manual Inventory",
    };
    Ok((manual + esi, source))
}

pub fn sync_one(conn: &Connection, client_id: &str, character_id: i64) -> Result<SyncResult, String> {
    sync_one_with(conn, client_id, character_id, &LiveAssetSnapshotSource)
}

pub fn sync_all(conn: &Connection, client_id: &str) -> Result<Vec<SyncResult>, String> {
    sync_all_with(conn, client_id, &LiveAssetSnapshotSource)
}

fn ensure_syncable(conn: &Connection, character_id: i64) -> Result<(), String> {
    let (is_demo, scopes): (i64, String) = conn
        .query_row(
            "SELECT is_demo, scopes_granted FROM characters WHERE character_id = ?1",
            [character_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("character {character_id} is not connected"))?;
    if is_demo != 0 {
        return Err("demo characters cannot synchronize ESI assets".into());
    }
    if !scopes.split_whitespace().any(|s| s == "esi-assets.read_assets.v1") {
        return Err("reauthorization required: use Add Character again to grant esi-assets.read_assets.v1".into());
    }
    Ok(())
}

pub fn sync_one_with<S: AssetSnapshotSource>(conn: &Connection, client_id: &str, character_id: i64, source: &S) -> Result<SyncResult, String> {
    ensure_syncable(conn, character_id)?;
    conn.execute(
        "INSERT INTO character_asset_sync_state (character_id, status, last_attempt_at)
         VALUES (?1, 'syncing', strftime('%Y-%m-%dT%H:%M:%SZ','now'))
         ON CONFLICT(character_id) DO UPDATE SET status='syncing', last_attempt_at=excluded.last_attempt_at, last_error=NULL",
        [character_id],
    ).map_err(|e| e.to_string())?;

    let (assets, pages) = match source.fetch(client_id, character_id) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            conn.execute(
                "UPDATE character_asset_sync_state SET status='error', last_error=?2 WHERE character_id=?1",
                (character_id, &error),
            ).map_err(|e| e.to_string())?;
            return Ok(SyncResult { character_id, status: "error".into(), asset_count: 0, page_count: 0, error: Some(error) });
        }
    };

    conn.execute_batch("BEGIN IMMEDIATE;").map_err(|e| format!("failed to begin asset snapshot transaction: {e}"))?;
    let result: Result<(), String> = (|| {
        conn.execute("DELETE FROM character_assets WHERE character_id=?1", [character_id]).map_err(|e| e.to_string())?;
        for asset in &assets {
            conn.execute(
                "INSERT INTO character_assets
                 (character_id,item_id,type_id,quantity,location_id,location_type,location_flag,is_singleton,synced_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
                (character_id, asset.item_id, asset.type_id, asset.quantity, asset.location_id,
                 &asset.location_type, &asset.location_flag, asset.is_singleton as i64),
            ).map_err(|e| format!("failed to persist asset {}: {e}", asset.item_id))?;
        }
        conn.execute(
            "UPDATE character_asset_sync_state SET status='success',
             last_success_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'), asset_count=?2, page_count=?3, last_error=NULL
             WHERE character_id=?1",
            (character_id, assets.len() as i64, pages as i64),
        ).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = conn.execute_batch("ROLLBACK;");
        let _ = conn.execute("UPDATE character_asset_sync_state SET status='error', last_error=?2 WHERE character_id=?1", (character_id, &error));
        return Err(error);
    }
    conn.execute_batch("COMMIT;").map_err(|e| format!("failed to commit asset snapshot: {e}"))?;
    Ok(SyncResult { character_id, status: "success".into(), asset_count: assets.len() as i64, page_count: pages as i64, error: None })
}

pub fn sync_all_with<S: AssetSnapshotSource>(conn: &Connection, client_id: &str, source: &S) -> Result<Vec<SyncResult>, String> {
    let mut stmt = conn.prepare("SELECT character_id FROM characters WHERE is_demo=0 AND enabled=1 ORDER BY character_id").map_err(|e| e.to_string())?;
    let ids = stmt.query_map([], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    Ok(ids.into_iter().map(|id| match sync_one_with(conn, client_id, id, source) {
        Ok(result) => result,
        Err(error) => SyncResult { character_id: id, status: "error".into(), asset_count: 0, page_count: 0, error: Some(error) },
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE characters(character_id INTEGER PRIMARY KEY,name TEXT,is_demo INTEGER,scopes_granted TEXT,enabled INTEGER);
             CREATE TABLE manual_inventory_entries(id INTEGER PRIMARY KEY,type_id INTEGER,quantity INTEGER);
             CREATE TABLE character_asset_sync_state(character_id INTEGER PRIMARY KEY REFERENCES characters(character_id) ON DELETE CASCADE,status TEXT NOT NULL DEFAULT 'never',last_attempt_at TEXT,last_success_at TEXT,asset_count INTEGER NOT NULL DEFAULT 0,page_count INTEGER NOT NULL DEFAULT 0,last_error TEXT);
             CREATE TABLE character_assets(character_id INTEGER REFERENCES characters(character_id) ON DELETE CASCADE,item_id INTEGER,type_id INTEGER,quantity INTEGER,location_id INTEGER,location_type TEXT,location_flag TEXT,is_singleton INTEGER,synced_at TEXT,PRIMARY KEY(character_id,item_id));"
        ).unwrap();
        conn
    }

    fn add_character(conn: &Connection, id: i64, enabled: bool, demo: bool) {
        conn.execute("INSERT INTO characters VALUES(?1,?2,?3,'esi-assets.read_assets.v1',?4)", (id, format!("Pilot {id}"), demo as i64, enabled as i64)).unwrap();
    }
    fn asset(item_id: i64, type_id: i64, quantity: i64, location_id: i64) -> AssetRecord {
        AssetRecord { item_id, type_id, quantity, location_id, location_type:"station".into(), location_flag:"Hangar".into(), is_singleton:false }
    }
    struct Fake { snapshots: HashMap<i64, Result<(Vec<AssetRecord>,u32),String>> }
    impl AssetSnapshotSource for Fake {
        fn fetch(&self, _client_id:&str, character_id:i64)->Result<(Vec<AssetRecord>,u32),String>{self.snapshots.get(&character_id).unwrap().clone()}
    }

    #[test]
    fn one_character_sync_persists_snapshot_and_metadata() {
        let conn=db(); add_character(&conn,1,true,false);
        let source=Fake{snapshots:HashMap::from([(1,Ok((vec![asset(10,34,7,600)],2)))])};
        let result=sync_one_with(&conn,"client",1,&source).unwrap();
        assert_eq!((result.asset_count,result.page_count),(1,2));
        assert_eq!(conn.query_row("SELECT quantity FROM character_assets WHERE character_id=1",[],|r|r.get::<_,i64>(0)).unwrap(),7);
    }

    #[test]
    fn sync_all_excludes_disabled_characters() {
        let conn=db(); add_character(&conn,1,true,false); add_character(&conn,2,false,false);
        let source=Fake{snapshots:HashMap::from([(1,Ok((vec![],1)))])};
        let results=sync_all_with(&conn,"client",&source).unwrap();
        assert_eq!(results.len(),1); assert_eq!(results[0].character_id,1);
    }

    #[test]
    fn failed_sync_preserves_prior_successful_snapshot() {
        let conn=db(); add_character(&conn,1,true,false);
        conn.execute("INSERT INTO character_assets VALUES(1,10,34,99,600,'station','Hangar',0,'old')",[]).unwrap();
        let source=Fake{snapshots:HashMap::from([(1,Err("network failure".into()))])};
        let result=sync_one_with(&conn,"client",1,&source).unwrap();
        assert_eq!(result.status,"error");
        assert_eq!(conn.query_row("SELECT quantity FROM character_assets",[],|r|r.get::<_,i64>(0)).unwrap(),99);
    }

    #[test]
    fn duplicate_type_ids_across_characters_and_separate_locations_remain_separate_stacks() {
        let conn=db(); add_character(&conn,1,true,false); add_character(&conn,2,true,false);
        let source=Fake{snapshots:HashMap::from([
            (1,Ok((vec![asset(10,34,5,600),asset(11,34,6,601)],1))),
            (2,Ok((vec![asset(20,34,7,600)],1)))])};
        sync_all_with(&conn,"client",&source).unwrap();
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM character_assets WHERE type_id=34",[],|r|r.get::<_,i64>(0)).unwrap(),3);
        assert_eq!(conn.query_row("SELECT COUNT(DISTINCT location_id) FROM character_assets WHERE character_id=1",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    }

    #[test]
    fn global_aggregation_combines_manual_and_enabled_esi_but_not_demo_or_disabled() {
        let conn=db(); add_character(&conn,1,true,false); add_character(&conn,2,false,false); add_character(&conn,-1,true,true);
        conn.execute("INSERT INTO manual_inventory_entries(type_id,quantity) VALUES(34,10)",[]).unwrap();
        for (cid,item,qty) in [(1,10,20),(2,20,200),(-1,30,300)] { conn.execute("INSERT INTO character_assets VALUES(?1,?2,34,?3,600,'station','Hangar',0,'now')",(cid,item,qty)).unwrap(); }
        let (total,source)=global_owned_quantity(&conn,34).unwrap();
        assert_eq!(total,30); assert_eq!(source,"Manual Inventory + ESI Character Assets");
    }

    #[test]
    fn duplicate_type_ids_across_characters_aggregate_globally() {
        let conn=db(); add_character(&conn,1,true,false); add_character(&conn,2,true,false);
        conn.execute("INSERT INTO character_assets VALUES(1,10,34,5,600,'station','Hangar',0,'now')",[]).unwrap();
        conn.execute("INSERT INTO character_assets VALUES(2,20,34,7,601,'station','Hangar',0,'now')",[]).unwrap();
        assert_eq!(global_owned_quantity(&conn,34).unwrap().0,12);
    }

    #[test]
    fn manual_and_esi_sources_remain_distinguishable() {
        let conn=db();
        conn.execute("INSERT INTO manual_inventory_entries(type_id,quantity) VALUES(34,10)",[]).unwrap();
        assert_eq!(global_owned_quantity(&conn,34).unwrap(),(10,"Manual Inventory"));
        add_character(&conn,1,true,false);
        conn.execute("INSERT INTO character_assets VALUES(1,10,35,5,600,'station','Hangar',0,'now')",[]).unwrap();
        assert_eq!(global_owned_quantity(&conn,35).unwrap(),(5,"ESI Character Assets"));
    }

    #[test]
    fn demo_inventory_never_contributes_to_global_owned() {
        let conn=db(); add_character(&conn,-1,true,true);
        conn.execute("INSERT INTO character_assets VALUES(-1,10,34,999,600,'station','Hangar',0,'now')",[]).unwrap();
        assert_eq!(global_owned_quantity(&conn,34).unwrap().0,0);
    }
}

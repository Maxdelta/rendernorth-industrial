//! Transactional ESI character blueprint snapshots. Fetch completes before
//! replacement begins, preserving the previous successful snapshot on error.

use crate::esi::blueprints::EsiBlueprint;
use rusqlite::Connection;
use serde::Serialize;

pub trait BlueprintSnapshotSource {
    fn fetch(&self, client_id:&str, character_id:i64)->Result<(Vec<EsiBlueprint>,u32),String>;
}
pub struct LiveBlueprintSnapshotSource;
impl BlueprintSnapshotSource for LiveBlueprintSnapshotSource {
    fn fetch(&self, client_id:&str, character_id:i64)->Result<(Vec<EsiBlueprint>,u32),String>{crate::esi::blueprints::fetch_character_blueprints(client_id,character_id)}
}

#[derive(Debug,Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct BlueprintSyncResult { pub character_id:i64,pub status:String,pub blueprint_count:i64,pub page_count:i64,pub error:Option<String> }

fn ensure_syncable(conn:&Connection,character_id:i64)->Result<(),String>{
    let (is_demo,scopes):(i64,String)=conn.query_row("SELECT is_demo,scopes_granted FROM characters WHERE character_id=?1",[character_id],|r|Ok((r.get(0)?,r.get(1)?)))
        .map_err(|_|format!("character {character_id} is not connected"))?;
    if is_demo!=0{return Err("demo characters cannot synchronize ESI blueprints".into())}
    if !scopes.split_whitespace().any(|s|s=="esi-characters.read_blueprints.v1"){
        return Err("reauthorization required: use Add / Reauthorize Character to grant esi-characters.read_blueprints.v1".into())
    }
    Ok(())
}

pub fn sync_one(conn:&Connection,client_id:&str,character_id:i64)->Result<BlueprintSyncResult,String>{sync_one_with(conn,client_id,character_id,&LiveBlueprintSnapshotSource)}
pub fn sync_all(conn:&Connection,client_id:&str)->Result<Vec<BlueprintSyncResult>,String>{sync_all_with(conn,client_id,&LiveBlueprintSnapshotSource)}

pub fn sync_one_with<S:BlueprintSnapshotSource>(conn:&Connection,client_id:&str,character_id:i64,source:&S)->Result<BlueprintSyncResult,String>{
    ensure_syncable(conn,character_id)?;
    conn.execute("INSERT INTO character_blueprint_sync_state(character_id,status,last_attempt_at) VALUES(?1,'syncing',strftime('%Y-%m-%dT%H:%M:%SZ','now')) ON CONFLICT(character_id) DO UPDATE SET status='syncing',last_attempt_at=excluded.last_attempt_at,last_error=NULL",[character_id]).map_err(|e|e.to_string())?;
    let (rows,pages)=match source.fetch(client_id,character_id){Ok(v)=>v,Err(error)=>{
        conn.execute("UPDATE character_blueprint_sync_state SET status='error',last_error=?2 WHERE character_id=?1",(character_id,&error)).map_err(|e|e.to_string())?;
        return Ok(BlueprintSyncResult{character_id,status:"error".into(),blueprint_count:0,page_count:0,error:Some(error)})
    }};
    conn.execute_batch("BEGIN IMMEDIATE;").map_err(|e|e.to_string())?;
    let result:Result<(),String>=(||{
        conn.execute("DELETE FROM character_blueprints WHERE character_id=?1",[character_id]).map_err(|e|e.to_string())?;
        for b in &rows{conn.execute("INSERT INTO character_blueprints(character_id,item_id,type_id,location_id,location_flag,quantity,material_efficiency,time_efficiency,runs,source,synced_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'ESI Character Blueprints',strftime('%Y-%m-%dT%H:%M:%SZ','now'))",(character_id,b.item_id,b.type_id,b.location_id,&b.location_flag,b.quantity,b.material_efficiency,b.time_efficiency,b.runs)).map_err(|e|format!("failed to persist blueprint {}: {e}",b.item_id))?;}
        conn.execute("UPDATE character_blueprint_sync_state SET status='success',last_success_at=strftime('%Y-%m-%dT%H:%M:%SZ','now'),blueprint_count=?2,page_count=?3,last_error=NULL WHERE character_id=?1",(character_id,rows.len() as i64,pages as i64)).map_err(|e|e.to_string())?;Ok(())})();
    if let Err(error)=result{let _=conn.execute_batch("ROLLBACK;");let _=conn.execute("UPDATE character_blueprint_sync_state SET status='error',last_error=?2 WHERE character_id=?1",(character_id,&error));return Err(error)}
    conn.execute_batch("COMMIT;").map_err(|e|e.to_string())?;
    Ok(BlueprintSyncResult{character_id,status:"success".into(),blueprint_count:rows.len() as i64,page_count:pages as i64,error:None})
}

pub fn sync_all_with<S:BlueprintSnapshotSource>(conn:&Connection,client_id:&str,source:&S)->Result<Vec<BlueprintSyncResult>,String>{
    let mut stmt=conn.prepare("SELECT character_id FROM characters WHERE is_demo=0 AND enabled=1 ORDER BY character_id").map_err(|e|e.to_string())?;
    let ids=stmt.query_map([],|r|r.get::<_,i64>(0)).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(ids.into_iter().map(|id|sync_one_with(conn,client_id,id,source).unwrap_or_else(|error|BlueprintSyncResult{character_id:id,status:"error".into(),blueprint_count:0,page_count:0,error:Some(error)})).collect())
}

#[cfg(test)]
mod tests{
 use super::*;use std::collections::HashMap;
 fn db()->Connection{let c=Connection::open_in_memory().unwrap();c.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE characters(character_id INTEGER PRIMARY KEY,name TEXT,is_demo INTEGER,scopes_granted TEXT,enabled INTEGER);CREATE TABLE character_blueprint_sync_state(character_id INTEGER PRIMARY KEY,status TEXT,last_attempt_at TEXT,last_success_at TEXT,blueprint_count INTEGER DEFAULT 0,page_count INTEGER DEFAULT 0,last_error TEXT);CREATE TABLE character_blueprints(character_id INTEGER,item_id INTEGER,type_id INTEGER,location_id INTEGER,location_flag TEXT,quantity INTEGER,material_efficiency INTEGER,time_efficiency INTEGER,runs INTEGER,source TEXT,synced_at TEXT,PRIMARY KEY(character_id,item_id));").unwrap();c}
 fn add(c:&Connection,id:i64,enabled:bool){c.execute("INSERT INTO characters VALUES(?1,'Pilot',0,'esi-assets.read_assets.v1 esi-characters.read_blueprints.v1',?2)",(id,enabled as i64)).unwrap();}
 fn bp(item:i64,typ:i64,runs:i64)->EsiBlueprint{EsiBlueprint{item_id:item,type_id:typ,location_id:600,location_flag:"Hangar".into(),quantity:if runs == -1{-1}else{-2},material_efficiency:10,time_efficiency:20,runs}}
 struct Fake(HashMap<i64,Result<(Vec<EsiBlueprint>,u32),String>>);impl BlueprintSnapshotSource for Fake{fn fetch(&self,_:&str,id:i64)->Result<(Vec<EsiBlueprint>,u32),String>{self.0.get(&id).unwrap().clone()}}
 #[test]fn one_character_sync_persists_me_te_and_runs(){let c=db();add(&c,1,true);let f=Fake(HashMap::from([(1,Ok((vec![bp(10,681,7)],2)))]));let r=sync_one_with(&c,"client",1,&f).unwrap();assert_eq!((r.blueprint_count,r.page_count),(1,2));let v:(i64,i64,i64)=c.query_row("SELECT material_efficiency,time_efficiency,runs FROM character_blueprints",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();assert_eq!(v,(10,20,7));}
 #[test]fn sync_all_excludes_disabled(){let c=db();add(&c,1,true);add(&c,2,false);let f=Fake(HashMap::from([(1,Ok((vec![],1)))]));let r=sync_all_with(&c,"client",&f).unwrap();assert_eq!(r.len(),1);assert_eq!(r[0].character_id,1)}
 #[test]fn failed_sync_preserves_previous_snapshot(){let c=db();add(&c,1,true);c.execute("INSERT INTO character_blueprints VALUES(1,10,681,600,'Hangar',-1,5,10,-1,'ESI Character Blueprints','old')",[]).unwrap();let f=Fake(HashMap::from([(1,Err("offline".into()))]));sync_one_with(&c,"client",1,&f).unwrap();assert_eq!(c.query_row("SELECT material_efficiency FROM character_blueprints",[],|r|r.get::<_,i64>(0)).unwrap(),5)}
 #[test]fn duplicate_types_and_item_ids_remain_distinguishable(){let c=db();add(&c,1,true);add(&c,2,true);let f=Fake(HashMap::from([(1,Ok((vec![bp(10,681,-1),bp(11,681,5)],1))),(2,Ok((vec![bp(10,681,-1)],1)))]));sync_all_with(&c,"client",&f).unwrap();assert_eq!(c.query_row("SELECT COUNT(*) FROM character_blueprints WHERE type_id=681",[],|r|r.get::<_,i64>(0)).unwrap(),3)}
 #[test]fn blueprint_records_never_write_inventory_assets(){let c=db();c.execute_batch("CREATE TABLE character_assets(character_id INTEGER,item_id INTEGER,type_id INTEGER);").unwrap();add(&c,1,true);let f=Fake(HashMap::from([(1,Ok((vec![bp(10,681,-1)],1)))]));sync_one_with(&c,"client",1,&f).unwrap();assert_eq!(c.query_row("SELECT COUNT(*) FROM character_assets",[],|r|r.get::<_,i64>(0)).unwrap(),0);assert_eq!(c.query_row("SELECT COUNT(*) FROM character_blueprints",[],|r|r.get::<_,i64>(0)).unwrap(),1)}
 #[test]fn missing_scope_requires_reauthorization_without_fetching(){let c=db();c.execute("INSERT INTO characters VALUES(1,'Pilot',0,'esi-assets.read_assets.v1',1)",[]).unwrap();let f=Fake(HashMap::new());let e=sync_one_with(&c,"client",1,&f).unwrap_err();assert!(e.contains("reauthorization required"));}
 #[test]fn esi_ownership_remains_separate_from_manual_and_reference_data(){let c=db();c.execute_batch("CREATE TABLE blueprints(blueprint_id INTEGER PRIMARY KEY,type_name TEXT);CREATE TABLE blueprint_products(blueprint_type_id INTEGER,product_type_id INTEGER);INSERT INTO blueprints VALUES(1,'Manual BP');INSERT INTO blueprint_products VALUES(681,999);").unwrap();add(&c,1,true);let f=Fake(HashMap::from([(1,Ok((vec![bp(10,681,-1)],1)))]));sync_one_with(&c,"client",1,&f).unwrap();assert_eq!(c.query_row("SELECT COUNT(*) FROM blueprints",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert_eq!(c.query_row("SELECT COUNT(*) FROM blueprint_products",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert_eq!(c.query_row("SELECT COUNT(*) FROM character_blueprints",[],|r|r.get::<_,i64>(0)).unwrap(),1);}
 #[test]fn bpo_and_bpc_rows_persist_with_distinct_run_semantics(){let c=db();add(&c,1,true);let f=Fake(HashMap::from([(1,Ok((vec![bp(10,681,-1),bp(11,681,4)],1)))]));sync_one_with(&c,"client",1,&f).unwrap();let values=c.prepare("SELECT quantity,runs FROM character_blueprints ORDER BY item_id").unwrap().query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();assert_eq!(values,vec![(-1,-1),(-2,4)]);}
}

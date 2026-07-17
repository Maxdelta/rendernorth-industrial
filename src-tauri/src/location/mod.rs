use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashSet;

const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all="camelCase")]
pub struct ResolvedLocation {
    pub raw_location_id:i64, pub display_name:String, pub location_kind:String,
    pub solar_system_name:Option<String>, pub constellation_name:Option<String>,
    pub region_name:Option<String>, pub container_path:Vec<String>,
    pub full_path:Vec<String>, pub resolution_status:String,
    pub resolution_source:String, pub last_resolved:Option<String>,
}

fn terminal(conn:&Connection,id:i64)->Result<Option<ResolvedLocation>,String>{
    let mut s=conn.prepare("SELECT location_kind,display_name,solar_system_name,constellation_name,region_name,resolution_status,resolution_source,resolved_at FROM location_cache WHERE location_id=?1").map_err(|e|e.to_string())?;
    match s.query_row([id],|r|Ok(ResolvedLocation{raw_location_id:id,location_kind:r.get(0)?,display_name:r.get(1)?,solar_system_name:r.get(2)?,constellation_name:r.get(3)?,region_name:r.get(4)?,container_path:vec![],full_path:vec![],resolution_status:r.get(5)?,resolution_source:r.get(6)?,last_resolved:r.get(7)?})){
        Ok(v)=>Ok(Some(v)),Err(rusqlite::Error::QueryReturnedNoRows)=>Ok(None),Err(e)=>Err(e.to_string())
    }
}

pub fn resolve(conn:&Connection,character_id:i64,location_id:i64)->Result<ResolvedLocation,String>{
    resolve_owned(conn,character_id,location_id,false)
}

pub fn resolve_corporation(conn:&Connection,corporation_id:i64,location_id:i64)->Result<ResolvedLocation,String>{
    resolve_owned(conn,corporation_id,location_id,true)
}

fn resolve_owned(conn:&Connection,character_id:i64,location_id:i64,corporation:bool)->Result<ResolvedLocation,String>{
    let raw=location_id; let mut current=location_id; let mut seen=HashSet::new(); let mut parents=Vec::new();
    for _ in 0..MAX_DEPTH {
        if !seen.insert(current){return Ok(fallback(raw,parents,"cycle_detected"));}
        if let Some(mut t)=terminal(conn,current)? { parents.reverse(); t.container_path=parents.clone(); t.full_path=vec![t.display_name.clone()];t.full_path.extend(parents);return Ok(t); }
        let sql=if corporation{"SELECT a.location_id,COALESCE(t.name,'Asset '||a.item_id) FROM corporation_assets a LEFT JOIN eve_types t ON t.type_id=a.type_id WHERE a.corporation_id=?1 AND a.item_id=?2"}else{"SELECT a.location_id,COALESCE(t.name,'Asset '||a.item_id) FROM character_assets a LEFT JOIN eve_types t ON t.type_id=a.type_id WHERE a.character_id=?1 AND a.item_id=?2"};
        let parent:Result<(i64,String),_>=conn.query_row(sql,(character_id,current),|r|Ok((r.get(0)?,r.get(1)?)));
        match parent { Ok((id,name))=>{parents.push(name);current=id}, Err(rusqlite::Error::QueryReturnedNoRows)=>return Ok(fallback(raw,parents,"missing_parent_asset")), Err(e)=>return Err(e.to_string()) }
    }
    Ok(fallback(raw,parents,"unsupported_location_type"))
}
fn fallback(id:i64,mut path:Vec<String>,status:&str)->ResolvedLocation{path.reverse();let name=if id>=1_000_000_000_000{format!("Inaccessible Structure (ID {id})")}else{format!("Unresolved Location (ID {id})")};let mut full=vec![name.clone()];full.extend(path.clone());ResolvedLocation{raw_location_id:id,display_name:name,location_kind:"unresolved".into(),solar_system_name:None,constellation_name:None,region_name:None,container_path:path,full_path:full,resolution_status:status.into(),resolution_source:"raw_id".into(),last_resolved:None}}

#[derive(Serialize)] #[serde(rename_all="camelCase")] pub struct RefreshResult{pub resolved:i64,pub inaccessible:i64,pub errors:Vec<String>}
fn get_json(http:&reqwest::Client,url:String,token:Option<&str>)->Result<serde_json::Value,String>{let mut req=http.get(url).header("X-Compatibility-Date","2026-07-12").header("User-Agent","RenderNorthIndustrial/0.1");if let Some(t)=token{req=req.bearer_auth(t)}let resp=tauri::async_runtime::block_on(req.send()).map_err(|e|e.to_string())?;let status=resp.status();let body=tauri::async_runtime::block_on(resp.text()).map_err(|e|e.to_string())?;if !status.is_success(){return Err(format!("ESI returned {status}: {body}"))}serde_json::from_str(&body).map_err(|e|e.to_string())}
fn text(v:&serde_json::Value,key:&str)->Option<String>{v.get(key)?.as_str().map(str::to_owned)} fn num(v:&serde_json::Value,key:&str)->Option<i64>{v.get(key)?.as_i64()}
fn upsert(conn:&Connection,id:i64,kind:&str,name:&str,system_id:Option<i64>,system:Option<&str>,constellation_id:Option<i64>,constellation:Option<&str>,region_id:Option<i64>,region:Option<&str>,status:&str,source:&str,error:Option<&str>)->Result<(),String>{conn.execute("INSERT INTO location_cache(location_id,location_kind,display_name,solar_system_id,solar_system_name,constellation_id,constellation_name,region_id,region_name,resolution_status,resolution_source,resolved_at,last_error) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,strftime('%Y-%m-%dT%H:%M:%SZ','now'),?12) ON CONFLICT(location_id) DO UPDATE SET location_kind=excluded.location_kind,display_name=excluded.display_name,solar_system_id=excluded.solar_system_id,solar_system_name=excluded.solar_system_name,constellation_id=excluded.constellation_id,constellation_name=excluded.constellation_name,region_id=excluded.region_id,region_name=excluded.region_name,resolution_status=excluded.resolution_status,resolution_source=excluded.resolution_source,resolved_at=excluded.resolved_at,last_error=excluded.last_error",(id,kind,name,system_id,system,constellation_id,constellation,region_id,region,status,source,error)).map_err(|e|e.to_string())?;Ok(())}
fn map_chain(http:&reqwest::Client,system_id:i64)->Result<(String,i64,String,i64,String),String>{let s=get_json(http,format!("https://esi.evetech.net/universe/systems/{system_id}"),None)?;let sid=num(&s,"constellation_id").ok_or("system lacks constellation_id")?;let sn=text(&s,"name").ok_or("system lacks name")?;let c=get_json(http,format!("https://esi.evetech.net/universe/constellations/{sid}"),None)?;let rid=num(&c,"region_id").ok_or("constellation lacks region_id")?;let cn=text(&c,"name").ok_or("constellation lacks name")?;let r=get_json(http,format!("https://esi.evetech.net/universe/regions/{rid}"),None)?;let rn=text(&r,"name").ok_or("region lacks name")?;Ok((sn,sid,cn,rid,rn))}
pub fn refresh(conn:&Connection,client_id:&str)->Result<RefreshResult,String>{
    let mut stmt=conn.prepare("SELECT DISTINCT character_id,location_id,location_type FROM character_assets UNION SELECT DISTINCT character_id,location_id,'item' FROM character_blueprints UNION SELECT DISTINCT c.authorizing_character_id,a.location_id,a.location_type FROM corporation_assets a JOIN corporations c ON c.corporation_id=a.corporation_id WHERE c.authorizing_character_id IS NOT NULL").map_err(|e|e.to_string())?;
    let candidates=stmt.query_map([],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let http=reqwest::Client::new();let mut done=HashSet::new();let(mut resolved,mut inaccessible,mut errors)=(0,0,vec![]);
    for(character_id,id,kind)in candidates{
        if !done.insert(id){continue}
        let fresh=conn.query_row("SELECT 1 FROM location_cache WHERE location_id=?1 AND resolution_status='resolved' AND resolved_at >= datetime('now','-24 hours')",[id],|r|r.get::<_,i64>(0)).is_ok();if fresh{continue}
        let personal_parent=conn.query_row("SELECT 1 FROM character_assets WHERE character_id=?1 AND item_id=?2",(character_id,id),|r|r.get::<_,i64>(0)).is_ok();
        let corporation_parent=conn.query_row("SELECT 1 FROM corporation_assets WHERE item_id=?1 LIMIT 1",[id],|r|r.get::<_,i64>(0)).is_ok();
        if personal_parent||corporation_parent{continue}
        let result=(||->Result<(),String>{
            let(name,system_id,source,actual_kind)=if (60_000_000..64_000_000).contains(&id){let v=get_json(&http,format!("https://esi.evetech.net/universe/stations/{id}"),None)?;(text(&v,"name").ok_or("station lacks name")?,num(&v,"system_id").ok_or("station lacks system_id")?,"esi_public","station")}else if (30_000_000..40_000_000).contains(&id){let v=get_json(&http,format!("https://esi.evetech.net/universe/systems/{id}"),None)?;(text(&v,"name").ok_or("system lacks name")?,id,"esi_public","solar_system")}else if id>=1_000_000_000_000{let refresh=crate::esi::token_store::get_refresh_token(character_id)?;let tokens=tauri::async_runtime::block_on(crate::esi::client::refresh_access_token(&http,client_id,&refresh))?;crate::esi::token_store::store_refresh_token(character_id,&tokens.refresh_token)?;let v=get_json(&http,format!("https://esi.evetech.net/universe/structures/{id}"),Some(&tokens.access_token))?;(text(&v,"name").ok_or("structure lacks name")?,num(&v,"solar_system_id").ok_or("structure lacks solar_system_id")?,"esi_authenticated","structure")}else{return Err(format!("unsupported {kind} location {id}"))};
            let(sn,cid,cn,rid,rn)=map_chain(&http,system_id)?;upsert(conn,id,actual_kind,&name,Some(system_id),Some(&sn),Some(cid),Some(&cn),Some(rid),Some(&rn),"resolved",source,None)?;Ok(())
        })();
        match result{Ok(())=>resolved+=1,Err(e)=>{let status=if id>=1_000_000_000_000{"inaccessible_structure"}else{"unresolved"};let name=if id>=1_000_000_000_000{format!("Inaccessible Structure (ID {id})")}else{format!("Unresolved Location (ID {id})")};upsert(conn,id,&kind,&name,None,None,None,None,None,None,status,"esi",Some(&e))?;if status=="inaccessible_structure"{inaccessible+=1}errors.push(format!("{id}: {e}"));}}
    }
    Ok(RefreshResult{resolved,inaccessible,errors})
}

#[cfg(test)] mod tests{use super::*;fn db()->Connection{let c=Connection::open_in_memory().unwrap();c.execute_batch("CREATE TABLE location_cache(location_id INTEGER PRIMARY KEY,location_kind TEXT,display_name TEXT,solar_system_name TEXT,constellation_name TEXT,region_name TEXT,resolution_status TEXT,resolution_source TEXT,resolved_at TEXT);CREATE TABLE character_assets(character_id INTEGER,item_id INTEGER,type_id INTEGER,location_id INTEGER);CREATE TABLE eve_types(type_id INTEGER PRIMARY KEY,name TEXT);INSERT INTO location_cache VALUES(6001,'station','Station','System','Constellation','Region','resolved','esi_public','now');INSERT INTO eve_types VALUES(10,'Container'),(20,'Ship');").unwrap();c}
#[test]fn station_and_map_hierarchy(){let c=db();let r=resolve(&c,1,6001).unwrap();assert_eq!((&r.display_name,r.solar_system_name.as_deref(),r.constellation_name.as_deref(),r.region_name.as_deref()),((&"Station".into()),Some("System"),Some("Constellation"),Some("Region")));}
#[test]fn multi_level_parent_path(){let c=db();c.execute_batch("INSERT INTO character_assets VALUES(1,101,10,6001),(1,102,20,101);").unwrap();let r=resolve(&c,1,102).unwrap();assert_eq!(r.full_path,vec!["Station","Container","Ship"]);}
#[test]fn missing_parent_is_explicit_and_raw_preserved(){let c=db();let r=resolve(&c,1,999).unwrap();assert_eq!(r.raw_location_id,999);assert_eq!(r.resolution_status,"missing_parent_asset");}
#[test]fn cycle_is_detected(){let c=db();c.execute_batch("INSERT INTO character_assets VALUES(1,1,10,2),(1,2,20,1);").unwrap();assert_eq!(resolve(&c,1,1).unwrap().resolution_status,"cycle_detected");}
#[test]fn structure_is_honest_when_inaccessible(){let c=db();assert!(resolve(&c,1,1036732971380).unwrap().display_name.starts_with("Inaccessible Structure"));}}

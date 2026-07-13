import { useEffect, useMemo, useState } from "react";
import { getBlueprintSummary,listBlueprints,getMissingBlueprintReport,type BlueprintSummary,type BlueprintRecord,type MissingBlueprintReport } from "../lib/backend";
import { Panel } from "../components/Panel";import { StatCard } from "../components/StatCard";
import {LocationDisplay} from "../components/LocationDisplay";

type Filter="All Owned"|"ESI Character Blueprints"|"Manual Ownership"|"CCP Reference Data";
export function BlueprintsPage(){
 const [summary,setSummary]=useState<BlueprintSummary|null>(null);const [rows,setRows]=useState<BlueprintRecord[]>([]);const [missing,setMissing]=useState<MissingBlueprintReport[]>([]);const [error,setError]=useState<string|null>(null);const [loading,setLoading]=useState(true);const [filter,setFilter]=useState<Filter>("All Owned");
 async function refresh(){setLoading(true);setError(null);try{const [s,b,m]=await Promise.all([getBlueprintSummary(),listBlueprints(),getMissingBlueprintReport()]);setSummary(s);setRows(b);setMissing(m);}catch(err){setError(String(err));}finally{setLoading(false)}}
 useEffect(()=>{refresh()},[]);
 const shown=useMemo(()=>rows.filter(r=>filter==="All Owned"?r.isOwned:r.source===filter),[rows,filter]);
 return <div className="dash">
  {error&&<div className="sd-error"><div className="conflict-desc">Failed to load blueprints: {error}</div></div>}
  {summary&&<div className="stat-strip"><StatCard label="Total Blueprints" value={String(summary.totalBlueprints)} tone="coolant" keel="coolant" note="owned BPOs and BPCs"/><StatCard label="BPO Count" value={String(summary.bpoCount)} tone="nominal" keel="nominal" note="originals owned"/><StatCard label="BPC Count" value={String(summary.bpcCount)} tone="coolant" keel="coolant" note="copies owned"/><StatCard label="Research Complete" value={String(summary.researchComplete)} tone="nominal" keel="nominal" note="available ownership records"/><StatCard label="Copies" value={String(summary.copies)} tone="coolant" keel="coolant" note="remaining BPC runs"/><StatCard label="Missing For Operations" value={String(summary.missingForOperations)} tone={summary.missingForOperations>0?"alert":"nominal"} keel={summary.missingForOperations>0?"alert":"nominal"} note="required types not owned"/></div>}
  <Panel title="Blueprint Library" keel="furnace" className="dash-hero" headerRight={<button className="target-select enabled" onClick={refresh} disabled={loading}>{loading?"Loading…":"Refresh"}</button>}>
   <div className="new-op-actions">{(["All Owned","ESI Character Blueprints","Manual Ownership","CCP Reference Data"] as Filter[]).map(f=><button key={f} className={`target-select ${filter===f?"enabled":""}`} onClick={()=>setFilter(f)}>{f}</button>)}</div>
   {!error&&!loading&&shown.length===0&&<p className="ph-mission">No blueprints in this source.</p>}
   {!error&&shown.length>0&&<div className="inv-table" style={{overflowX:"auto"}}><div className="inv-row" style={{gridTemplateColumns:"1.5fr .5fr 1fr .4fr .4fr .5fr 2fr 1fr 1.2fr 1fr",minWidth:1400}}><div>Blueprint</div><div>Type</div><div>Owner</div><div>ME</div><div>TE</div><div>Runs</div><div>Location</div><div>System</div><div>Source</div><div>Status</div></div>
   {shown.map(b=><div className="inv-row" key={`${b.source}-${b.ownerName}-${b.blueprintId}`} style={{gridTemplateColumns:"1.5fr .5fr 1fr .4fr .4fr .5fr 2fr 1fr 1.2fr 1fr",minWidth:1400}}><div className="inv-name">{b.typeName}</div><div>{b.isCopy?"BPC":"BPO"}</div><div>{b.ownerName}</div><div>{b.meLevel}</div><div>{b.teLevel}</div><div>{b.runsRemaining??"∞"}</div><LocationDisplay location={b.resolvedLocation}/><div>{b.resolvedLocation?.solarSystemName??"—"}</div><div>{b.source}</div><div>{b.status}</div></div>)}</div>}
  </Panel>
  {!error&&<Panel title="Missing Blueprint Report" keel={missing.length?"alert":"nominal"} className="dash-hero">{missing.length===0?<p className="ph-mission">Every required blueprint type is owned somewhere.</p>:<div className="conflict-list">{missing.map(m=><div className="conflict-row" key={m.typeName}><div className="conflict-type">not owned</div><div className="conflict-desc">{m.typeName} — required by {m.requiredByOperations.join(", ")}</div></div>)}</div>}</Panel>}
 </div>
}

import { useEffect, useState } from "react";
import {
  getAppSetting, getLatestImport, importOfficialSde, inspectSdeDirectory, openExternalUrl, openSelectedFolder,
  pickSdeArchive, pickSdeDirectory, setAppSetting, type ImportSummary, type SdeInspection,
} from "../lib/backend";
import { Panel } from "./Panel";

export function StaticDataSetup({ onReadyChange }: { onReadyChange?: (ready: boolean) => void }) {
  const [latest, setLatest] = useState<ImportSummary | null>(null);
  const [path, setPath] = useState("");
  const [inspection, setInspection] = useState<SdeInspection | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    const [summary, remembered] = await Promise.all([getLatestImport(), getAppSetting("sde_directory")]);
    setLatest(summary);
    const candidate = remembered || summary?.sourcePath || "";
    setPath(candidate);
    const inspected = candidate ? await inspectSdeDirectory(candidate) : null;
    if (inspected) setInspection(inspected);
    const selectedPath = candidate.replaceAll("/", "\\").toLowerCase();
    const importedPath = (summary?.sourcePath ?? "").replaceAll("/", "\\").toLowerCase();
    onReadyChange?.(Boolean(inspected?.valid && importedPath === selectedPath && (summary?.status === "success" || summary?.status === "partial")));
  }

  useEffect(() => { load().catch((cause) => setError(String(cause))); }, []);

  async function browse() {
    setError(null);
    try {
      const selected = await pickSdeDirectory();
      if (!selected) return;
      setPath(selected);
      await setAppSetting("sde_directory", selected);
      setInspection(await inspectSdeDirectory(selected));
    } catch (cause) { setError(String(cause)); }
  }
  async function checkArchive() {
    const selected = await pickSdeArchive(); if (!selected) return;
    setPath(selected); setInspection(await inspectSdeDirectory(selected)); onReadyChange?.(false);
  }

  async function runImport() {
    if (!inspection?.valid) return;
    setWorking(true); setError(null);
    try {
      const summary = await importOfficialSde(path);
      setLatest(summary);
      onReadyChange?.(summary.status === "success" || summary.status === "partial");
    } catch (cause) { setError(String(cause)); onReadyChange?.(false); }
    finally { setWorking(false); }
  }

  return <Panel title="CCP Static Data" keel={latest ? "nominal" : inspection?.valid ? "coolant" : "furnace"} className="dash-hero">
    <p className="ph-mission">RenderNorth Industrial uses CCP’s official JSONL static-data export for names, blueprints, recipes, volumes, market groups, and universe reference data.</p>
    <div className="new-op-actions setup-wrap"><button className="target-select enabled" onClick={() => openExternalUrl("https://developers.eveonline.com/static-data")}>Download Official CCP Static Data</button><span className="data-source">External official CCP/EVE website</span></div>
    <ol><li>Download the JSONL archive.</li><li>Extract the ZIP to a permanent folder.</li><li>Browse to the extracted folder, not the ZIP.</li></ol>
    <div className="sd-import-row">
      <input className="sd-path-input" readOnly value={path} placeholder="No CCP SDE directory selected" aria-label="CCP static data directory" />
      <button className="target-select enabled" onClick={browse} disabled={working}>Browse…</button>
      <button className="target-select" onClick={checkArchive} disabled={working}>Check Downloaded ZIP</button>
      <button className="target-select enabled" onClick={runImport} disabled={working || !inspection?.valid}>{working ? "Importing…" : latest ? "Re-import" : "Import"}</button>
    </div>
    {inspection && <div className={`setup-validation ${inspection.valid ? "nominal" : "alert"}`}>
      <strong>{inspection.valid ? "Valid CCP SDE directory" : "SDE validation failed"}</strong>
      <span>Version/build: {inspection.sourceBuild ?? latest?.sourceBuild ?? "Not reported"}</span>
      <span>Selected path: {inspection.path}</span>
      {inspection.files.map(file => <span key={file.name}>{file.present ? "Found" : "Missing"}: {file.name}</span>)}
      {inspection.error && <span>{inspection.error}</span>}
    </div>}
    <button className="target-select" disabled={!inspection?.valid} onClick={() => openSelectedFolder(path)}>Open Selected Folder</button>
    <p className="data-source">Do not place the CCP static data inside the RenderNorth Industrial installation folder. You may keep it anywhere permanent on your computer. RenderNorth remembers the selected location.</p>
    {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
    {latest ? <div className="sd-summary-grid">
      <div className="res-summary-cell"><div className="res-summary-label">Status</div><div className={`res-summary-value ${latest.status === "failed" ? "alert" : "nominal"}`}>{latest.status}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">SDE version</div><div className="ops-field-value">{latest.sourceBuild ?? inspection?.sourceBuild ?? "Not reported"}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Types</div><div className="res-summary-value">{latest.typeCount.toLocaleString()}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Blueprints</div><div className="res-summary-value">{latest.blueprintCount.toLocaleString()}</div></div>
      <div className="res-summary-cell"><div className="res-summary-label">Imported</div><div className="ops-field-value">{new Date(latest.importedAt).toLocaleString()}</div></div>
      <div className="res-summary-cell setup-wide"><div className="res-summary-label">Source</div><div className="ops-field-value setup-path">{latest.sourcePath}</div></div>
    </div> : <div className="data-source">Static data is not configured. Inventory type names, production planning, and doctrine analysis remain unavailable.</div>}
  </Panel>;
}

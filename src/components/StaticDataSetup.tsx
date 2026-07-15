import { useEffect, useState } from "react";
import {
  getAppSetting, getLatestImport, importOfficialSde, inspectSdeDirectory,
  pickSdeDirectory, setAppSetting, type ImportSummary, type SdeInspection,
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
    if (candidate) setInspection(await inspectSdeDirectory(candidate));
    onReadyChange?.(summary?.status === "success" || summary?.status === "partial");
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
    <p className="ph-mission">Select the already-extracted official CCP JSONL SDE folder. RenderNorth validates the required files before importing and remembers the selected location.</p>
    <div className="sd-import-row">
      <input className="sd-path-input" readOnly value={path} placeholder="No CCP SDE directory selected" aria-label="CCP static data directory" />
      <button className="target-select enabled" onClick={browse} disabled={working}>Browse…</button>
      <button className="target-select enabled" onClick={runImport} disabled={working || !inspection?.valid}>{working ? "Importing…" : latest ? "Re-import" : "Import"}</button>
    </div>
    {inspection && <div className={`setup-validation ${inspection.valid ? "nominal" : "alert"}`}>
      <strong>{inspection.valid ? "Valid CCP SDE directory" : "SDE validation failed"}</strong>
      <span>Version/build: {inspection.sourceBuild ?? latest?.sourceBuild ?? "Not reported"}</span>
      <span>{inspection.files.map(file => `${file.present ? "✓" : "✕"} ${file.name}`).join(" · ")}</span>
      {inspection.error && <span>{inspection.error}</span>}
    </div>}
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

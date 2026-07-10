import { useEffect, useState } from "react";
import { importStaticData, importOfficialSde, getLatestImport, type ImportSummary } from "../lib/backend";
import { useShowDemoData } from "../lib/demoDataPreference";
import { Panel } from "../components/Panel";

type ImportMode = "sample" | "official";

export function SettingsPage() {
  const [showDemoData, setShowDemoData] = useShowDemoData();
  const [latest, setLatest] = useState<ImportSummary | null>(null);
  const [loadingLatest, setLoadingLatest] = useState(true);
  const [mode, setMode] = useState<ImportMode>("official");
  const [dirPath, setDirPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    getLatestImport()
      .then((s) => live && setLatest(s))
      .finally(() => live && setLoadingLatest(false));
    return () => {
      live = false;
    };
  }, []);

  async function handleImport() {
    if (!dirPath.trim()) return;
    setImporting(true);
    setImportError(null);
    try {
      const summary = mode === "sample" ? await importStaticData(dirPath.trim()) : await importOfficialSde(dirPath.trim());
      setLatest(summary);
    } catch (err) {
      setImportError(String(err));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="dash">
      <Panel title="Static Data" keel={latest ? (latest.status === "success" ? "nominal" : "alert") : "coolant"} className="dash-hero">
        <p className="ph-mission">
          The Build Target Selector, real requirement calculation, and manual inventory all depend on static EVE
          data — type names, groups, categories, and blueprint manufacturing data. No network access is used either
          way; both paths read a local directory you point at.
        </p>

        <div className="new-op-mode-toggle" style={{ marginBottom: 14 }}>
          <button className={mode === "official" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("official")}>
            Import Official CCP SDE
          </button>
          <button className={mode === "sample" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("sample")}>
            Import Sample Fixture
          </button>
        </div>

        {mode === "official" ? (
          <div className="sd-mode-detail">
            <p className="ph-mission">
              Reads an already-<strong>extracted</strong> official CCP JSON Lines SDE directory (this sprint does not
              open the ZIP directly — extract it first) containing <code>categories.jsonl</code>,{" "}
              <code>groups.jsonl</code>, <code>types.jsonl</code>, and <code>blueprints.jsonl</code>. This path has
              not been verified against a real CCP export in the environment this was built in — field-name matching
              is tolerant and best-effort. If a record type doesn't match, the import will report exactly which file
              and line, not fail silently. See <code>docs/REAL_PRODUCTION_PLANNER.md</code> for full details.
            </p>
          </div>
        ) : (
          <div className="sd-mode-detail">
            <p className="ph-mission">
              Reads the small, hand-verified CSV-derivative fixture bundle (<code>invCategories.csv</code>,{" "}
              <code>invGroups.csv</code>, <code>invTypes.csv</code>, <code>industryActivityProducts.csv</code>,{" "}
              <code>industryActivityMaterials.csv</code>) shipped at <code>fixtures/sample-sde-csv/</code>. This is
              the only import path verified end-to-end against real data — useful for testing the workflow before
              trusting a real import.
            </p>
          </div>
        )}

        <div className="sd-import-row">
          <input
            className="sd-path-input"
            type="text"
            placeholder={mode === "official" ? "e.g. C:\\EVE\\sde-jsonl or /home/user/eve-sde-jsonl" : "e.g. /path/to/fixtures/sample-sde-csv"}
            value={dirPath}
            onChange={(e) => setDirPath(e.target.value)}
          />
          <button className="target-select enabled" onClick={handleImport} disabled={importing || !dirPath.trim()}>
            {importing ? "Importing…" : latest ? "Re-import" : "Start Import"}
          </button>
        </div>

        {importError && (
          <div className="sd-error">
            <div className="conflict-type">import failed</div>
            <div className="conflict-desc">{importError}</div>
          </div>
        )}

        {loadingLatest ? (
          <div className="data-source">Checking for a previous import…</div>
        ) : latest ? (
          <div className="sd-summary-grid">
            <div className="res-summary-cell">
              <div className="res-summary-label">Status</div>
              <div className={`res-summary-value ${latest.status === "success" ? "nominal" : "alert"}`}>{latest.status}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Format</div>
              <div className="ops-field-value" style={{ fontSize: 13 }}>
                {latest.format === "jsonl_official" ? "Official CCP SDE (JSONL)" : "Sample Fixture (CSV)"}
              </div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Types</div>
              <div className="res-summary-value">{latest.typeCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Groups</div>
              <div className="res-summary-value">{latest.groupCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Categories</div>
              <div className="res-summary-value">{latest.categoryCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Blueprints</div>
              <div className="res-summary-value">{latest.blueprintCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Material Lines</div>
              <div className="res-summary-value">{latest.materialCount}</div>
            </div>
            <div className="res-summary-cell" style={{ gridColumn: "span 2" }}>
              <div className="res-summary-label">Source</div>
              <div className="ops-field-value" style={{ fontSize: 13 }}>{latest.sourcePath}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Imported</div>
              <div className="ops-field-value" style={{ fontSize: 12 }}>{latest.importedAt}</div>
            </div>
            {latest.errorSummary && (
              <div className="res-summary-cell" style={{ gridColumn: "1 / -1" }}>
                <div className="res-summary-label">Skipped / malformed records</div>
                <pre className="sd-error-detail">{latest.errorSummary}</pre>
              </div>
            )}
          </div>
        ) : (
          <p className="ph-mission">No static data has been imported yet. Build Target Selector and real requirement calculation are unavailable until an import completes.</p>
        )}
      </Panel>

      <Panel title="Demo Data" keel="coolant">
        <p className="ph-mission">
          RenderNorth Industrial ships with seeded demo operations, blueprints, and inventory (Sprints 001–007) so the
          app is usable before you import anything or create a real build. Demo operations are labeled{" "}
          <span className="demo-badge">DEMO</span> throughout the app and are kept separate from anything real you
          create. Demo data is never deleted by this toggle — it only controls whether demo operations are shown in
          the Operations list and Mission Control.
        </p>
        <div className="demo-toggle-row">
          <span className="ops-field-label">Show Demo Data</span>
          <button className={showDemoData ? "target-select enabled active" : "target-select enabled"} onClick={() => setShowDemoData(!showDemoData)}>
            {showDemoData ? "ON — showing demo operations" : "OFF — demo operations hidden"}
          </button>
        </div>
      </Panel>

      <Panel title="Other Settings" keel="coolant">
        <p className="ph-mission">
          Character management (official CCP ESI OAuth), scope review, and notification preferences remain planned —
          this app performs no ESI synchronization and no gameplay automation.
        </p>
      </Panel>
    </div>
  );
}

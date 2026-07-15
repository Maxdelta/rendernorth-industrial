import { useEffect, useState } from "react";
import { importStaticData, importOfficialSde, getLatestImport, type ImportSummary } from "../lib/backend";
import { Panel } from "../components/Panel";
import { CharactersPanel } from "../components/CharactersPanel";
import { SHOW_DEVELOPMENT_UI } from "../lib/runtimeMode";

type ImportMode = "sample" | "official";

export function SettingsPage() {
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
          way; the importer reads a local directory you point at.
        </p>

        <div className="new-op-mode-toggle" style={{ marginBottom: 14 }}>
          <button className={mode === "official" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("official")}>
            Import Official CCP SDE
          </button>
          {SHOW_DEVELOPMENT_UI && <button className={mode === "sample" ? "target-select enabled active" : "target-select enabled"} onClick={() => setMode("sample")}>
            Import Sample Fixture
          </button>}
        </div>

        {mode === "official" ? (
          <div className="sd-mode-detail">
            <p className="ph-mission">
              Reads an already-<strong>extracted</strong> official CCP JSON Lines SDE directory. ZIP archives are not
              opened directly, so extract the archive first. The directory must contain <code>categories.jsonl</code>,{" "}
              <code>groups.jsonl</code>, <code>types.jsonl</code>, and <code>blueprints.jsonl</code>. Field-name matching
              is tolerant; an incompatible record reports its source file and line instead of failing silently.
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

      <CharactersPanel />

      {SHOW_DEVELOPMENT_UI && <Panel title="Other Settings" keel="coolant">
        <p className="ph-mission">
          Notification preferences remain planned. Blueprint jobs and industry job tracking are separate future
          scopes, not covered by the character connection above.
        </p>
      </Panel>}
    </div>
  );
}

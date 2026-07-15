import { useEffect, useMemo, useState } from "react";
import {
  addCharacter, getAppSetting, getLatestImport, getMarketProfile, importOfficialSde,
  inspectSdeDirectory, listCharacters, pickSdeDirectory, refreshAssetLocations,
  refreshMarketPrices, setAppSetting, syncAllCharacterAssets, syncAllCharacterBlueprints,
  type CharacterSummary, type ImportSummary, type MarketProfile, type SdeInspection,
} from "../lib/backend";

const STEPS = ["Welcome", "Client ID", "Static Data", "Characters", "Assets & Locations", "Blueprints", "Market", "Done"];

export function FirstRunWizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const [clientId, setClientId] = useState("");
  const [latest, setLatest] = useState<ImportSummary | null>(null);
  const [sdePath, setSdePath] = useState("");
  const [inspection, setInspection] = useState<SdeInspection | null>(null);
  const [characters, setCharacters] = useState<CharacterSummary[]>([]);
  const [market, setMarket] = useState<MarketProfile | null>(null);
  const [locationsReady, setLocationsReady] = useState(false);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  async function reload() {
    const [savedClient, importSummary, connected, profile, locationDone] = await Promise.all([
      getAppSetting("esi_client_id"), getLatestImport(), listCharacters(), getMarketProfile().catch(() => null), getAppSetting("onboarding_locations_refreshed"),
    ]);
    setClientId(savedClient ?? ""); setLatest(importSummary); setCharacters(connected); setMarket(profile); setLocationsReady(locationDone === "true");
    const remembered = await getAppSetting("sde_directory");
    const path = remembered || importSummary?.sourcePath || ""; setSdePath(path);
    if (path) setInspection(await inspectSdeDirectory(path));
  }
  useEffect(() => { reload().catch((cause) => setError(String(cause))); }, []);

  const enabled = characters.filter(character => character.enabled);
  const clientReady = /^[A-Za-z0-9_-]{16,}$/.test(clientId.trim());
  const sdeReady = latest?.status === "success" || latest?.status === "partial";
  const charactersReady = enabled.length > 0;
  const assetsReady = charactersReady && enabled.every(character => character.syncStatus === "success" && Boolean(character.lastSyncAt));
  const blueprintsReady = charactersReady && enabled.every(character => character.blueprintSyncStatus === "success" && Boolean(character.blueprintLastSyncAt));
  const marketReady = Boolean(market?.fetchedAt);
  const complete = clientReady && sdeReady && charactersReady && assetsReady && locationsReady && blueprintsReady && marketReady;
  const readiness = [true, clientReady, sdeReady, charactersReady, assetsReady && locationsReady, blueprintsReady, marketReady, complete];

  async function run(action: () => Promise<void>) {
    setWorking(true); setError(null); setFeedback(null);
    try { await action(); await reload(); } catch (cause) { setError(String(cause)); }
    finally { setWorking(false); }
  }

  async function saveClient() {
    if (!clientReady) { setError("Enter the Client ID from your EVE Developer application. It is not an EVE username, password, or client secret."); return; }
    await run(async () => { await setAppSetting("esi_client_id", clientId.trim()); setFeedback("Client ID saved locally"); });
  }
  async function browse() {
    await run(async () => {
      const selected = await pickSdeDirectory(); if (!selected) return;
      setSdePath(selected); await setAppSetting("sde_directory", selected); setInspection(await inspectSdeDirectory(selected));
    });
  }
  async function importSde() { await run(async () => { const result = await importOfficialSde(sdePath); setLatest(result); setFeedback("CCP static data imported"); }); }
  async function connect() { await run(async () => { await addCharacter(clientId.trim()); setFeedback("Character authorization completed"); }); }
  async function syncAssetsAndLocations() {
    await run(async () => {
      const results = await syncAllCharacterAssets(clientId.trim());
      const failures = results.filter(result => result.error).map(result => result.error);
      if (failures.length) throw new Error(failures.join("; "));
      const locations = await refreshAssetLocations(clientId.trim());
      await setAppSetting("onboarding_locations_refreshed", "true"); setLocationsReady(true);
      setFeedback(`Assets synchronized; ${locations.resolved} locations resolved, ${locations.inaccessible} inaccessible`);
    });
  }
  async function syncBlueprints() {
    await run(async () => {
      const results = await syncAllCharacterBlueprints(clientId.trim());
      const failures = results.filter(result => result.error).map(result => result.error);
      if (failures.length) throw new Error(failures.join("; "));
      setFeedback("Blueprints synchronized");
    });
  }
  async function refreshMarket() { await run(async () => { await refreshMarketPrices(); setFeedback("Market snapshot refreshed"); }); }
  async function finish() {
    if (!complete) return;
    await setAppSetting("onboarding_completed", "true");
    window.location.hash = "#/quartermaster";
    onComplete();
  }

  const permissionWarnings = useMemo(() => enabled.flatMap(character => [
    !character.assetScopeGranted ? `${character.name}: reauthorize to add esi-assets.read_assets.v1 for read-only personal asset locations and quantities.` : null,
    !character.blueprintScopeGranted ? `${character.name}: reauthorize to add esi-characters.read_blueprints.v1 for owned BPO/BPC, ME, TE, and runs.` : null,
    !character.structureScopeGranted ? `${character.name}: reauthorize to add esi-universe.read_structures.v1 for names of accessible player structures.` : null,
  ].filter(Boolean) as string[]), [characters]);

  return <div className="setup-shell">
    <aside className="setup-rail"><div className="setup-brand">RenderNorth <span>Industrial</span></div><div className="setup-kicker">Private Beta Setup</div>{STEPS.map((label, index) => <button key={label} onClick={() => setStep(index)} className={`${index === step ? "active" : ""} ${readiness[index] ? "complete" : ""}`}><span>{readiness[index] ? "✓" : index + 1}</span>{label}</button>)}</aside>
    <main className="setup-main">
      <div className="setup-heading"><div><div className="panel-title">First-run configuration</div><h1>{STEPS[step]}</h1></div><div className="setup-progress">Step {step + 1} of {STEPS.length}</div></div>
      {error && <div className="sd-error"><div className="conflict-type">Setup action failed</div><div className="conflict-desc">{error}</div></div>}
      {feedback && <div className="setup-feedback">{feedback}</div>}

      {step === 0 && <section className="setup-card setup-welcome"><h2>The industrial operating system for EVE Online</h2><p>RenderNorth turns CCP-authorized data into deterministic inventory, production, market, procurement, and doctrine intelligence. It never automates gameplay.</p><div className="privacy-grid"><div><strong>Read-only ESI</strong><span>Only permissions shown during EVE SSO are requested.</span></div><div><strong>Local-first</strong><span>Your industrial database remains on this computer.</span></div><div><strong>No credentials</strong><span>EVE usernames and passwords are entered only on CCP's website.</span></div><div><strong>Secure tokens</strong><span>Refresh tokens remain in Windows Credential Manager and never enter diagnostics.</span></div></div></section>}

      {step === 1 && <section className="setup-card"><h2>Configure official EVE SSO</h2><p>Create a native EVE Developer application with callback <code>http://localhost:17117/callback</code>, then paste its Client ID. Do not enter a client secret.</p><label className="new-op-field"><span className="ops-field-label">CCP Developer Client ID</span><input className="sd-path-input" value={clientId} onChange={event => setClientId(event.target.value)} placeholder="Client ID only" /></label><button className="target-select enabled setup-primary" disabled={!clientReady || working} onClick={saveClient}>Save Client ID</button></section>}

      {step === 2 && <section className="setup-card"><h2>Locate CCP static data</h2><p>Download and extract the official JSONL SDE. Select its folder; RenderNorth validates all four required files before import.</p><div className="sd-import-row"><input className="sd-path-input" readOnly value={sdePath} placeholder="No directory selected" /><button className="target-select enabled" disabled={working} onClick={browse}>Browse…</button><button className="target-select enabled" disabled={working || !inspection?.valid} onClick={importSde}>{latest ? "Re-import" : "Import"}</button></div>{inspection && <div className={`setup-validation ${inspection.valid ? "nominal" : "alert"}`}><strong>{inspection.valid ? "Valid SDE" : "Invalid SDE"}</strong><span>Version/build: {inspection.sourceBuild ?? "Not reported"}</span><span>{inspection.files.map(file => `${file.present ? "✓" : "✕"} ${file.name}`).join(" · ")}</span>{inspection.error && <span>{inspection.error}</span>}</div>}{latest && <div className="setup-feedback">Imported {latest.typeCount.toLocaleString()} types and {latest.blueprintCount.toLocaleString()} blueprints on {new Date(latest.importedAt).toLocaleString()}</div>}</section>}

      {step === 3 && <section className="setup-card"><h2>Connect characters</h2><p>Authorization opens in your system browser on CCP's official login page. RenderNorth never sees your EVE credentials.</p><button className="target-select enabled setup-primary" disabled={working || !clientReady} onClick={connect}>{working ? "Waiting for CCP authorization…" : "Add / Reauthorize Character"}</button><div className="setup-character-list">{characters.length ? characters.map(character => <div key={character.characterId}><strong>{character.name}</strong><span className={character.authorizationStatus === "authorized" ? "nominal" : "alert"}>{character.authorizationStatus}</span><span>{character.enabled ? "Enabled" : "Disabled"}</span></div>) : <p>No characters connected.</p>}</div>{permissionWarnings.map(warning => <div className="setup-warning" key={warning}>{warning}</div>)}</section>}

      {step === 4 && <section className="setup-card"><h2>Refresh assets and locations</h2><p>This reads all asset pages for enabled characters, then resolves NPC stations and accessible player structures. Inaccessible structures remain clearly labeled by raw ID.</p><button className="target-select enabled setup-primary" disabled={working || !charactersReady || permissionWarnings.some(warning => warning.includes("read_assets"))} onClick={syncAssetsAndLocations}>{working ? "Synchronizing…" : "Refresh Assets & Locations"}</button><div className="setup-character-list">{enabled.map(character => <div key={character.characterId}><strong>{character.name}</strong><span>{character.assetCount.toLocaleString()} assets</span><span className={character.syncStatus === "success" ? "nominal" : "furnace"}>{character.syncStatus} · {character.lastSyncAt ? new Date(character.lastSyncAt).toLocaleString() : "never"}</span></div>)}</div><div className={locationsReady ? "setup-feedback" : "setup-warning"}>{locationsReady ? "✓ Location refresh completed" : "Location refresh has not completed for onboarding."}</div></section>}

      {step === 5 && <section className="setup-card"><h2>Refresh blueprints</h2><p>Blueprint synchronization reads owned BPOs and BPCs, including ME, TE, location, and remaining runs.</p><button className="target-select enabled setup-primary" disabled={working || !charactersReady || permissionWarnings.some(warning => warning.includes("read_blueprints"))} onClick={syncBlueprints}>{working ? "Synchronizing…" : "Refresh Blueprints"}</button><div className="setup-character-list">{enabled.map(character => <div key={character.characterId}><strong>{character.name}</strong><span>{character.blueprintCount.toLocaleString()} blueprints</span><span className={character.blueprintSyncStatus === "success" ? "nominal" : "furnace"}>{character.blueprintSyncStatus} · {character.blueprintLastSyncAt ? new Date(character.blueprintLastSyncAt).toLocaleString() : "never"}</span></div>)}</div></section>}

      {step === 6 && <section className="setup-card"><h2>Refresh market</h2><p>RenderNorth stores an explicit local snapshot for the selected market. Pricing always displays its source, timestamp, and stale/current state.</p><button className="target-select enabled setup-primary" disabled={working || !sdeReady} onClick={refreshMarket}>{working ? "Refreshing market…" : "Refresh Market"}</button><div className="about-grid"><div><span>Current market</span><strong>{market?.displayName ?? "Unavailable"}</strong></div><div><span>Last refresh</span><strong>{market?.fetchedAt ? new Date(market.fetchedAt).toLocaleString() : "Never"}</strong></div><div><span>State</span><strong className={market?.stale ? "furnace" : "nominal"}>{market?.stale ? "STALE" : market?.fetchedAt ? "CURRENT" : "NOT READY"}</strong></div><div><span>Orders</span><strong>{market?.orderCount.toLocaleString() ?? 0}</strong></div></div></section>}

      {step === 7 && <section className="setup-card"><h2>{complete ? "RenderNorth is ready" : "Setup is incomplete"}</h2><p>{complete ? "Your local data foundation is ready. Open Quartermaster, import an EFT doctrine, set target quantities, and analyze fleet readiness." : "Complete every required item below. Empty application pages are not treated as successful setup."}</p><div className="setup-checklist">{[["Client ID", clientReady], ["Static data", sdeReady], ["Connected character", charactersReady], ["Assets", assetsReady], ["Locations", locationsReady], ["Blueprints", blueprintsReady], ["Market", marketReady]].map(([label, ready]) => <div key={String(label)} className={ready ? "complete" : "incomplete"}><span>{ready ? "✓" : "!"}</span>{String(label)}</div>)}</div><button className="target-select enabled setup-primary" disabled={!complete} onClick={finish}>Finish & Open Quartermaster</button></section>}

      <div className="setup-nav"><button className="target-select" disabled={step === 0} onClick={() => setStep(value => value - 1)}>Back</button><button className="target-select enabled" disabled={step === STEPS.length - 1 || !readiness[step]} onClick={() => setStep(value => value + 1)}>Continue</button></div>
    </main>
  </div>;
}

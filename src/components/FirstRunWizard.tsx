import { useEffect, useMemo, useState } from "react";
import {
  addCharacter, getAppSetting, getLatestImport, getMarketProfile, importOfficialSde,
  inspectSdeDirectory, listCharacters, openExternalUrl, openSelectedFolder, pickSdeArchive, pickSdeDirectory, refreshAssetLocations,
  refreshMarketPrices, setAppSetting, syncAllCharacterAssets, syncAllCharacterBlueprints,
  type CharacterSummary, type ImportSummary, type MarketProfile, type SdeInspection,
} from "../lib/backend";
import { getAuthenticationConfig, restoreOfficialAuthentication, saveCustomClientId, type AuthenticationConfig } from "../lib/authentication";

const STEPS = ["Welcome", "Connect Character", "Static Data", "Character Status", "Assets & Locations", "Blueprints", "Market", "Done"];
const SDE_DOWNLOAD_URL = "https://developers.eveonline.com/static-data";
const SETUP_GUIDE_URL = "https://github.com/Maxdelta/rendernorth-industrial/blob/main/docs/OPEN_BETA_ONBOARDING.md";

export function FirstRunWizard({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0);
  const [clientId, setClientId] = useState("");
  const [customClientId, setCustomClientId] = useState("");
  const [auth, setAuth] = useState<AuthenticationConfig | null>(null);
  const [advanced, setAdvanced] = useState(false);
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
      getAuthenticationConfig(), getLatestImport(), listCharacters(), getMarketProfile().catch(() => null), getAppSetting("onboarding_locations_refreshed"),
    ]);
    setAuth(savedClient); setClientId(savedClient.clientId); setCustomClientId(savedClient.mode === "custom" ? savedClient.clientId : ""); setLatest(importSummary); setCharacters(connected); setMarket(profile); setLocationsReady(locationDone === "true");
    const remembered = await getAppSetting("sde_directory");
    const path = remembered || importSummary?.sourcePath || ""; setSdePath(path);
    if (path) setInspection(await inspectSdeDirectory(path));
  }
  useEffect(() => { reload().catch((cause) => setError(String(cause))); }, []);

  const enabled = characters.filter(character => character.enabled);
  const clientReady = Boolean(auth?.clientId);
  const normalizedSdePath = sdePath.replaceAll("/", "\\").toLowerCase();
  const importedSdePath = (latest?.sourcePath ?? "").replaceAll("/", "\\").toLowerCase();
  const sdeReady = Boolean(inspection?.valid && normalizedSdePath === importedSdePath && (latest?.status === "success" || latest?.status === "partial"));
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

  async function saveClient() { await run(async () => { const config = await saveCustomClientId(customClientId); setAuth(config); setClientId(config.clientId); setFeedback("Custom CCP application saved locally"); }); }
  async function restoreOfficial() { await run(async () => { const config = await restoreOfficialAuthentication(); setAuth(config); setClientId(config.clientId); setCustomClientId(""); setFeedback("Official RenderNorth authentication restored"); }); }
  async function browse() {
    await run(async () => {
      const selected = await pickSdeDirectory(); if (!selected) return;
      setSdePath(selected); await setAppSetting("sde_directory", selected); setInspection(await inspectSdeDirectory(selected));
    });
  }
  async function chooseArchive() {
    setWorking(true); setError(null); setFeedback(null);
    try { const selected = await pickSdeArchive(); if (!selected) return; setSdePath(selected); setInspection(await inspectSdeDirectory(selected)); }
    catch (cause) { setError(String(cause)); }
    finally { setWorking(false); }
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
    <aside className="setup-rail"><div className="setup-brand">RenderNorth <span>Industrial</span></div><div className="setup-kicker">Open Beta 0.1.6 Setup</div>{STEPS.map((label, index) => <button key={label} onClick={() => setStep(index)} className={`${index === step ? "active" : ""} ${readiness[index] ? "complete" : ""}`}><span>{readiness[index] ? "✓" : index + 1}</span>{label}</button>)}</aside>
    <main className="setup-main">
      <div className="setup-heading"><div><div className="panel-title">First-run configuration</div><h1>{STEPS[step]}</h1></div><div className="setup-progress">Step {step + 1} of {STEPS.length}</div></div>
      {error && <div className="sd-error"><div className="conflict-type">Setup action failed</div><div className="conflict-desc">{error}</div></div>}
      {feedback && <div className="setup-feedback">{feedback}</div>}

      {step === 0 && <section className="setup-card setup-welcome"><h2>The industrial operating system for EVE Online</h2><p>RenderNorth turns CCP-authorized data into deterministic inventory, production, market, procurement, and doctrine intelligence. It never automates gameplay.</p><div className="privacy-grid"><div><strong>Read-only ESI</strong><span>Only permissions shown during EVE SSO are requested.</span></div><div><strong>Local-first</strong><span>Your industrial database remains on this computer.</span></div><div><strong>No credentials</strong><span>EVE usernames and passwords are entered only on CCP's website.</span></div><div><strong>Secure tokens</strong><span>Refresh tokens remain in Windows Credential Manager and never enter diagnostics.</span></div></div></section>}
      {step === 0 && <section className="setup-card"><h2>Open Beta 0.1.6 help</h2><p>Report beta issues or contact Maxdelta on Discord: <code>maxdelta0089</code>. Community support is not official CCP support.</p><div className="new-op-actions setup-wrap"><button className="target-select enabled" onClick={() => openExternalUrl("https://github.com/Maxdelta/rendernorth-industrial/issues")}>Report a Bug</button><button className="target-select enabled" onClick={() => openExternalUrl(SETUP_GUIDE_URL)}>Open Setup Guide</button><button className="target-select enabled" onClick={() => openExternalUrl("https://discord.gg/XycCz6ppx")}>Join Discord</button><button className="target-select enabled" onClick={() => openExternalUrl("https://buymeacoffee.com/maxdelta")}>Buy Maxdelta a Coffee</button></div><p className="data-source">Optional support is always appreciated, never required. In EVE: Character Maxdelta — ISK or PLEX.</p></section>}

      {step === 1 && <section className="setup-card"><h2>Connect an EVE character</h2><p>RenderNorth Industrial uses the official RenderNorth Industrial CCP application for secure read-only EVE authentication.</p><div className="setup-feedback">Current Application: <strong>{auth?.applicationName ?? "Loading…"}</strong></div><button className="target-select enabled setup-primary" disabled={!clientReady || working} onClick={connect}>{working ? "Waiting for official CCP login…" : "Connect Character"}</button><button className="target-select" onClick={() => setAdvanced(value => !value)}>Advanced: Use my own CCP Developer Application</button>{advanced && <div className="setup-advanced"><p>Optional advanced override. Enter only a public Client ID. RenderNorth never asks for a Client Secret.</p><label className="new-op-field"><span className="ops-field-label">Custom CCP Client ID</span><input className="sd-path-input" value={customClientId} onChange={event => setCustomClientId(event.target.value)} placeholder="Client ID only — never a Client Secret" /></label><div className="new-op-actions setup-wrap"><button className="target-select enabled" disabled={working} onClick={saveClient}>Use Custom Application</button><button className="target-select enabled" disabled={working || auth?.mode === "official"} onClick={restoreOfficial}>Restore Official RenderNorth Client ID</button></div></div>}</section>}

      {step === 2 && <section className="setup-card"><h2>CCP Static Data Required</h2><p>RenderNorth Industrial uses CCP’s official static-data export for item names, blueprints, production recipes, item volumes, market groups, and universe reference data.</p><button className="target-select enabled setup-primary" onClick={() => openExternalUrl(SDE_DOWNLOAD_URL)}>Download Official CCP Static Data</button><p className="data-source">Opens an external official CCP/EVE website. RenderNorth does not automatically download, extract, or execute files.</p><ol><li>Download the supported JSONL archive from CCP.</li><li>Extract the downloaded ZIP to a permanent folder.</li><li>Return to RenderNorth Industrial.</li><li>Click Browse.</li><li>Select the extracted folder, not the ZIP file.</li></ol><p>Required: <code>categories.jsonl</code>, <code>groups.jsonl</code>, <code>types.jsonl</code>, <code>blueprints.jsonl</code>.</p><div className="sd-import-row"><input className="sd-path-input" readOnly value={sdePath} placeholder="No directory selected" /><button className="target-select enabled" disabled={working} onClick={browse}>Browse…</button><button className="target-select" disabled={working} onClick={chooseArchive}>Check Downloaded ZIP</button><button className="target-select enabled" disabled={working || !inspection?.valid} onClick={importSde}>{latest ? "Re-import" : "Import"}</button></div><button className="target-select" disabled={!inspection?.valid} onClick={() => openSelectedFolder(sdePath)}>Open Selected Folder</button>{inspection && <div className={`setup-validation ${inspection.valid ? "nominal" : "alert"}`}><strong>{inspection.valid ? "Valid SDE" : "SDE validation failed"}</strong><span>Selected path: {inspection.path}</span><span>Version/build: {inspection.sourceBuild ?? "Not reported"}</span>{inspection.files.map(file => <span key={file.name}>{file.present ? "Found" : "Missing"}: {file.name}</span>)}{inspection.error && <span>{inspection.error}</span>}</div>}<p className="data-source">Do not place the CCP static data inside the RenderNorth Industrial installation folder. You may keep it anywhere permanent on your computer. RenderNorth remembers the selected location.</p>{latest && <div className="setup-feedback">Last successful import: {new Date(latest.importedAt).toLocaleString()} · build {latest.sourceBuild ?? "not reported"}</div>}</section>}

      {step === 3 && <section className="setup-card"><h2>Character status</h2><p>Authorization opens in your system browser on CCP's official login page. RenderNorth never sees your EVE credentials.</p><button className="target-select enabled setup-primary" disabled={working || !clientReady} onClick={connect}>{working ? "Waiting for CCP authorization…" : "Add / Reauthorize Character"}</button><div className="setup-character-list">{characters.length ? characters.map(character => <div key={character.characterId}><strong>{character.name}</strong><span className={character.authorizationStatus === "authorized" ? "nominal" : "alert"}>{character.authorizationStatus}</span><span>{character.enabled ? "Enabled" : "Disabled"}</span></div>) : <p>No characters connected.</p>}</div>{permissionWarnings.map(warning => <div className="setup-warning" key={warning}>{warning}</div>)}</section>}

      {step === 4 && <section className="setup-card"><h2>Refresh assets and locations</h2><p>This reads all asset pages for enabled characters, then resolves NPC stations and accessible player structures. Inaccessible structures remain clearly labeled by raw ID.</p><button className="target-select enabled setup-primary" disabled={working || !charactersReady || permissionWarnings.some(warning => warning.includes("read_assets"))} onClick={syncAssetsAndLocations}>{working ? "Synchronizing…" : "Refresh Assets & Locations"}</button><div className="setup-character-list">{enabled.map(character => <div key={character.characterId}><strong>{character.name}</strong><span>{character.assetCount.toLocaleString()} assets</span><span className={character.syncStatus === "success" ? "nominal" : "furnace"}>{character.syncStatus} · {character.lastSyncAt ? new Date(character.lastSyncAt).toLocaleString() : "never"}</span></div>)}</div><div className={locationsReady ? "setup-feedback" : "setup-warning"}>{locationsReady ? "✓ Location refresh completed" : "Location refresh has not completed for onboarding."}</div></section>}

      {step === 5 && <section className="setup-card"><h2>Refresh blueprints</h2><p>Blueprint synchronization reads owned BPOs and BPCs, including ME, TE, location, and remaining runs.</p><button className="target-select enabled setup-primary" disabled={working || !charactersReady || permissionWarnings.some(warning => warning.includes("read_blueprints"))} onClick={syncBlueprints}>{working ? "Synchronizing…" : "Refresh Blueprints"}</button><div className="setup-character-list">{enabled.map(character => <div key={character.characterId}><strong>{character.name}</strong><span>{character.blueprintCount.toLocaleString()} blueprints</span><span className={character.blueprintSyncStatus === "success" ? "nominal" : "furnace"}>{character.blueprintSyncStatus} · {character.blueprintLastSyncAt ? new Date(character.blueprintLastSyncAt).toLocaleString() : "never"}</span></div>)}</div></section>}

      {step === 6 && <section className="setup-card"><h2>Refresh market</h2><p>RenderNorth stores an explicit local snapshot for the selected market. Pricing always displays its source, timestamp, and stale/current state.</p><button className="target-select enabled setup-primary" disabled={working || !sdeReady} onClick={refreshMarket}>{working ? "Refreshing market…" : "Refresh Market"}</button><div className="about-grid"><div><span>Current market</span><strong>{market?.displayName ?? "Unavailable"}</strong></div><div><span>Last refresh</span><strong>{market?.fetchedAt ? new Date(market.fetchedAt).toLocaleString() : "Never"}</strong></div><div><span>State</span><strong className={market?.stale ? "furnace" : "nominal"}>{market?.stale ? "STALE" : market?.fetchedAt ? "CURRENT" : "NOT READY"}</strong></div><div><span>Orders</span><strong>{market?.orderCount.toLocaleString() ?? 0}</strong></div></div></section>}

      {step === 7 && <section className="setup-card"><h2>{complete ? "RenderNorth is ready" : "Setup is incomplete"}</h2><p>{complete ? "Your local data foundation is ready. Open Quartermaster, import an EFT doctrine, set target quantities, and analyze fleet readiness." : "Complete every required item below. Empty application pages are not treated as successful setup."}</p><div className="setup-checklist">{[["Authentication", clientReady], ["Static data", sdeReady], ["Connected character", charactersReady], ["Assets", assetsReady], ["Locations", locationsReady], ["Blueprints", blueprintsReady], ["Market", marketReady]].map(([label, ready]) => <div key={String(label)} className={ready ? "complete" : "incomplete"}><span>{ready ? "✓" : "!"}</span>{String(label)}</div>)}</div><button className="target-select enabled setup-primary" disabled={!complete} onClick={finish}>Finish & Open Quartermaster</button></section>}

      <div className="setup-nav"><button className="target-select" disabled={step === 0} onClick={() => setStep(value => value - 1)}>Back</button><button className="target-select enabled" disabled={step === STEPS.length - 1 || !readiness[step]} onClick={() => setStep(value => value + 1)}>Continue</button></div>
    </main>
  </div>;
}

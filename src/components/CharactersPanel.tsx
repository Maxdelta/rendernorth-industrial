import { useEffect, useState } from "react";
import {
  addCharacter, listCharacters, refreshAssetLocations, removeCharacter,
  setCharacterEnabled, syncAllCharacterAssets, syncAllCharacterBlueprints,
  syncCharacterAssets, syncCharacterBlueprints, type CharacterSummary,
} from "../lib/backend";
import { getAuthenticationConfig, restoreOfficialAuthentication, saveCustomClientId, type AuthenticationConfig } from "../lib/authentication";
import { Panel } from "./Panel";

function statusTone(character: CharacterSummary): "nominal" | "furnace" | "alert" {
  if (character.authorizationStatus === "revoked" || character.syncStatus === "error") return "alert";
  if (!character.assetScopeGranted || !character.blueprintScopeGranted || !character.structureScopeGranted || character.authorizationStatus === "expired") return "furnace";
  return "nominal";
}

export function CharactersPanel() {
  const [clientId, setClientId] = useState("");
  const [customClientId, setCustomClientId] = useState("");
  const [auth, setAuth] = useState<AuthenticationConfig | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [characters, setCharacters] = useState<CharacterSummary[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<number | null>(null);
  const [syncing, setSyncing] = useState<number | "all" | null>(null);
  const [blueprintSyncing, setBlueprintSyncing] = useState<number | "all" | null>(null);
  const [locationSyncing, setLocationSyncing] = useState(false);
  const [locationFeedback, setLocationFeedback] = useState<string | null>(null);

  function refresh() { listCharacters().then(setCharacters).catch(cause => setError(String(cause))); }
  useEffect(refresh, []);
  useEffect(() => { getAuthenticationConfig().then(config => { setAuth(config); setClientId(config.clientId); setCustomClientId(config.mode === "custom" ? config.clientId : ""); }); }, []);
  async function saveCustom() { try { const config = await saveCustomClientId(customClientId); setAuth(config); setClientId(config.clientId); } catch (cause) { setError(String(cause)); } }
  async function restoreOfficial() { const config = await restoreOfficialAuthentication(); setAuth(config); setClientId(config.clientId); setCustomClientId(""); }
  async function handleAddCharacter() {
    if (!clientId.trim()) { setError("Authentication configuration is unavailable."); return; }
    setAdding(true); setError(null);
    try { await addCharacter(clientId.trim()); refresh(); } catch (cause) { setError(String(cause)); }
    finally { setAdding(false); }
  }
  async function handleSyncOne(characterId: number) {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's Client ID first."); return; }
    setSyncing(characterId); setError(null);
    try { const result = await syncCharacterAssets(clientId.trim(), characterId); if (result.error) setError(result.error); }
    catch (cause) { setError(String(cause)); } finally { setSyncing(null); refresh(); }
  }
  async function handleSyncAll() {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's Client ID first."); return; }
    setSyncing("all"); setError(null);
    try { const results = await syncAllCharacterAssets(clientId.trim()); const failures = results.filter(result => result.error).map(result => result.error).join("; "); if (failures) setError(failures); }
    catch (cause) { setError(String(cause)); } finally { setSyncing(null); refresh(); }
  }
  async function handleBlueprintSyncOne(characterId: number) {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's Client ID first."); return; }
    setBlueprintSyncing(characterId); setError(null);
    try { const result = await syncCharacterBlueprints(clientId.trim(), characterId); if (result.error) setError(result.error); }
    catch (cause) { setError(String(cause)); } finally { setBlueprintSyncing(null); refresh(); }
  }
  async function handleBlueprintSyncAll() {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's Client ID first."); return; }
    setBlueprintSyncing("all"); setError(null);
    try { const results = await syncAllCharacterBlueprints(clientId.trim()); const failures = results.filter(result => result.error).map(result => result.error).join("; "); if (failures) setError(failures); }
    catch (cause) { setError(String(cause)); } finally { setBlueprintSyncing(null); refresh(); }
  }
  async function handleLocations() {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's Client ID first."); return; }
    setLocationSyncing(true); setError(null); setLocationFeedback(null);
    try { const result = await refreshAssetLocations(clientId.trim()); setLocationFeedback(`${result.resolved} resolved · ${result.inaccessible} inaccessible · ${result.errors.length} errors`); }
    catch (cause) { setError(String(cause)); } finally { setLocationSyncing(false); }
  }
  async function handleToggleEnabled(character: CharacterSummary) { await setCharacterEnabled(character.characterId, !character.enabled); refresh(); }
  async function handleRemove(characterId: number) { await removeCharacter(characterId); setConfirmRemove(null); refresh(); }

  return <Panel title="Characters & ESI Sync" keel="coolant" className="dash-hero">
    <p className="ph-mission">Connect characters through official EVE SSO and synchronize read-only personal assets and blueprints. Tokens remain in Windows Credential Manager and are never exposed to the frontend.</p>
    <div className="setup-feedback">Current Application: <strong>{auth?.applicationName ?? "Loading…"}</strong>{auth?.mode === "official" && <> · <code>{auth.clientId}</code></>}</div>
    <button className="target-select" onClick={() => setAdvanced(value => !value)}>Advanced Authentication</button>
    {advanced && <div className="setup-advanced"><p>Use a custom CCP Developer Application only if you intentionally manage one. Enter a public Client ID; never enter a Client Secret.</p><label className="new-op-field"><span className="ops-field-label">Custom CCP Client ID</span><input className="sd-path-input" value={customClientId} onChange={event => setCustomClientId(event.target.value)} /></label><div className="new-op-actions setup-wrap"><button className="target-select enabled" onClick={saveCustom}>Use Custom Application</button><button className="target-select enabled" disabled={auth?.mode === "official"} onClick={restoreOfficial}>Restore Official RenderNorth Client ID</button></div></div>}
    {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
    <div className="new-op-actions setup-wrap">
      <button className="target-select enabled" onClick={handleAddCharacter} disabled={adding || syncing !== null}>{adding ? "Waiting for browser login…" : "Add / Reauthorize Character"}</button>
      <button className="target-select enabled" onClick={handleSyncAll} disabled={adding || syncing !== null}>{syncing === "all" ? "Syncing enabled characters…" : "Sync All Assets"}</button>
      <button className="target-select enabled" onClick={handleBlueprintSyncAll} disabled={adding || blueprintSyncing !== null}>{blueprintSyncing === "all" ? "Syncing blueprints…" : "Sync All Blueprints"}</button>
      <button className="target-select enabled" onClick={handleLocations} disabled={adding || locationSyncing || characters?.length === 0}>{locationSyncing ? "Resolving locations…" : "Resolve Locations"}</button>
    </div>
    {locationFeedback && <div className="setup-feedback">Locations: {locationFeedback}</div>}
    {characters?.length === 0 && <p className="ph-mission">No characters connected. Add a character to continue setup.</p>}
    {characters && characters.length > 0 && <div className="character-setup-list">
      {characters.map(character => <div className="character-setup-card" key={character.characterId}>
        <div className="character-setup-head"><div className="inv-name">{character.name}</div><div className={`inv-status ${statusTone(character)}`}>{character.authorizationStatus}</div><button className="target-select enabled" onClick={() => handleToggleEnabled(character)}>{character.enabled ? "Enabled" : "Disabled"}</button></div>
        <div className="character-progress">
          <div className={character.authorizationStatus === "authorized" ? "complete" : "incomplete"}><span>{character.authorizationStatus === "authorized" ? "✓" : "!"}</span>Authorized</div>
          <div className={character.syncStatus === "success" ? "complete" : "incomplete"}><span>{character.syncStatus === "success" ? "✓" : "!"}</span>Assets · {character.assetCount.toLocaleString()}</div>
          <div className={character.blueprintSyncStatus === "success" ? "complete" : "incomplete"}><span>{character.blueprintSyncStatus === "success" ? "✓" : "!"}</span>Blueprints · {character.blueprintCount.toLocaleString()}</div>
          <div className={character.structureScopeGranted ? "complete" : "incomplete"}><span>{character.structureScopeGranted ? "✓" : "!"}</span>Locations permission</div>
        </div>
        <div className="data-source">Assets: {character.syncStatus} · {character.lastSyncAt ? new Date(character.lastSyncAt).toLocaleString() : "never"} · {character.pageCount} pages<br />Blueprints: {character.blueprintSyncStatus} · {character.blueprintLastSyncAt ? new Date(character.blueprintLastSyncAt).toLocaleString() : "never"} · {character.blueprintPageCount} pages</div>
        {(character.syncError || character.blueprintSyncError) && <div className="sd-error"><div className="conflict-desc">{character.syncError || character.blueprintSyncError}</div></div>}
        {(!character.assetScopeGranted || !character.blueprintScopeGranted || !character.structureScopeGranted) && <div className="setup-scope-explanation"><strong>Reauthorization adds missing read-only permissions:</strong>{!character.assetScopeGranted && <span><code>esi-assets.read_assets.v1</code> — personal asset types, quantities, and locations.</span>}{!character.blueprintScopeGranted && <span><code>esi-characters.read_blueprints.v1</code> — owned BPO/BPC records, ME, TE, and runs.</span>}{!character.structureScopeGranted && <span><code>esi-universe.read_structures.v1</code> — names of accessible player structures; inaccessible structures remain raw IDs.</span>}<span>Existing working read-only permissions remain requested during reauthorization.</span></div>}
        <div className="new-op-actions setup-wrap"><button className="target-select enabled" disabled={!character.assetScopeGranted || syncing !== null} onClick={() => handleSyncOne(character.characterId)}>{syncing === character.characterId ? "Syncing…" : "Sync Assets"}</button><button className="target-select enabled" disabled={!character.blueprintScopeGranted || blueprintSyncing !== null} onClick={() => handleBlueprintSyncOne(character.characterId)}>{blueprintSyncing === character.characterId ? "Syncing…" : "Sync Blueprints"}</button>{confirmRemove === character.characterId ? <><button className="target-select destructive" onClick={() => handleRemove(character.characterId)}>Confirm Remove</button><button className="target-select" onClick={() => setConfirmRemove(null)}>Cancel</button></> : <button className="target-select destructive" onClick={() => setConfirmRemove(character.characterId)}>Remove</button>}</div>
      </div>)}
    </div>}
  </Panel>;
}

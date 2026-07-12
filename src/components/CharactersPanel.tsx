import { useEffect, useState } from "react";
import {
  addCharacter, removeCharacter, setCharacterEnabled, listCharacters,
  getAppSetting, setAppSetting, syncCharacterAssets, syncAllCharacterAssets,
  type CharacterSummary,
} from "../lib/backend";
import { Panel } from "./Panel";

function statusTone(c: CharacterSummary): "nominal" | "furnace" | "alert" {
  if (c.authorizationStatus === "revoked" || c.syncStatus === "error") return "alert";
  if (!c.assetScopeGranted || c.authorizationStatus === "expired") return "furnace";
  return "nominal";
}

export function CharactersPanel() {
  const [clientId, setClientId] = useState("");
  const [characters, setCharacters] = useState<CharacterSummary[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<number | null>(null);
  const [syncing, setSyncing] = useState<number | "all" | null>(null);

  function refresh() { listCharacters().then(setCharacters); }
  useEffect(refresh, []);
  useEffect(() => { getAppSetting("esi_client_id").then((saved) => { if (saved) setClientId(saved); }); }, []);

  function handleClientIdChange(value: string) {
    setClientId(value);
    setAppSetting("esi_client_id", value).catch(() => undefined);
  }

  async function handleAddCharacter() {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's client ID first."); return; }
    setAdding(true); setError(null);
    try { await addCharacter(clientId.trim()); refresh(); } catch (err) { setError(String(err)); }
    finally { setAdding(false); }
  }

  async function handleSyncOne(characterId: number) {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's client ID first."); return; }
    setSyncing(characterId); setError(null);
    try { const result = await syncCharacterAssets(clientId.trim(), characterId); if (result.error) setError(result.error); }
    catch (err) { setError(String(err)); }
    finally { setSyncing(null); refresh(); }
  }

  async function handleSyncAll() {
    if (!clientId.trim()) { setError("Enter your EVE Developer application's client ID first."); return; }
    setSyncing("all"); setError(null);
    try {
      const results = await syncAllCharacterAssets(clientId.trim());
      const failures = results.filter((r) => r.error).map((r) => r.error).join("; ");
      if (failures) setError(failures);
    } catch (err) { setError(String(err)); }
    finally { setSyncing(null); refresh(); }
  }

  async function handleToggleEnabled(c: CharacterSummary) { await setCharacterEnabled(c.characterId, !c.enabled); refresh(); }
  async function handleRemove(characterId: number) { await removeCharacter(characterId); setConfirmRemove(null); refresh(); }

  return <Panel title="Characters & Asset Sync" keel="coolant">
    <p className="ph-mission">
      Connect characters through official EVE SSO and synchronize read-only personal assets. Characters connected
      before Sprint 011B must use Add Character again to grant <code>esi-assets.read_assets.v1</code>.
      Tokens remain in Windows Credential Manager and are never exposed here.
    </p>
    <label className="new-op-field" style={{ marginTop: 10 }}>
      <span className="ops-field-label">EVE Developer application client ID</span>
      <input className="sd-path-input" value={clientId} onChange={(e) => setClientId(e.target.value)}
        onBlur={(e) => handleClientIdChange(e.target.value)} placeholder="Paste your application's client ID" />
    </label>
    {error && <div className="sd-error" style={{ marginTop: 8 }}><div className="conflict-desc">{error}</div></div>}
    <div className="new-op-actions" style={{ marginTop: 10 }}>
      <button className="target-select enabled" onClick={handleAddCharacter} disabled={adding || syncing !== null}>
        {adding ? "Waiting for browser login…" : "Add / Reauthorize Character"}
      </button>
      <button className="target-select enabled" onClick={handleSyncAll} disabled={adding || syncing !== null}>
        {syncing === "all" ? "Syncing enabled characters…" : "Sync All Enabled Characters"}
      </button>
    </div>
    {characters?.length === 0 && <p className="ph-mission" style={{ marginTop: 14 }}>No characters connected yet.</p>}
    {characters && characters.length > 0 && <div className="inv-table" style={{ marginTop: 14 }}>
      <div className="inv-row inv-head"><div>Character</div><div>Authorization</div><div>Enabled</div><div>Asset Sync</div><div>Actions</div></div>
      {characters.map((c) => <div className="inv-row" key={c.characterId}>
        <div className="inv-name">{c.name}</div>
        <div className={`inv-status ${statusTone(c)}`}>{c.authorizationStatus}{!c.assetScopeGranted && <><br />reauthorize</>}</div>
        <div><button className="target-select enabled" onClick={() => handleToggleEnabled(c)}>{c.enabled ? "ON" : "OFF"}</button></div>
        <div className="bts-result-meta">
          {c.syncStatus} · {c.assetCount.toLocaleString()} assets · {c.pageCount} pages<br />
          {c.lastSyncAt ? new Date(c.lastSyncAt).toLocaleString() : "Never synced"}
          {c.syncError && <><br />{c.syncError}</>}
        </div>
        <div>
          <button className="target-select enabled" disabled={!c.assetScopeGranted || syncing !== null}
            onClick={() => handleSyncOne(c.characterId)}>{syncing === c.characterId ? "Syncing…" : "Sync"}</button>{" "}
          {confirmRemove === c.characterId ? <>
            <button className="target-select destructive" onClick={() => handleRemove(c.characterId)}>Confirm</button>{" "}
            <button className="target-select" onClick={() => setConfirmRemove(null)}>Cancel</button>
          </> : <button className="target-select destructive" onClick={() => setConfirmRemove(c.characterId)}>Remove</button>}
        </div>
      </div>)}
    </div>}
  </Panel>;
}

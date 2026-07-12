import { useEffect, useState } from "react";
import {
  addCharacter,
  removeCharacter,
  setCharacterEnabled,
  listCharacters,
  getAppSetting,
  setAppSetting,
  type CharacterSummary,
} from "../lib/backend";
import { Panel } from "./Panel";

function statusTone(c: CharacterSummary): "nominal" | "furnace" | "alert" {
  if (c.authorizationStatus === "revoked") return "alert";
  if (c.authorizationStatus === "expired") return "furnace";
  return "nominal";
}

export function CharactersPanel() {
  const [clientId, setClientId] = useState("");
  const [characters, setCharacters] = useState<CharacterSummary[] | null>(null);
  const [adding, setAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [confirmRemove, setConfirmRemove] = useState<number | null>(null);

  function refresh() {
    listCharacters().then(setCharacters);
  }
  useEffect(refresh, []);
  useEffect(() => {
    getAppSetting("esi_client_id").then((saved) => {
      if (saved) setClientId(saved);
    });
  }, []);

  function handleClientIdChange(value: string) {
    setClientId(value);
    setAppSetting("esi_client_id", value).catch(() => {
      // Non-fatal — the field still works for this session even if
      // persisting it fails; the user just has to paste it again next
      // time the app starts.
    });
  }

  async function handleAddCharacter() {
    if (!clientId.trim()) {
      setAddError("Enter your EVE Developer application's client ID first.");
      return;
    }
    setAdding(true);
    setAddError(null);
    try {
      await addCharacter(clientId.trim());
      refresh();
    } catch (err) {
      setAddError(String(err));
    } finally {
      setAdding(false);
    }
  }

  async function handleToggleEnabled(c: CharacterSummary) {
    await setCharacterEnabled(c.characterId, !c.enabled);
    refresh();
  }

  async function handleRemove(characterId: number) {
    await removeCharacter(characterId);
    setConfirmRemove(null);
    refresh();
  }

  return (
    <Panel title="Characters" keel="coolant">
      <p className="ph-mission">
        Connect any number of EVE characters via official EVE SSO. Each authorization is character-specific — adding
        a second character from the same or a different EVE account is just repeating this flow. Nothing about which
        characters belong to the same account is inferred or stored. Authentication only — no asset data, no
        gameplay actions, no EVE username or password is ever requested by this app.
      </p>

      <label className="new-op-field" style={{ marginTop: 10 }}>
        <span className="ops-field-label">EVE Developer application client ID</span>
        <input
          className="sd-path-input"
          value={clientId}
          onChange={(e) => setClientId(e.target.value)}
          onBlur={(e) => handleClientIdChange(e.target.value)}
          placeholder="Paste your application's client ID from developers.eveonline.com"
        />
      </label>

      {addError && (
        <div className="sd-error" style={{ marginTop: 8 }}>
          <div className="conflict-desc">{addError}</div>
        </div>
      )}

      <div className="new-op-actions" style={{ marginTop: 10 }}>
        <button className="target-select enabled" onClick={handleAddCharacter} disabled={adding}>
          {adding ? "Waiting for login in your browser…" : "Add Character"}
        </button>
      </div>

      {characters && characters.length === 0 && (
        <p className="ph-mission" style={{ marginTop: 14 }}>No characters connected yet.</p>
      )}

      {characters && characters.length > 0 && (
        <div className="inv-table" style={{ marginTop: 14 }}>
          <div className="inv-row inv-head">
            <div>Character</div>
            <div>Authorization</div>
            <div>Enabled</div>
            <div>Last Login</div>
            <div>Actions</div>
          </div>
          {characters.map((c) => (
            <div className="inv-row" key={c.characterId}>
              <div className="inv-name">{c.name}</div>
              <div className={`inv-status ${statusTone(c)}`}>{c.authorizationStatus}</div>
              <div>
                <button className="target-select enabled" onClick={() => handleToggleEnabled(c)} style={{ padding: "3px 10px", fontSize: 11 }}>
                  {c.enabled ? "ON" : "OFF"}
                </button>
              </div>
              <div className="bts-result-meta">
                {c.lastLoginAt ? new Date(c.lastLoginAt).toLocaleString() : "never"}
              </div>
              <div>
                {confirmRemove === c.characterId ? (
                  <span style={{ display: "flex", gap: 6 }}>
                    <button className="target-select destructive" onClick={() => handleRemove(c.characterId)} style={{ padding: "3px 10px", fontSize: 11 }}>
                      Confirm
                    </button>
                    <button className="target-select" onClick={() => setConfirmRemove(null)} style={{ padding: "3px 10px", fontSize: 11 }}>
                      Cancel
                    </button>
                  </span>
                ) : (
                  <button className="target-select destructive" onClick={() => setConfirmRemove(c.characterId)} style={{ padding: "3px 10px", fontSize: 11 }}>
                    Remove
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}

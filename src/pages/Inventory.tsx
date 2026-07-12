import { useEffect, useState } from "react";
import {
  listManualInventory,
  addManualInventoryEntry,
  updateManualInventoryQuantity,
  removeManualInventoryEntry,
  formatQty,
  type ManualInventoryEntry,
  type TypeSearchResult,
  listSyncedAssets,
  type SyncedAsset,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { BuildTargetSearch } from "../components/BuildTargetSearch";
import { PasteInventory } from "../components/PasteInventory";

function ManualInventorySection() {
  const [entries, setEntries] = useState<ManualInventoryEntry[]>([]);
  const [target, setTarget] = useState<TypeSearchResult | null>(null);
  const [quantity, setQuantity] = useState(1);
  const [location, setLocation] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editQuantity, setEditQuantity] = useState(0);
  const [showPaste, setShowPaste] = useState(false);

  function refresh() {
    listManualInventory().then(setEntries);
  }

  useEffect(refresh, []);

  async function handleAdd() {
    if (!target) return;
    setError(null);
    try {
      await addManualInventoryEntry({ typeId: target.typeId, quantity, locationName: location || null });
      setTarget(null);
      setQuantity(1);
      setLocation("");
      refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleSaveEdit(id: number) {
    await updateManualInventoryQuantity(id, editQuantity);
    setEditingId(null);
    refresh();
  }

  async function handleRemove(id: number) {
    await removeManualInventoryEntry(id);
    refresh();
  }

  return (
    <>
      {showPaste && (
        <>
          <PasteInventory onClose={() => setShowPaste(false)} onImported={refresh} />
          <div style={{ height: 14 }} />
        </>
      )}
    <Panel
      title="Manual Inventory"
      keel="coolant"
      className="dash-hero"
      headerRight={
        <button className="target-select enabled" onClick={() => setShowPaste((v) => !v)}>
          {showPaste ? "Close Paste" : "Paste Inventory"}
        </button>
      }
    >
      <p className="ph-mission">
        Real, user-entered stock — kept entirely separate from the demo inventory below. Requires static data to be
        imported (Settings → Static Data) so a type can be searched by name.
      </p>

      <div className="manual-inv-form">
        <div className="new-op-field" style={{ minWidth: 280 }}>
          <span className="ops-field-label">Type</span>
          <BuildTargetSearch selected={target} onSelect={setTarget} />
        </div>
        <label className="new-op-field">
          <span className="ops-field-label">Quantity</span>
          <input className="sd-path-input" type="number" min={1} value={quantity} onChange={(e) => setQuantity(Math.max(1, Number(e.target.value)))} />
        </label>
        <label className="new-op-field">
          <span className="ops-field-label">Location (optional)</span>
          <input className="sd-path-input" value={location} onChange={(e) => setLocation(e.target.value)} />
        </label>
        <button className="target-select enabled" onClick={handleAdd} disabled={!target}>
          Add Entry
        </button>
      </div>

      {error && (
        <div className="sd-error">
          <div className="conflict-desc">{error}</div>
        </div>
      )}

      {entries.length === 0 ? (
        <p className="ph-mission">No manual inventory entries yet.</p>
      ) : (
        <div className="inv-table">
          <div className="inv-row bp-row bp-head">
            <div>Type</div>
            <div>Quantity</div>
            <div>Location</div>
            <div>Updated</div>
            <div>Actions</div>
          </div>
          {entries.map((e) => (
            <div className="inv-row bp-row" key={e.id}>
              <div className="inv-name">{e.typeName}</div>
              <div className="inv-qty">
                {editingId === e.id ? (
                  <input
                    className="sd-path-input"
                    style={{ width: 80 }}
                    type="number"
                    value={editQuantity}
                    onChange={(ev) => setEditQuantity(Number(ev.target.value))}
                  />
                ) : (
                  formatQty(e.quantity)
                )}
              </div>
              <div className="inv-location">{e.locationName}</div>
              <div className="inv-location">{e.updatedAt}</div>
              <div>
                {editingId === e.id ? (
                  <button className="target-select enabled" onClick={() => handleSaveEdit(e.id)}>
                    Save
                  </button>
                ) : (
                  <>
                    <button
                      className="target-select enabled"
                      onClick={() => {
                        setEditingId(e.id);
                        setEditQuantity(e.quantity);
                      }}
                    >
                      Edit
                    </button>{" "}
                    <button className="target-select" onClick={() => handleRemove(e.id)}>
                      Remove
                    </button>
                  </>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </Panel>
    </>
  );
}

function SyncedAssetsSection() {
  const [assets, setAssets] = useState<SyncedAsset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  async function refresh() {
    setLoading(true); setError(null);
    try { setAssets(await listSyncedAssets()); }
    catch (err) { setError(String(err)); }
    finally { setLoading(false); }
  }
  useEffect(() => { refresh(); }, []);
  return <Panel title="Synchronized ESI Assets" keel="furnace" className="dash-hero"
    headerRight={<button className="target-select enabled" onClick={refresh} disabled={loading}>{loading ? "Loading…" : "Refresh"}</button>}>
    <p className="ph-mission">Read-only personal assets from connected characters. Raw location IDs are shown; structure-name resolution and location filtering are deferred.</p>
    {error ? <div className="sd-error"><div className="conflict-desc">Failed to load synchronized assets: {error}</div></div> :
    loading ? <p className="ph-mission">Loading synchronized assets…</p> :
    assets.length === 0 ? <p className="ph-mission">No synchronized character assets yet.</p> :
      <div className="inv-table" style={{ overflowX: "auto" }}>
        <div className="inv-row" style={{ gridTemplateColumns: "1fr 1.4fr .7fr 1fr 1fr .8fr 1fr .7fr 1fr 1.2fr", minWidth: 1300 }}>
          <div>Owner</div><div>Type</div><div>Quantity</div><div>Item ID</div><div>Location ID</div>
          <div>Location Type</div><div>Location Flag</div><div>Singleton</div><div>Source</div><div>Last Synced</div>
        </div>
        {assets.map((a) => <div className="inv-row" key={`${a.characterId}-${a.itemId}`}
          style={{ gridTemplateColumns: "1fr 1.4fr .7fr 1fr 1fr .8fr 1fr .7fr 1fr 1.2fr", minWidth: 1300 }}>
          <div className="inv-name">{a.characterOwner}</div><div>{a.typeName}</div><div>{formatQty(a.quantity)}</div>
          <div>{a.itemId}</div><div>{a.locationId}</div><div>{a.locationType}</div><div>{a.locationFlag}</div>
          <div>{a.singleton ? "Yes" : "No"}</div><div>{a.source}</div><div>{new Date(a.lastSynced).toLocaleString()}</div>
        </div>)}
      </div>}
  </Panel>;
}

export function InventoryPage() {
  return (
    <div className="dash">
      <SyncedAssetsSection />
      <ManualInventorySection />
    </div>
  );
}

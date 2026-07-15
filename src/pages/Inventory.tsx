import { useEffect, useState } from "react";
import {
  listManualInventory,
  addManualInventoryEntry,
  updateManualInventoryQuantity,
  removeManualInventoryEntry,
  formatQty,
  formatVolume,
  type ManualInventoryEntry,
  type TypeSearchResult,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { BuildTargetSearch } from "../components/BuildTargetSearch";
import { PasteInventory } from "../components/PasteInventory";
import { MarketInventory } from "../components/MarketInventory";

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
        Real, user-entered stock — kept entirely separate from synchronized character inventory. Static data must be
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
          <div className="inv-row" style={{ gridTemplateColumns: "1.4fr .7fr .7fr .8fr 1fr 1fr 1fr", minWidth: 1000 }}>
            <div>Type</div>
            <div>Quantity</div>
            <div>Unit m³</div>
            <div>Stack m³</div>
            <div>Location</div>
            <div>Updated</div>
            <div>Actions</div>
          </div>
          {entries.map((e) => (
            <div className="inv-row" style={{ gridTemplateColumns: "1.4fr .7fr .7fr .8fr 1fr 1fr 1fr", minWidth: 1000 }} key={e.id}>
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
              <div className="inv-qty">{formatVolume(e.unitVolumeM3)}</div>
              <div className="inv-qty">{formatVolume(e.stackVolumeM3)}</div>
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

export function InventoryPage() {
  return (
    <div className="dash">
      <MarketInventory />
      <ManualInventorySection />
    </div>
  );
}

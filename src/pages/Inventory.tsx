import { useEffect, useMemo, useState } from "react";
import {
  getInventorySummary,
  listInventoryCategories,
  listInventoryItems,
  listManualInventory,
  addManualInventoryEntry,
  updateManualInventoryQuantity,
  removeManualInventoryEntry,
  formatIsk,
  formatQty,
  type InventorySummary,
  type InventoryCategory,
  type InventoryItem,
  type ManualInventoryEntry,
  type TypeSearchResult,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";
import { BuildTargetSearch } from "../components/BuildTargetSearch";
import { PasteInventory } from "../components/PasteInventory";

function statusTone(status: string): "nominal" | "furnace" | "alert" | "coolant" {
  if (status === "Available") return "nominal";
  if (status === "Reserved" || status === "Partially Reserved" || status === "Allocated") return "furnace";
  if (status === "Destroyed") return "alert";
  return "coolant";
}

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

export function InventoryPage() {
  const [summary, setSummary] = useState<InventorySummary | null>(null);
  const [categories, setCategories] = useState<InventoryCategory[]>([]);
  const [items, setItems] = useState<InventoryItem[] | null>(null);
  const [activeCategory, setActiveCategory] = useState<string | undefined>(undefined);

  useEffect(() => {
    let live = true;
    getInventorySummary().then((s) => live && setSummary(s));
    listInventoryCategories().then((c) => live && setCategories(c));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    let live = true;
    setItems(null);
    listInventoryItems(activeCategory).then((i) => live && setItems(i));
    return () => {
      live = false;
    };
  }, [activeCategory]);

  const totalItemCount = useMemo(
    () => categories.reduce((sum, c) => sum + c.itemCount, 0),
    [categories],
  );

  if (!summary) {
    return <div className="data-source">Bringing Inventory Engine online…</div>;
  }

  return (
    <div className="dash">
      <ManualInventorySection />

      <div className="data-source" style={{ gridColumn: "1 / -1", marginTop: 4 }}>
        Below: the Sprint 003 demo-seeded inventory <span className="demo-badge">DEMO</span>
      </div>

      <div className="stat-strip">
        <StatCard label="Total Assets" value={formatQty(summary.totalAssets)} tone="coolant" keel="coolant" note="units across every category" />
        <StatCard label="Estimated Value" value={formatIsk(summary.estimatedValue)} tone="nominal" keel="nominal" note="quantity × unit value" />
        <StatCard label="Unique Item Types" value={String(summary.uniqueItemTypes)} tone="coolant" keel="coolant" note="distinct type names" />
        <StatCard label="Locations" value={String(summary.locations)} tone="coolant" keel="coolant" note="stations, structures, safety, transit" />
        <StatCard label="Reserved" value={formatIsk(summary.reservedValue)} tone="furnace" keel="furnace" note="held against operations" />
        <StatCard label="Available" value={formatIsk(summary.availableValue)} tone="nominal" keel="nominal" note="free to allocate" />
      </div>

      <div className="inv-layout">
        <Panel title="Categories" keel="coolant" className="inv-rail">
          <div className="cat-list">
            <button
              className={activeCategory === undefined ? "cat-row active" : "cat-row"}
              onClick={() => setActiveCategory(undefined)}
            >
              <span>Everything</span>
              <span className="cat-count">{formatQty(totalItemCount)}</span>
            </button>
            {categories.map((c) => (
              <button
                key={c.key}
                className={activeCategory === c.key ? "cat-row active" : "cat-row"}
                onClick={() => setActiveCategory(c.key)}
              >
                <span>{c.label}</span>
                <span className="cat-count">{formatQty(c.itemCount)}</span>
              </button>
            ))}
          </div>
        </Panel>

        <Panel
          title={activeCategory ? categories.find((c) => c.key === activeCategory)?.label ?? "Items" : "Everything"}
          keel="furnace"
          className="inv-table-panel"
        >
          {!items ? (
            <div className="data-source">Loading items…</div>
          ) : items.length === 0 ? (
            <p className="ph-mission">No items in this category yet.</p>
          ) : (
            <div className="inv-table">
              <div className="inv-row inv-head">
                <div>Name</div>
                <div>Quantity</div>
                <div>Reserved</div>
                <div>Free</div>
                <div>Available</div>
                <div>Location</div>
                <div>Owner</div>
                <div>Operation</div>
                <div>Status</div>
              </div>
              {items.map((item) => {
                const tone = statusTone(item.status);
                const operation = item.reservedOperation ?? item.allocatedOperation;
                return (
                  <div className="inv-row" key={item.itemId}>
                    <div className="inv-name">{item.typeName}</div>
                    <div className="inv-qty">{formatQty(item.quantity)}</div>
                    <div className="inv-reserved">{item.reservedQuantity > 0 ? formatQty(item.reservedQuantity) : "—"}</div>
                    <div className="inv-qty">{formatQty(item.freeQuantity)}</div>
                    <div className="inv-qty">{formatQty(item.availableQuantity)}</div>
                    <div className="inv-location">{item.locationName}</div>
                    <div className="inv-owner">{item.ownerName}</div>
                    <div className="inv-operation">{operation ?? "—"}</div>
                    <div className={`inv-status ${tone}`}>{item.status}</div>
                  </div>
                );
              })}
            </div>
          )}
        </Panel>
      </div>

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        Inventory Engine — categories are views over one inventory, never separate systems.
      </div>
    </div>
  );
}

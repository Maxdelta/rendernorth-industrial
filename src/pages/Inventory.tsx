import { useEffect, useRef, useState } from "react";
import {
  listManualInventory,
  addManualInventoryEntry,
  updateManualInventoryQuantity,
  removeManualInventoryEntry,
  formatQty,
  formatVolume,
  type ManualInventoryEntry,
  type TypeSearchResult,
  listSyncedAssets,
  type SyncedAsset,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { BuildTargetSearch } from "../components/BuildTargetSearch";
import { PasteInventory } from "../components/PasteInventory";
import { MarketInventory } from "../components/MarketInventory";
import { LocationDisplay } from "../components/LocationDisplay";
import { createRequestGate, type RequestGate } from "../lib/requestGate";

type OwnerScope = "personal" | "corporation" | "both";
const RAW_ASSET_PAGE_SIZE = 100;

function SynchronizedAssetsSection() {
  const [expanded, setExpanded] = useState(false);
  const [scope, setScope] = useState<OwnerScope>("both");
  const [assets, setAssets] = useState<SyncedAsset[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [page, setPage] = useState(0);
  const requestGate = useRef<RequestGate>(createRequestGate());

  useEffect(() => {
    if (!expanded) return;
    const isCurrent = requestGate.current.begin();
    setLoading(true);
    setError(null);
    listSyncedAssets(scope)
      .then((rows) => { if (isCurrent()) setAssets(rows); })
      .catch((cause) => { if (isCurrent()) setError(String(cause)); })
      .finally(() => { if (isCurrent()) setLoading(false); });
    return () => requestGate.current.invalidate();
  }, [expanded, scope]);
  useEffect(() => () => requestGate.current.invalidate(), []);

  const pageCount = Math.max(1, Math.ceil(assets.length / RAW_ASSET_PAGE_SIZE));
  const activePage = Math.min(page, pageCount - 1);
  const firstRow = activePage * RAW_ASSET_PAGE_SIZE;
  const visibleAssets = assets.slice(firstRow, firstRow + RAW_ASSET_PAGE_SIZE);

  return (
    <details
      className="inventory-details-panel dash-hero"
      data-inventory-module="raw-asset-details"
      open={expanded}
      onToggle={(event) => setExpanded(event.currentTarget.open)}
    >
      <summary className="inventory-details-summary">Asset Source Details</summary>
      {expanded && <div className="inventory-details-body">
        <p className="ph-mission">
          Diagnostic synchronized rows for ownership and source verification, division inspection, raw location inspection,
          and synchronization troubleshooting. Corporation assets remain read-only and are not used by Production,
          Quartermaster, or Procurement.
        </p>
        <div className="new-op-mode-toggle" style={{ marginBottom: 12 }}>
          {(["personal", "corporation", "both"] as OwnerScope[]).map((value) => (
            <button
              key={value}
              className={scope === value ? "target-select enabled active" : "target-select enabled"}
              onClick={() => { setScope(value); setPage(0); }}
            >
              {value === "personal" ? "Personal" : value === "corporation" ? "Corporation" : "Both"}
            </button>
          ))}
        </div>
        {error && <div className="sd-error"><div className="conflict-desc">Corporation or personal asset query failed: {error}</div></div>}
        {loading ? <div className="data-source">Loading synchronized assets…</div> : assets.length === 0 ? (
          <div className="data-source">No {scope === "both" ? "synchronized" : scope} assets are available.</div>
        ) : <>
          <div className="inventory-details-pagination">
            <span>{(firstRow + 1).toLocaleString()}–{Math.min(firstRow + RAW_ASSET_PAGE_SIZE, assets.length).toLocaleString()} of {assets.length.toLocaleString()} assets</span>
            <div>
              <button className="target-select enabled" disabled={activePage === 0} onClick={() => setPage((value) => Math.max(0, value - 1))}>Previous</button>{" "}
              <button className="target-select enabled" disabled={activePage >= pageCount - 1} onClick={() => setPage((value) => Math.min(pageCount - 1, value + 1))}>Next</button>
            </div>
          </div>
          <div className="inv-table synchronized-assets-table-wrap">
            <div className="inv-row inv-head synchronized-assets-table-head" style={{ gridTemplateColumns: ".7fr 1fr 1.4fr .7fr .7fr .8fr 1fr 1.8fr .9fr .9fr", minWidth: 1550 }}><div>Owner Type</div><div>Owner Name</div><div>Type</div><div>Quantity</div><div>Unit m³</div><div>Stack m³</div><div>Division</div><div>Location</div><div>Source</div><div>Last synced</div></div>
            {visibleAssets.map((asset) => <div className="inv-row" key={`${asset.ownerType}:${asset.ownerId}:${asset.itemId}`} style={{ gridTemplateColumns: ".7fr 1fr 1.4fr .7fr .7fr .8fr 1fr 1.8fr .9fr .9fr", minWidth: 1550 }}><div><span className={`inv-status ${asset.ownerType === "Corporation" ? "furnace" : "nominal"}`}>{asset.ownerType}</span></div><div>{asset.ownerName}</div><div className="inv-name">{asset.typeName}<div className="bts-result-meta">Type {asset.typeId} · Item {asset.itemId}</div></div><div>{formatQty(asset.quantity)}</div><div>{formatVolume(asset.unitVolumeM3)}</div><div>{formatVolume(asset.stackVolumeM3)}</div><div>{asset.division ?? "—"}<div className="bts-result-meta">{asset.locationFlag}</div></div><div><LocationDisplay location={asset.resolvedLocation}/><div className="bts-result-meta">Raw {asset.locationId} · {asset.locationType}</div></div><div>{asset.source}</div><div>{new Date(asset.lastSynced).toLocaleString()}</div></div>)}
          </div>
        </>}
      </div>}
    </details>
  );
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
      <div className="dash-hero" data-inventory-module="market-valuation"><MarketInventory /></div>
      <ManualInventorySection />
      <SynchronizedAssetsSection />
    </div>
  );
}

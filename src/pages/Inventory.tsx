import { useEffect, useMemo, useState } from "react";
import {
  getInventorySummary,
  listInventoryCategories,
  listInventoryItems,
  formatIsk,
  formatQty,
  type InventorySummary,
  type InventoryCategory,
  type InventoryItem,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";

function statusTone(status: string): "nominal" | "furnace" | "alert" | "coolant" {
  if (status === "Available") return "nominal";
  if (status === "Reserved" || status === "Partially Reserved" || status === "Allocated") return "furnace";
  if (status === "Destroyed") return "alert";
  return "coolant";
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
                <div>Location</div>
                <div>Owner</div>
                <div>Reserved</div>
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
                    <div className="inv-location">{item.locationName}</div>
                    <div className="inv-owner">{item.ownerName}</div>
                    <div className="inv-reserved">{item.reservedQuantity > 0 ? formatQty(item.reservedQuantity) : "—"}</div>
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

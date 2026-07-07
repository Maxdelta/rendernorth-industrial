import { useEffect, useMemo, useState } from "react";
import {
  getRequirementSummary,
  listRequirementLines,
  listRequirementCategories,
  getRequirementShortages,
  getCriticalBottlenecks,
  formatQty,
  type RequirementSummary,
  type RequirementLine,
  type RequirementCategory,
  type RequirementShortage,
  type CriticalBottleneck,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";

export function ProductionPage() {
  const [summary, setSummary] = useState<RequirementSummary | null>(null);
  const [categories, setCategories] = useState<RequirementCategory[]>([]);
  const [lines, setLines] = useState<RequirementLine[] | null>(null);
  const [shortages, setShortages] = useState<RequirementShortage[] | null>(null);
  const [bottlenecks, setBottlenecks] = useState<CriticalBottleneck[] | null>(null);
  const [activeCategory, setActiveCategory] = useState<string | undefined>(undefined);

  useEffect(() => {
    let live = true;
    getRequirementSummary().then((s) => live && setSummary(s));
    listRequirementCategories().then((c) => live && setCategories(c));
    getRequirementShortages().then((s) => live && setShortages(s));
    getCriticalBottlenecks().then((b) => live && setBottlenecks(b));
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    let live = true;
    setLines(null);
    listRequirementLines(undefined, activeCategory).then((l) => live && setLines(l));
    return () => {
      live = false;
    };
  }, [activeCategory]);

  const operationsInView = useMemo(() => {
    if (!lines) return [];
    const seen = new Map<number, string>();
    for (const l of lines) seen.set(l.operationId, l.operationGoal);
    return [...seen.entries()];
  }, [lines]);

  if (!summary) {
    return <div className="data-source">Bringing Production Requirement Engine online…</div>;
  }

  return (
    <div className="dash">
      <div className="stat-strip">
        <StatCard label="Total Requirements" value={String(summary.totalRequirements)} tone="coolant" keel="coolant" note="across every operation" />
        <StatCard label="Satisfied" value={String(summary.satisfied)} tone="nominal" keel="nominal" note="fully covered lines" />
        <StatCard label="Missing" value={String(summary.missing)} tone={summary.missing > 0 ? "alert" : "nominal"} keel={summary.missing > 0 ? "alert" : "nominal"} note="short on at least one input" />
        <StatCard label="Coverage %" value={`${Math.round(summary.coveragePercent)}%`} tone="coolant" keel="coolant" note="weighted by quantity" />
        <StatCard
          label="Critical Bottlenecks"
          value={String(summary.criticalBottleneckCount)}
          tone={summary.criticalBottleneckCount > 0 ? "alert" : "nominal"}
          keel={summary.criticalBottleneckCount > 0 ? "alert" : "nominal"}
          note="shared, unmet materials"
        />
      </div>

      <div className="inv-layout">
        <Panel title="Categories" keel="coolant" className="inv-rail">
          <div className="cat-list">
            <button
              className={activeCategory === undefined ? "cat-row active" : "cat-row"}
              onClick={() => setActiveCategory(undefined)}
            >
              <span>Everything</span>
            </button>
            {categories.map((c) => (
              <button
                key={c.categoryKey}
                className={activeCategory === c.categoryKey ? "cat-row active" : "cat-row"}
                onClick={() => setActiveCategory(c.categoryKey)}
              >
                <span>{c.categoryLabel}</span>
              </button>
            ))}
          </div>
        </Panel>

        <Panel
          title={activeCategory ? categories.find((c) => c.categoryKey === activeCategory)?.categoryLabel ?? "Requirement Tree" : "Requirement Tree"}
          keel="furnace"
          className="inv-table-panel"
        >
          {!lines ? (
            <div className="data-source">Loading requirements…</div>
          ) : lines.length === 0 ? (
            <p className="ph-mission">No requirements in this category yet.</p>
          ) : (
            <div className="inv-table">
              <div className="inv-row req-row req-head">
                <div>Material</div>
                <div>Operation</div>
                <div>Category</div>
                <div>Required</div>
                <div>Owned</div>
                <div>Reserved</div>
                <div>Coverage</div>
                <div>Status</div>
              </div>
              {lines.map((l) => (
                <div className="inv-row req-row" key={l.requirementId}>
                  <div className="inv-name">{l.typeName}</div>
                  <div className="inv-operation">{l.operationGoal}</div>
                  <div className="bp-type">{l.categoryLabel}</div>
                  <div className="inv-qty">{formatQty(l.requiredQuantity)}</div>
                  <div className="inv-qty">{formatQty(l.ownedQuantity)}</div>
                  <div className="inv-qty">{l.reservedForOperation > 0 ? formatQty(l.reservedForOperation) : "—"}</div>
                  <div className="inv-qty">{Math.round(l.coverageFraction * 100)}%</div>
                  <div className={`inv-status ${l.isSatisfied ? "nominal" : "alert"}`}>
                    {l.isSatisfied ? "Satisfied" : "Short"}
                  </div>
                </div>
              ))}
            </div>
          )}
          {operationsInView.length > 0 && (
            <div className="data-source" style={{ marginTop: 10 }}>
              {operationsInView.length} operation{operationsInView.length > 1 ? "s" : ""} represented in this view.
            </div>
          )}
        </Panel>
      </div>

      {bottlenecks && (
        <Panel title="Critical Bottlenecks" keel={bottlenecks.length > 0 ? "alert" : "nominal"} className="dash-hero">
          {bottlenecks.length === 0 ? (
            <p className="ph-mission">No material is currently a shared, unmet bottleneck across operations.</p>
          ) : (
            <div className="conflict-list">
              {bottlenecks.map((b) => (
                <div className="conflict-row" key={b.typeName}>
                  <div className="conflict-type">shared shortage</div>
                  <div className="conflict-desc">
                    {b.typeName} ({formatQty(b.ownedQuantity)} on hand) — required by {b.operationsRequiring.join(", ")}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      )}

      {shortages && (
        <Panel title="Shortage Report" keel={shortages.length > 0 ? "alert" : "nominal"} className="dash-hero">
          {shortages.length === 0 ? (
            <p className="ph-mission">Every requirement line is currently satisfied.</p>
          ) : (
            <div className="conflict-list">
              {shortages.map((s, i) => (
                <div className="conflict-row" key={`${s.typeName}-${s.operationGoal}-${i}`}>
                  <div className="conflict-type">short {formatQty(s.shortage)}</div>
                  <div className="conflict-desc">
                    {s.typeName} for {s.operationGoal} — {formatQty(s.ownedQuantity)} of {formatQty(s.requiredQuantity)} on hand
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      )}

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        Production Requirement Engine — every figure here is derived live from inventory, never a stored flag.
      </div>
    </div>
  );
}

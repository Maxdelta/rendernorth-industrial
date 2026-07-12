import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  calculateProductionPlan,
  getOperationsDashboard,
  formatQty,
  type ProductionPlan,
  type OperationSummary,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { TreeNodeRow } from "../components/TreeNodeRow";
import { toCsv, toMarkdown, copyToClipboard, downloadFile } from "../lib/materialListExport";

function RealProductionPlanSection() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [operations, setOperations] = useState<OperationSummary[]>([]);
  const [plan, setPlan] = useState<ProductionPlan | null>(null);
  const [viewMode, setViewMode] = useState<"tree" | "flat">("tree");
  const [validationMode, setValidationMode] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const opId = Number(searchParams.get("op")) || null;

  useEffect(() => {
    getOperationsDashboard().then((d) => setOperations(d.currentOperations));
  }, []);

  useEffect(() => {
    if (!opId) {
      setPlan(null);
      return;
    }
    setError(null);
    setPlan(null);
    calculateProductionPlan(opId)
      .then(setPlan)
      .catch((err) => setError(String(err)));
  }, [opId]);

  async function handleExport(kind: "csv" | "markdown" | "clipboard") {
    if (!plan) return;
    if (kind === "clipboard") {
      await copyToClipboard(toMarkdown(plan));
      setExportMessage("Copied to clipboard.");
    } else if (kind === "csv") {
      downloadFile(`material-list-${plan.operationId}.csv`, toCsv(plan), "text/csv");
      setExportMessage("CSV downloaded.");
    } else {
      downloadFile(`material-list-${plan.operationId}.md`, toMarkdown(plan), "text/markdown");
      setExportMessage("Markdown downloaded.");
    }
    setTimeout(() => setExportMessage(null), 3000);
  }

  return (
    <Panel title="Real Production Plan" keel={plan && plan.warnings.length > 0 ? "furnace" : "coolant"} className="dash-hero">
      <p className="ph-mission">
        Calculated recursively from imported static blueprint data, owned/assumed ME &amp; TE, and current inventory —
        never a hardcoded demo quantity. Select a real operation with a build target below.
      </p>
      <div className="new-op-mode-toggle" style={{ flexWrap: "wrap", marginBottom: 14 }}>
        {operations.map((op) => (
          <button
            key={op.operationId}
            className={op.operationId === opId ? "target-select enabled active" : "target-select enabled"}
            onClick={() => setSearchParams({ op: String(op.operationId) })}
          >
            {op.goal}
          </button>
        ))}
      </div>

      {!opId ? (
        <p className="ph-mission">Select an operation above, or open one from the Operations Workspace.</p>
      ) : error ? (
        <div className="sd-error">
          <div className="conflict-type">no plan available</div>
          <div className="conflict-desc">{error}</div>
        </div>
      ) : !plan ? (
        <div className="data-source">Calculating…</div>
      ) : (
        <>
          <div className="res-summary-grid" style={{ marginBottom: 14 }}>
            <div className="res-summary-cell">
              <div className="res-summary-label">Build Target</div>
              <div className="ops-field-value" style={{ fontSize: 14 }}>{plan.buildTargetName}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Requested</div>
              <div className="res-summary-value">{formatQty(plan.requestedQuantity)}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Total Runs</div>
              <div className="res-summary-value">{plan.totalRuns}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Blueprint Source</div>
              <div className="ops-field-value" style={{ fontSize: 13 }}>
                {plan.blueprintMode} · ME {plan.me} · TE {plan.te}
                {plan.synchronizedBlueprints.length > 0 && <><br />Owned: {plan.synchronizedBlueprints.map(b => `${b.ownerName} ${b.isCopy ? "BPC" : "BPO"} ME ${b.me} TE ${b.te}${b.runsRemaining == null ? "" : ` · ${b.runsRemaining} runs`} · ${b.source}`).join("; ")}</>}
              </div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Inventory Scope</div>
              <div className="ops-field-value" style={{ fontSize: 13 }}>
                {plan.inventoryScope
                  .split("_")
                  .map((w) => w[0].toUpperCase() + w.slice(1))
                  .join(" ")}
              </div>
            </div>
          </div>

          {plan.warnings.length > 0 && (
            <div className="tree-warning-list">
              {plan.warnings.map((w, i) => (
                <div className="conflict-row" key={i}>
                  <div className="conflict-type">warning</div>
                  <div className="conflict-desc">{w}</div>
                </div>
              ))}
            </div>
          )}

          <div className="new-op-mode-toggle" style={{ marginTop: 14, marginBottom: 4 }}>
            <button className={viewMode === "tree" ? "target-select enabled active" : "target-select enabled"} onClick={() => setViewMode("tree")}>
              Build Tree
            </button>
            <button className={viewMode === "flat" ? "target-select enabled active" : "target-select enabled"} onClick={() => setViewMode("flat")}>
              Flattened Materials
            </button>
            <button
              className={validationMode ? "target-select enabled active" : "target-select enabled"}
              onClick={() => setValidationMode((v) => !v)}
              title="Engineering verification: blueprint source, activity, and calculation path per row"
            >
              Validation Mode
            </button>
          </div>

          {viewMode === "tree" ? (
            <div className="inv-table" style={{ marginTop: 10 }}>
              <div className="tree-node-row" style={{ fontFamily: "var(--font-display)", fontWeight: 700, fontSize: "10.5px", letterSpacing: "0.1em", textTransform: "uppercase", color: "var(--text-dim)" }}>
                <div></div>
                <div>Material</div>
                <div>Required</div>
                <div>Owned</div>
                <div>Reserved</div>
                <div>Available</div>
                <div>Coverage</div>
                <div>Status</div>
              </div>
              <TreeNodeRow node={plan.tree} depth={0} validationMode={validationMode} />
            </div>
          ) : (
            <div className="inv-table" style={{ marginTop: 10 }}>
              <div className="inv-row req-row req-head">
                <div>Material</div>
                <div>Category</div>
                <div>Required</div>
                <div>Owned</div>
                <div>Reserved</div>
                <div>Available</div>
                <div>Missing</div>
                <div>Status</div>
              </div>
              {plan.leafTotals.map((l) => (
                <div key={l.typeId}>
                  <div className="inv-row req-row">
                    <div className="inv-name">{l.typeName}</div>
                    <div className="bp-type">{l.groupName ?? l.categoryName ?? "—"}</div>
                    <div className="inv-qty">{formatQty(l.requiredQuantity)}</div>
                    <div className="inv-qty">
                      {formatQty(l.ownedQuantity)}
                      {l.ownedQuantity > 0 && <span className="bts-result-meta"> · {l.ownedSource}</span>}
                    </div>
                    <div className="inv-qty">{l.reservedQuantity > 0 ? formatQty(l.reservedQuantity) : "—"}</div>
                    <div className="inv-qty">{formatQty(l.availableQuantity)}</div>
                    <div className="inv-qty">{formatQty(l.missingQuantity)}</div>
                    <div className={`inv-status ${l.isSatisfied ? "nominal" : "alert"}`}>{l.isSatisfied ? "OK" : "Short"}</div>
                  </div>
                  {validationMode && (
                    <div className="validation-detail">
                      <span>Activity: manufacturing</span>
                      <span>Required (leaf total): {formatQty(l.requiredQuantity)}</span>
                      <span>
                        Calculation path{l.calculationPaths.length > 1 ? "s" : ""} ({l.calculationPaths.length}):{" "}
                        {l.calculationPaths.join("  |  ")}
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          <div className="new-op-actions" style={{ marginTop: 14 }}>
            <button className="target-select enabled" onClick={() => handleExport("clipboard")}>
              Copy Material List
            </button>
            <button className="target-select enabled" onClick={() => handleExport("csv")}>
              Export CSV
            </button>
            <button className="target-select enabled" onClick={() => handleExport("markdown")}>
              Export Markdown
            </button>
          </div>
          {exportMessage && <div className="data-source">{exportMessage}</div>}
        </>
      )}
    </Panel>
  );
}

export function ProductionPage() {
  return (
    <div className="dash">
      <RealProductionPlanSection />
    </div>
  );
}

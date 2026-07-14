import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  calculateProductionPlan,
  getOperationsDashboard,
  formatQty,
  formatVolume,
  type ProductionPlan,
  type OperationSummary,
  getOperationEconomics,
  getOperationCostAssumptions,
  saveOperationCostAssumptions,
  resetOperationCostAssumptions,
  type OperationEconomics,
  type CostAssumptions,
  type RevenueBasis,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { TreeNodeRow } from "../components/TreeNodeRow";
import { ShoppingProcurement } from "../components/ShoppingProcurement";
import { toCsv, toMarkdown, copyToClipboard, downloadFile } from "../lib/materialListExport";

const ZERO_COST_ASSUMPTIONS: CostAssumptions = { salesTaxPercent: 0, brokerFeePercent: 0, manufacturingJobCost: 0, haulingCost: 0, otherCost: 0 };
type CostKey = keyof CostAssumptions;
type CostDraft = Record<CostKey, string>;
const COST_FIELDS: Array<[CostKey, string, boolean]> = [
  ["salesTaxPercent", "Sales tax %", false],
  ["brokerFeePercent", "Broker fee %", false],
  ["manufacturingJobCost", "Manufacturing job cost", true],
  ["haulingCost", "Hauling cost", true],
  ["otherCost", "Other cost", true],
];

const toCostDraft = (value: CostAssumptions): CostDraft => ({
  salesTaxPercent: String(value.salesTaxPercent),
  brokerFeePercent: String(value.brokerFeePercent),
  manufacturingJobCost: String(value.manufacturingJobCost),
  haulingCost: String(value.haulingCost),
  otherCost: String(value.otherCost),
});

function parseCostDraft(draft: CostDraft): { value: CostAssumptions | null; error: string | null } {
  const parsed = Object.fromEntries(Object.entries(draft).map(([key, raw]) => [key, Number(raw)])) as unknown as CostAssumptions;
  if (Object.values(draft).some((raw) => raw.trim() === "") || Object.values(parsed).some((value) => !Number.isFinite(value))) return { value: null, error: "Every cost assumption must be a valid number." };
  if (parsed.salesTaxPercent < 0 || parsed.salesTaxPercent > 100 || parsed.brokerFeePercent < 0 || parsed.brokerFeePercent > 100) return { value: null, error: "Sales tax and broker fee must be between 0 and 100 percent." };
  if (parsed.manufacturingJobCost < 0 || parsed.haulingCost < 0 || parsed.otherCost < 0) return { value: null, error: "ISK costs cannot be negative." };
  return { value: parsed, error: null };
}

function readableNumber(raw: string) {
  if (!raw || !Number.isFinite(Number(raw))) return raw;
  const [whole, fraction] = raw.split(".");
  const grouped = Number(whole || "0").toLocaleString("en-US", { maximumFractionDigits: 0 });
  return fraction === undefined ? grouped : `${grouped}.${fraction}`;
}

function sameCosts(left: CostAssumptions | null, right: CostAssumptions | null) {
  return !!left && !!right && (Object.keys(ZERO_COST_ASSUMPTIONS) as CostKey[]).every((key) => left[key] === right[key]);
}

function CostProfitPanel({ operationId, plan }: { operationId: number; plan: ProductionPlan }) {
  const [economics, setEconomics] = useState<OperationEconomics | null>(null);
  const [draft, setDraft] = useState<CostDraft>(toCostDraft(ZERO_COST_ASSUMPTIONS));
  const [saved, setSaved] = useState<CostAssumptions | null>(null);
  const [savedAt, setSavedAt] = useState<string | null>(null);
  const [basis, setBasis] = useState<RevenueBasis | null>(null);
  const [focused, setFocused] = useState<CostKey | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [working, setWorking] = useState(false);
  const parsed = parseCostDraft(draft);
  const dirty = parsed.value != null && !sameCosts(parsed.value, saved);

  function acceptEconomics(next: OperationEconomics) {
    setEconomics(next);
    setBasis(next.revenueBasis);
  }

  useEffect(() => {
    let active = true;
    setWorking(true);
    setError(null);
    setFeedback(null);
    Promise.all([getOperationCostAssumptions(operationId), getOperationEconomics(operationId, null)])
      .then(([state, nextEconomics]) => {
        if (!active) return;
        setSaved(state.assumptions);
        setDraft(toCostDraft(state.assumptions));
        setSavedAt(state.savedAt);
        acceptEconomics(nextEconomics);
      })
      .catch((cause) => active && setError(`Failed to load saved cost assumptions or economics: ${String(cause)}`))
      .finally(() => active && setWorking(false));
    return () => { active = false; };
  }, [operationId, plan]);

  async function apply() {
    if (!parsed.value || !dirty) return;
    setWorking(true);
    setError(null);
    setFeedback(null);
    try {
      const state = await saveOperationCostAssumptions(operationId, parsed.value);
      const nextEconomics = await getOperationEconomics(operationId, basis);
      setSaved(state.assumptions);
      setDraft(toCostDraft(state.assumptions));
      setSavedAt(state.savedAt);
      acceptEconomics(nextEconomics);
      setFeedback("Cost assumptions saved");
    } catch (cause) {
      setError(`Cost assumptions were not saved: ${String(cause)}`);
    } finally {
      setWorking(false);
    }
  }

  async function reset() {
    setWorking(true);
    setError(null);
    setFeedback(null);
    try {
      const state = await resetOperationCostAssumptions(operationId);
      const nextEconomics = await getOperationEconomics(operationId, basis);
      setSaved(state.assumptions);
      setDraft(toCostDraft(state.assumptions));
      setSavedAt(state.savedAt);
      acceptEconomics(nextEconomics);
      setFeedback("Cost assumptions reset to zero");
    } catch (cause) {
      setError(`Cost assumptions could not be reset: ${String(cause)}`);
    } finally {
      setWorking(false);
    }
  }

  async function changeBasis(nextBasis: RevenueBasis) {
    setWorking(true);
    setError(null);
    try {
      acceptEconomics(await getOperationEconomics(operationId, nextBasis));
    } catch (cause) {
      setError(`Revenue basis could not be changed: ${String(cause)}`);
    } finally {
      setWorking(false);
    }
  }

  const isk = (value: number | null) => value == null ? "No market orders" : `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })} ISK`;
  const basisLabel = economics?.revenueBasis === "listed" ? "Listed Sale" : "Immediate Sale";
  const savedStatus = parsed.error
    ? "Invalid unsaved changes"
    : dirty
      ? "Unsaved changes"
    : feedback
      ? `${feedback}${savedAt ? ` · Saved ${new Date(savedAt).toLocaleString()}` : ""}`
      : savedAt
        ? `Saved ${new Date(savedAt).toLocaleString()}`
        : "Using unsaved zero defaults";

  return <Panel title="Cost & Profit" keel="furnace">
    <p className="ph-mission">Replacement cost values every required leaf material, including owned stock. Cash required prices shortages only. Saved assumptions are operation-specific.</p>
    <div className="manual-inv-form" style={{ flexWrap: "wrap" }}>{COST_FIELDS.map(([key, label, isIsk]) => <label className="new-op-field" key={key}><span className="ops-field-label">{label}</span><input className="sd-path-input" type="text" inputMode="decimal" value={focused === key ? draft[key] : readableNumber(draft[key])} onFocus={() => setFocused(key)} onBlur={() => setFocused(null)} onChange={(event) => { const raw = event.target.value.replaceAll(",", ""); if (/^\d*(\.\d*)?$/.test(raw)) setDraft((current) => ({ ...current, [key]: raw })); }} aria-label={`${label}${isIsk ? " in ISK" : ""}`} /></label>)}</div>
    {parsed.error && <div className="sd-error"><div className="conflict-desc">{parsed.error}</div></div>}
    <div className="new-op-actions" style={{ marginTop: 10 }}>
      <button className="target-select enabled" disabled={working || !parsed.value || !dirty} onClick={apply}>Apply Cost Assumptions</button>
      <button className="target-select enabled" disabled={working || (sameCosts(saved, ZERO_COST_ASSUMPTIONS) && !dirty && !parsed.error)} onClick={reset}>Reset to Zero</button>
      <span className="data-source">{savedStatus}</span>
    </div>
    {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
    <div className="new-op-mode-toggle" style={{ marginTop: 14 }}><span className="ops-field-label">Revenue Basis</span><button className={basis === "immediate" ? "target-select enabled active" : "target-select enabled"} disabled={working} onClick={() => changeBasis("immediate")}>Immediate Sale</button><button className={basis === "listed" ? "target-select enabled active" : "target-select enabled"} disabled={working} onClick={() => changeBasis("listed")}>Listed Sale</button></div>
    {economics?.revenueBasisMessage && <div className="tree-warning-list"><div className="conflict-row"><div className="conflict-type">valuation</div><div className="conflict-desc">{economics.revenueBasisMessage}</div></div></div>}
    {!economics ? <div className="data-source">{working ? "Calculating market economics…" : "No economics available"}</div> : <>
      <div className="data-source">{economics.marketProfile} · {economics.stale ? "STALE" : "CURRENT"} · {economics.priceTimestamp ? new Date(economics.priceTimestamp).toLocaleString() : "No price snapshot"} · Profit calculated using {basisLabel}</div>
      <div className="res-summary-grid" style={{ marginTop: 12 }}>{[
        ["Total Material Replacement Cost", economics.totalMaterialReplacementCost, "All required leaf materials at weighted acquisition cost"], ["Owned Material Opportunity Cost", economics.ownedMaterialOpportunityCost, "Owned usable materials consumed; owned inventory is not free"], ["Cash Required", economics.cashRequired, "Weighted acquisition cost of shortages only"], ["Immediate Sale Value", economics.outputImmediateSaleValue, "Weighted fill into available buy orders"], ["Listed Sale Value", economics.outputListedSaleValue, "Weighted valuation against sell orders"], ["Gross Profit", economics.grossProfit, `${basisLabel} value minus replacement cost`], ["Estimated Net Profit", economics.estimatedNetProfit, "Gross profit minus configured fees and costs"], ["Margin", economics.margin == null ? null : economics.margin * 100, `Gross profit divided by ${basisLabel} value`, true], ["ROI", economics.roi == null ? null : economics.roi * 100, "Gross profit divided by replacement cost", true]
      ].map(([label, value, title, percent]) => <div className="res-summary-cell" key={String(label)} title={String(title)}><div className="res-summary-label">{String(label)}</div><div className="ops-field-value">{value == null ? "No market orders" : percent ? `${Number(value).toFixed(2)}%` : isk(Number(value))}</div></div>)}</div>
      <div className="res-summary-grid" style={{ marginTop: 12 }}>
        <div className="res-summary-cell"><div className="res-summary-label">Total material volume</div><div className="ops-field-value">{formatVolume(economics.totalMaterialVolumeM3)}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Purchase volume</div><div className="ops-field-value">{formatVolume(economics.totalPurchaseVolumeM3)}</div></div>
      </div>
      <details style={{ marginTop: 12 }}><summary className="target-select enabled">Calculation Breakdown</summary><div className="inv-table" style={{ overflowX: "auto" }}><div className="inv-row" style={{ gridTemplateColumns: "1.4fr .6fr .7fr .6fr .7fr .6fr .7fr .7fr .8fr .8fr .8fr .8fr", minWidth: 1750 }}><div>Material</div><div>Required</div><div>Required m³</div><div>Owned</div><div>Owned m³</div><div>Shortage</div><div>Purchase m³</div><div>Unit m³</div><div>Unit acquisition</div><div>Replacement</div><div>Owned opportunity</div><div>Shortage cash</div></div>{economics.materials.map((material) => <div className="inv-row" key={material.typeId} style={{ gridTemplateColumns: "1.4fr .6fr .7fr .6fr .7fr .6fr .7fr .7fr .8fr .8fr .8fr .8fr", minWidth: 1750 }}><div className="inv-name">{material.typeName}</div><div>{formatQty(material.requiredQuantity)}</div><div>{formatVolume(material.requiredVolumeM3)}</div><div>{formatQty(material.ownedQuantity)}</div><div>{formatVolume(material.ownedVolumeM3)}</div><div>{formatQty(material.shortageQuantity)}</div><div>{formatVolume(material.purchaseVolumeM3)}</div><div>{formatVolume(material.unitVolumeM3)}</div><div>{isk(material.unitAcquisitionPrice)}</div><div>{isk(material.replacementValue)}</div><div>{isk(material.ownedOpportunityCost)}</div><div>{isk(material.shortageCashCost)}</div></div>)}</div><div className="data-source">Revenue basis: {basisLabel} · Sales tax: {isk(economics.salesTax)} · Broker fee: {isk(economics.brokerFee)} · Manufacturing: {isk(economics.manufacturingJobCost)} · Hauling: {isk(economics.haulingCost)} · Other: {isk(economics.otherCost)} · Gross profit: {isk(economics.grossProfit)} · Estimated net profit: {isk(economics.estimatedNetProfit)}</div></details>
    </>}
  </Panel>;
}

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
            <div className="res-summary-cell"><div className="res-summary-label">Required Volume</div><div className="ops-field-value">{formatVolume(plan.totalRequiredVolumeM3)}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Owned Volume</div><div className="ops-field-value">{formatVolume(plan.totalOwnedVolumeM3)}</div></div>
            <div className="res-summary-cell"><div className="res-summary-label">Missing / Purchase Volume</div><div className="ops-field-value">{formatVolume(plan.totalMissingVolumeM3)}</div></div>
          </div>

          <CostProfitPanel operationId={opId} plan={plan} />
          <ShoppingProcurement operationId={opId} plan={plan} />

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
              <div className="inv-row" style={{ gridTemplateColumns: "1.3fr .8fr .6fr .7fr .6fr .7fr .6fr .6fr .7fr .7fr .7fr .6fr", minWidth: 1600 }}>
                <div>Material</div>
                <div>Category</div>
                <div>Required</div>
                <div>Required m³</div>
                <div>Owned</div>
                <div>Owned m³</div>
                <div>Reserved</div>
                <div>Available</div>
                <div>Missing</div>
                <div>Missing m³</div>
                <div>Unit m³</div>
                <div>Status</div>
              </div>
              {plan.leafTotals.map((l) => (
                <div key={l.typeId}>
                  <div className="inv-row" style={{ gridTemplateColumns: "1.3fr .8fr .6fr .7fr .6fr .7fr .6fr .6fr .7fr .7fr .7fr .6fr", minWidth: 1600 }}>
                    <div className="inv-name">{l.typeName}</div>
                    <div className="bp-type">{l.groupName ?? l.categoryName ?? "—"}</div>
                    <div className="inv-qty">{formatQty(l.requiredQuantity)}</div>
                    <div className="inv-qty">{formatVolume(l.requiredVolumeM3)}</div>
                    <div className="inv-qty">
                      {formatQty(l.ownedQuantity)}
                      {l.ownedQuantity > 0 && <span className="bts-result-meta"> · {l.ownedSource}</span>}
                    </div>
                    <div className="inv-qty">{formatVolume(l.ownedVolumeM3)}</div>
                    <div className="inv-qty">{l.reservedQuantity > 0 ? formatQty(l.reservedQuantity) : "—"}</div>
                    <div className="inv-qty">{formatQty(l.availableQuantity)}</div>
                    <div className="inv-qty">{formatQty(l.missingQuantity)}</div>
                    <div className="inv-qty">{formatVolume(l.missingVolumeM3)}</div>
                    <div className="inv-qty">{formatVolume(l.unitVolumeM3)}</div>
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

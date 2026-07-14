import { useEffect, useState } from "react";
import {
  formatQty,
  formatVolume,
  getOperationShoppingList,
  resetProcurementState,
  updateProcurementLine,
  type OperationShoppingList,
  type ProcurementStatus,
  type ProductionPlan,
} from "../lib/backend";
import { downloadFile } from "../lib/materialListExport";
import { Panel } from "./Panel";

const STATUSES: ProcurementStatus[] = ["Needed", "Planned", "Purchased", "Skipped", "Fulfilled Manually"];
const isk = (value: number | null) => value == null ? "Unpriced" : `${value.toLocaleString(undefined, { maximumFractionDigits: 2 })} ISK`;

export function ShoppingProcurement({ operationId, plan }: { operationId: number; plan: ProductionPlan }) {
  const [list, setList] = useState<OperationShoppingList | null>(null);
  const [draftNotes, setDraftNotes] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [workingType, setWorkingType] = useState<number | null>(null);

  async function reload() {
    setError(null);
    try {
      const next = await getOperationShoppingList(operationId);
      setList(next);
      setDraftNotes(Object.fromEntries(next.lines.map((line) => [line.typeId, line.notes])));
    } catch (cause) {
      setList(null);
      setError(`Shopping list could not be loaded: ${String(cause)}`);
    }
  }

  useEffect(() => { void reload(); }, [operationId, plan]);

  async function save(typeId: number, status: ProcurementStatus) {
    setWorkingType(typeId);
    setError(null);
    setFeedback(null);
    try {
      await updateProcurementLine(operationId, typeId, status, draftNotes[typeId] ?? "");
      await reload();
      setFeedback("Procurement state saved");
    } catch (cause) {
      setError(`Procurement state was not saved: ${String(cause)}`);
    } finally {
      setWorkingType(null);
    }
  }

  async function reset() {
    setWorkingType(-1);
    setError(null);
    setFeedback(null);
    try {
      await resetProcurementState(operationId);
      await reload();
      setFeedback("Procurement state reset to Needed; inventory was not changed");
    } catch (cause) {
      setError(`Procurement state could not be reset: ${String(cause)}`);
    } finally {
      setWorkingType(null);
    }
  }

  async function copy(label: string, value: string) {
    setError(null);
    setFeedback(null);
    try {
      await navigator.clipboard.writeText(value);
      setFeedback(`${label} copied`);
    } catch (cause) {
      setError(`${label} could not be copied: ${String(cause)}`);
    }
  }

  function exportCsv() {
    if (!list) return;
    try {
      downloadFile(`procurement-${operationId}.csv`, list.exports.csv, "text/csv");
      setFeedback("Procurement CSV exported");
      setError(null);
    } catch (cause) {
      setError(`Procurement CSV could not be exported: ${String(cause)}`);
    }
  }

  return <Panel title="Shopping / Procurement" keel={list && (list.summary.unpricedLines > 0 || list.summary.insufficientVolumeLines > 0) ? "furnace" : "coolant"}>
    <p className="ph-mission">Live shortages from this operation’s production plan, scoped inventory, reservations, cached market prices, and CCP volume. Procurement status and notes never change inventory quantities.</p>
    {error && <div className="sd-error"><div className="conflict-desc">{error}</div></div>}
    {feedback && <div className="data-source">{feedback}</div>}
    {!list ? <div className="data-source">Loading shopping list…</div> : <>
      <div className="res-summary-grid" style={{ margin: "12px 0" }}>
        <div className="res-summary-cell"><div className="res-summary-label">Missing item types</div><div className="res-summary-value">{list.summary.missingItemTypes}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Missing units</div><div className="res-summary-value">{formatQty(list.summary.totalMissingUnits)}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Estimated purchase cost</div><div className="ops-field-value">{isk(list.summary.totalPurchaseCost)}</div><div className="bts-result-meta">Priced lines only · {list.marketProfile}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Purchase volume</div><div className="ops-field-value">{formatVolume(list.summary.totalPurchaseVolumeM3)}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Priced / unpriced</div><div className="ops-field-value">{list.summary.pricedLines} / {list.summary.unpricedLines}</div></div>
        <div className="res-summary-cell"><div className="res-summary-label">Insufficient volume</div><div className="ops-field-value">{list.summary.insufficientVolumeLines}</div></div>
      </div>
      <div className="new-op-actions" style={{ marginBottom: 12 }}>
        <button className="target-select enabled" disabled={!list.exports.eveMultiBuy} onClick={() => copy("EVE Multi-Buy list", list.exports.eveMultiBuy)}>Copy EVE Multi-Buy</button>
        <button className="target-select enabled" disabled={!list.exports.discordReport} onClick={() => copy("Discord report", list.exports.discordReport)}>Copy Discord Report</button>
        <button className="target-select enabled" disabled={list.lines.length === 0} onClick={exportCsv}>Export CSV</button>
        <button className="target-select enabled" disabled={workingType !== null} onClick={reset}>Reset Procurement State</button>
      </div>
      {list.lines.length === 0 ? <div className="data-source">No missing materials. This operation currently has nothing to procure.</div> : <div className="inv-table" style={{ overflowX: "auto" }}>
        <div className="inv-row" style={{ gridTemplateColumns: "1.4fr .55fr .55fr .55fr .7fr .8fr .8fr .9fr .8fr 1fr 1.6fr .7fr", minWidth: 1800 }}><div>Item</div><div>Required</div><div>Owned</div><div>Shortage</div><div>Unit m³</div><div>Total m³</div><div>Unit price</div><div>Total cost</div><div>Market</div><div>Status</div><div>Notes</div><div>Updated</div></div>
        {list.lines.map((line) => <div className="inv-row" key={line.typeId} style={{ gridTemplateColumns: "1.4fr .55fr .55fr .55fr .7fr .8fr .8fr .9fr .8fr 1fr 1.6fr .7fr", minWidth: 1800 }}>
          <div className="inv-name">{line.itemName}<div className="bts-result-meta">Type {line.typeId}</div></div>
          <div>{formatQty(line.requiredQuantity)}</div><div>{formatQty(line.ownedQuantity)}</div><div>{formatQty(line.shortageQuantity)}</div>
          <div>{formatVolume(line.unitVolumeM3)}</div><div>{formatVolume(line.totalVolumeM3)}</div><div>{isk(line.acquisitionUnitPrice)}</div><div>{isk(line.totalAcquisitionCost)}</div>
          <div className={`inv-status ${line.marketStatus === "Current" ? "nominal" : "alert"}`}>{line.marketStatus}<div className="bts-result-meta">Available: {formatQty(line.availableMarketVolume)}<br />{line.priceTimestamp ? new Date(line.priceTimestamp).toLocaleString() : "No timestamp"}</div></div>
          <div><select value={line.procurementStatus} disabled={workingType === line.typeId} onChange={(event) => save(line.typeId, event.target.value as ProcurementStatus)}>{STATUSES.map((status) => <option key={status}>{status}</option>)}</select></div>
          <div><input className="sd-path-input" value={draftNotes[line.typeId] ?? ""} placeholder="Procurement note" onChange={(event) => setDraftNotes((current) => ({ ...current, [line.typeId]: event.target.value }))} /></div>
          <div><button className="target-select enabled" disabled={workingType === line.typeId || (draftNotes[line.typeId] ?? "") === line.notes} onClick={() => save(line.typeId, line.procurementStatus)}>Save Note</button><div className="bts-result-meta">{line.lastUpdated ? new Date(line.lastUpdated).toLocaleString() : "Never"}</div></div>
        </div>)}
      </div>}
      {(list.summary.unpricedLines > 0 || list.summary.insufficientVolumeLines > 0) && <div className="tree-warning-list">
        {list.summary.unpricedLines > 0 && <div className="conflict-row"><div className="conflict-type">No market orders</div><div className="conflict-desc">{list.summary.unpricedLines} line(s) are excluded from estimated purchase cost.</div></div>}
        {list.summary.insufficientVolumeLines > 0 && <div className="conflict-row"><div className="conflict-type">Insufficient market volume</div><div className="conflict-desc">{list.summary.insufficientVolumeLines} line(s) cannot be completely filled and are excluded from estimated purchase cost.</div></div>}
      </div>}
    </>}
  </Panel>;
}

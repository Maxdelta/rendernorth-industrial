import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  getOperationsDashboard,
  getInventoryCommitment,
  getReservationConflicts,
  getBlueprintReadinessAll,
  getLatestImport,
  type ImportSummary,
  healthCheck,
  formatQty,
  type OperationsDashboard,
  type OperationSummary,
  type InventoryCommitment,
  type ReservationConflict,
  type BlueprintReadiness,
  type DbHealth,
} from "../lib/backend";
import { Panel } from "../components/Panel";

function opStatusTone(op: OperationSummary): "nominal" | "furnace" | "alert" | "coolant" {
  if (op.isBlocked) return "alert";
  if (op.status === "active") return "nominal";
  if (op.status === "planned") return "coolant";
  return "furnace";
}

export function MissionControlPage() {
  const [ops, setOps] = useState<OperationsDashboard | null>(null);
  const [commitment, setCommitment] = useState<InventoryCommitment | null>(null);
  const [conflicts, setConflicts] = useState<ReservationConflict[] | null>(null);
  const [readiness, setReadiness] = useState<BlueprintReadiness[] | null>(null);
  const [importStatus, setImportStatus] = useState<ImportSummary | null>(null);
  const [health, setHealth] = useState<DbHealth | null>(null);

  useEffect(() => {
    let live = true;
    getOperationsDashboard().then((o) => live && setOps(o));
    getInventoryCommitment().then((c) => live && setCommitment(c));
    getReservationConflicts().then((c) => live && setConflicts(c));
    getBlueprintReadinessAll().then((r) => live && setReadiness(r));
    getLatestImport().then((s) => live && setImportStatus(s));
    healthCheck().then((h) => live && setHealth(h));
    return () => {
      live = false;
    };
  }, []);

  return (
    <div className="dash">
      <Panel title="Real Operations Readiness" keel={importStatus ? "coolant" : "furnace"} className="dash-hero">
        <div className="res-summary-grid">
          <div className="res-summary-cell">
            <div className="res-summary-label">Static Data</div>
            <div className={`res-summary-value ${importStatus ? "nominal" : "furnace"}`}>
              {importStatus ? `${importStatus.typeCount} types` : "Not imported"}
            </div>
          </div>
          {ops && (
            <div className="res-summary-cell">
              <div className="res-summary-label">Real Operations</div>
              <div className="res-summary-value">{ops.currentOperations.length}</div>
            </div>
          )}
        </div>
        {!importStatus && (
          <p className="ph-mission" style={{ marginTop: 12 }}>
            Import static data in Settings to unlock the Build Target Selector and real, blueprint-derived production
            plans. Until then, real operations without a build target can still be created and tracked.
          </p>
        )}
        {ops && ops.currentOperations.length > 0 && (
          <div className="op-simple-list" style={{ marginTop: 12 }}>
            {ops.currentOperations.map((o) => (
              <Link className="op-simple-row" to={`/operations?op=${o.operationId}`} key={o.operationId}>
                <span className="op-simple-goal">{o.goal}</span>
                <span className="op-status coolant">real</span>
              </Link>
            ))}
          </div>
        )}
        {ops && ops.currentOperations.length === 0 && (
          <p className="ph-mission" style={{ marginTop: 12 }}>
            No real builds yet. Create one from Operations → New Build.
          </p>
        )}
      </Panel>

      {ops && (
        <>
          <Panel title="Operation Health" keel={ops.health.blockedOperations > 0 ? "alert" : "nominal"} className="dash-hero">
            <div className="health-grid op-health-grid">
              <div className="health-cell">
                <div className="stat-label">Total Operations</div>
                <div className="stat-value coolant">{ops.health.totalOperations}</div>
              </div>
              <div className="health-cell">
                <div className="stat-label">Active</div>
                <div className="stat-value nominal">{ops.health.activeOperations}</div>
              </div>
              <div className="health-cell">
                <div className="stat-label">Blocked</div>
                <div className={`stat-value ${ops.health.blockedOperations > 0 ? "alert" : "nominal"}`}>
                  {ops.health.blockedOperations}
                </div>
              </div>
              <div className="health-cell">
                <div className="stat-label">Healthy</div>
                <div className={`stat-value ${ops.health.healthyFraction >= 0.6 ? "nominal" : "furnace"}`}>
                  {Math.round(ops.health.healthyFraction * 100)}%
                </div>
              </div>
            </div>
          </Panel>

          <Panel title="Current Operations" keel="coolant" className="dash-hero">
            {ops.currentOperations.length === 0 ? (
              <p className="ph-mission">No real builds yet.</p>
            ) : (
              <div className="op-list">
                <div className="op-row op-head">
                  <div>Goal</div>
                  <div>Target</div>
                  <div>Priority</div>
                  <div>Progress</div>
                  <div>Status</div>
                </div>
                {ops.currentOperations.map((op) => (
                  <Link className="op-row op-link" to={`/operations?op=${op.operationId}`} key={op.operationId}>
                    <div className="op-goal">{op.goal}</div>
                    <div className="op-target">{op.targetTypeName ?? "—"}</div>
                    <div className="op-priority">P{op.priority}</div>
                    <div className="op-progress">{Math.round(op.progress * 100)}%</div>
                    <div className={`op-status ${opStatusTone(op)}`}>{op.isBlocked ? "Blocked" : op.status}</div>
                  </Link>
                ))}
              </div>
            )}
          </Panel>

          <Panel title="Priority Queue" keel="furnace" className="dash-half">
            {ops.priorityQueue.length === 0 ? (
              <p className="ph-mission">Nothing queued.</p>
            ) : (
              <div className="op-simple-list">
                {ops.priorityQueue.map((op) => (
                  <Link className="op-simple-row" to={`/operations?op=${op.operationId}`} key={op.operationId}>
                    <span className="op-simple-priority">P{op.priority}</span>
                    <span className="op-simple-goal">{op.goal}</span>
                    <span className={`op-status ${opStatusTone(op)}`}>{op.isBlocked ? "Blocked" : op.status}</span>
                  </Link>
                ))}
              </div>
            )}
          </Panel>

          <Panel title="Blocked Operations" keel={ops.blocked.length > 0 ? "alert" : "nominal"} className="dash-half">
            {ops.blocked.length === 0 ? (
              <p className="ph-mission">Nothing is blocked right now.</p>
            ) : (
              <div className="op-simple-list">
                {ops.blocked.map((op) => (
                  <Link className="op-simple-row" to={`/operations?op=${op.operationId}`} key={op.operationId}>
                    <span className="op-simple-goal">{op.goal}</span>
                    <span className="op-status alert">Blocked</span>
                  </Link>
                ))}
              </div>
            )}
          </Panel>

          <Panel title="Upcoming Completions" keel="coolant" className="dash-hero">
            {ops.upcomingCompletions.length === 0 ? (
              <p className="ph-mission">No deadlines scheduled.</p>
            ) : (
              <div className="op-simple-list">
                {ops.upcomingCompletions.map((op) => (
                  <Link className="op-simple-row" to={`/operations?op=${op.operationId}`} key={op.operationId}>
                    <span className="op-simple-goal">{op.goal}</span>
                    <span className="op-deadline">{op.deadline}</span>
                    <span className={`op-status ${opStatusTone(op)}`}>{op.isBlocked ? "Blocked" : op.status}</span>
                  </Link>
                ))}
              </div>
            )}
          </Panel>
        </>
      )}

      {commitment && (
        <Panel title="Inventory Commitment" keel={commitment.blocked > 0 ? "alert" : "coolant"} className="dash-hero">
          <div className="health-grid commitment-grid">
            <div className="health-cell">
              <div className="stat-label">Total Inventory</div>
              <div className="stat-value coolant">{formatQty(commitment.totalInventory)}</div>
            </div>
            <div className="health-cell">
              <div className="stat-label">Reserved</div>
              <div className="stat-value furnace">{formatQty(commitment.reserved)}</div>
            </div>
            <div className="health-cell">
              <div className="stat-label">Available</div>
              <div className="stat-value nominal">{formatQty(commitment.available)}</div>
            </div>
            <div className="health-cell">
              <div className="stat-label">Blocked</div>
              <div className={`stat-value ${commitment.blocked > 0 ? "alert" : "nominal"}`}>
                {formatQty(commitment.blocked)}
              </div>
            </div>
            <div className="health-cell">
              <div className="stat-label">Unallocated</div>
              <div className="stat-value coolant">{formatQty(commitment.unallocated)}</div>
            </div>
          </div>
        </Panel>
      )}

      {conflicts && (
        <Panel title="Reservation Conflicts" keel={conflicts.length > 0 ? "alert" : "nominal"} className="dash-hero">
          {conflicts.length === 0 ? (
            <p className="ph-mission">No reservation conflicts detected. Detection only — nothing here resolves a conflict automatically.</p>
          ) : (
            <div className="conflict-list">
              {conflicts.map((c, i) => (
                <div className="conflict-row" key={`${c.itemId}-${c.conflictType}-${i}`}>
                  <div className="conflict-type">{c.conflictType.replace(/_/g, " ")}</div>
                  <div className="conflict-desc">{c.description}</div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      )}

      {readiness && (
        <Panel title="Blueprint Readiness" keel={readiness.some((r) => r.missingCount > 0) ? "alert" : "nominal"} className="dash-hero">
          {readiness.length === 0 ? (
            <p className="ph-mission">No real operation has blueprint requirements recorded yet.</p>
          ) : (
            <div className="readiness-list">
              <div className="readiness-row">
                <div className="ops-field-label">Operation</div>
                <div className="ops-field-label">Owned</div>
                <div className="ops-field-label">Missing</div>
                <div className="ops-field-label">Warnings</div>
              </div>
              {readiness.map((r) => (
                <Link className="readiness-row readiness-link" to={`/operations?op=${r.operationId}`} key={r.operationId}>
                  <div className="readiness-goal">{r.operationGoal}</div>
                  <div className="readiness-metric nominal">{r.ownedCount}</div>
                  <div className={`readiness-metric ${r.missingCount > 0 ? "alert" : "nominal"}`}>{r.missingCount}</div>
                  <div className={`readiness-metric ${r.warningCount > 0 ? "furnace" : "nominal"}`}>{r.warningCount}</div>
                </Link>
              ))}
            </div>
          )}
        </Panel>
      )}

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        {health && (
          <>
            db <b>{health.ok ? "connected" : "error"}</b> · schema v<b>{health.schemaVersion}</b> · {health.dbPath}
          </>
        )}
        {!health && <>Running in browser without the Tauri shell.</>}
      </div>
    </div>
  );
}

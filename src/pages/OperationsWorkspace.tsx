import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  getOperationsDashboard,
  getOperationDetail,
  getOperationReservations,
  getBlueprintReadinessForOperation,
  formatQty,
  type OperationSummary,
  type OperationDetail,
  type OperationReservations,
  type BlueprintReadiness,
} from "../lib/backend";
import { Panel } from "../components/Panel";

function statusTone(status: string, isBlocked: boolean): "nominal" | "furnace" | "alert" | "coolant" {
  if (isBlocked) return "alert";
  if (status === "active") return "nominal";
  if (status === "planned") return "coolant";
  return "furnace";
}

const RESERVED_SECTIONS = ["Production", "Inventory", "Shopping", "Cost", "Timeline"];

export function OperationsWorkspacePage() {
  const [searchParams, setSearchParams] = useSearchParams();
  const [operations, setOperations] = useState<OperationSummary[]>([]);
  const [detail, setDetail] = useState<OperationDetail | null>(null);
  const [reservations, setReservations] = useState<OperationReservations | null>(null);
  const [blueprintReadiness, setBlueprintReadiness] = useState<BlueprintReadiness | null>(null);

  useEffect(() => {
    let live = true;
    getOperationsDashboard().then((d) => {
      if (!live) return;
      setOperations(d.currentOperations);
      const requested = Number(searchParams.get("op"));
      const initial = d.currentOperations.find((o) => o.operationId === requested) ?? d.currentOperations[0];
      if (initial) setSearchParams({ op: String(initial.operationId) }, { replace: true });
    });
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectedId = Number(searchParams.get("op")) || null;

  useEffect(() => {
    if (!selectedId) return;
    let live = true;
    getOperationDetail(selectedId).then((d) => live && setDetail(d));
    getOperationReservations(selectedId).then((r) => live && setReservations(r));
    getBlueprintReadinessForOperation(selectedId).then((r) => live && setBlueprintReadiness(r));
    return () => {
      live = false;
    };
  }, [selectedId]);

  return (
    <div className="dash">
      <div className="ops-workspace-layout">
        <Panel title="Operations" keel="coolant">
          <div className="ops-rail-list">
            {operations.map((op) => (
              <button
                key={op.operationId}
                className={op.operationId === selectedId ? "ops-rail-row active" : "ops-rail-row"}
                onClick={() => setSearchParams({ op: String(op.operationId) })}
              >
                <span className="ops-rail-goal">{op.goal}</span>
                <span className="ops-rail-meta">
                  <span>P{op.priority}</span>
                  <span className={op.isBlocked ? "alert" : ""}>{op.isBlocked ? "Blocked" : op.status}</span>
                  <span>{Math.round(op.progress * 100)}%</span>
                </span>
              </button>
            ))}
          </div>
        </Panel>

        {!detail ? (
          <Panel title="Overview" keel="furnace">
            <div className="data-source">Loading operation…</div>
          </Panel>
        ) : (
          <div>
            <Panel title="Overview" keel={statusTone(detail.status, detail.isBlocked)} className="dash-hero">
              <div className="ops-overview-grid">
                <div>
                  <div className="ops-field-label">Goal</div>
                  <div className="ops-field-value">{detail.goal}</div>
                </div>
                <div>
                  <div className="ops-field-label">Priority</div>
                  <div className="ops-field-value">P{detail.priority}</div>
                </div>
                <div>
                  <div className="ops-field-label">Status</div>
                  <div className={`ops-field-value ${statusTone(detail.status, detail.isBlocked)}`}>
                    {detail.isBlocked ? "Blocked" : detail.status}
                  </div>
                </div>
                <div>
                  <div className="ops-field-label">Progress</div>
                  <div className="ops-field-value">{Math.round(detail.progress * 100)}%</div>
                </div>
              </div>
              {detail.targetTypeName && (
                <>
                  <div className="ops-field-label">Target</div>
                  <div className="ops-field-value" style={{ marginBottom: 14 }}>
                    {detail.targetTypeName}
                  </div>
                </>
              )}
              <div className="ops-field-label">Notes</div>
              <p className="ops-notes">{detail.notes || "No notes recorded for this operation."}</p>
            </Panel>

            <div style={{ height: 14 }} />

            <Panel title="Dependencies" keel={detail.dependencies.length > 0 ? "furnace" : "nominal"} className="dash-hero">
              {detail.dependencies.length === 0 ? (
                <p className="ph-mission">This operation has no dependencies — nothing else has to finish first.</p>
              ) : (
                <div className="ops-deps-list">
                  {detail.dependencies.map((dep) => (
                    <div className="ops-dep-row" key={dep.dependsOnOperationId}>
                      <div className="ops-dep-goal">
                        {dep.dependsOnGoal} <span className="op-status coolant">{dep.dependsOnStatus}</span>
                      </div>
                      <div className="ops-dep-reason">{dep.reason}</div>
                    </div>
                  ))}
                </div>
              )}
            </Panel>

            <div style={{ height: 14 }} />

            {reservations && (
              <>
                <Panel title="Reservations" keel={reservations.missingReservations > 0 ? "furnace" : "nominal"} className="dash-hero">
                  <div className="res-summary-grid">
                    <div className="res-summary-cell">
                      <div className="res-summary-label">Reserved Minerals</div>
                      <div className="res-summary-value">{formatQty(reservations.reservedMinerals)}</div>
                    </div>
                    <div className="res-summary-cell">
                      <div className="res-summary-label">Reserved Components</div>
                      <div className="res-summary-value">{formatQty(reservations.reservedComponents)}</div>
                    </div>
                    <div className="res-summary-cell">
                      <div className="res-summary-label">Reserved PI</div>
                      <div className="res-summary-value">{formatQty(reservations.reservedPi)}</div>
                    </div>
                    <div className="res-summary-cell">
                      <div className="res-summary-label">Missing Reservations</div>
                      <div className={`res-summary-value ${reservations.missingReservations > 0 ? "furnace" : "nominal"}`}>
                        {reservations.missingReservations}
                      </div>
                    </div>
                  </div>
                </Panel>

                <div style={{ height: 14 }} />
              </>
            )}

            {blueprintReadiness && (
              <>
                <Panel
                  title="Blueprints"
                  keel={blueprintReadiness.missingCount > 0 ? "alert" : blueprintReadiness.warningCount > 0 ? "furnace" : "nominal"}
                  className="dash-hero"
                >
                  {blueprintReadiness.required.length === 0 ? (
                    <p className="ph-mission">No blueprint requirements recorded for this operation.</p>
                  ) : (
                    <>
                      <div className="res-summary-grid" style={{ marginBottom: 14 }}>
                        <div className="res-summary-cell">
                          <div className="res-summary-label">Required</div>
                          <div className="res-summary-value">{blueprintReadiness.required.length}</div>
                        </div>
                        <div className="res-summary-cell">
                          <div className="res-summary-label">Owned</div>
                          <div className="res-summary-value nominal">{blueprintReadiness.ownedCount}</div>
                        </div>
                        <div className="res-summary-cell">
                          <div className="res-summary-label">Missing</div>
                          <div className={`res-summary-value ${blueprintReadiness.missingCount > 0 ? "alert" : "nominal"}`}>
                            {blueprintReadiness.missingCount}
                          </div>
                        </div>
                        <div className="res-summary-cell">
                          <div className="res-summary-label">Research/Copy Warnings</div>
                          <div className={`res-summary-value ${blueprintReadiness.warningCount > 0 ? "furnace" : "nominal"}`}>
                            {blueprintReadiness.warningCount}
                          </div>
                        </div>
                      </div>
                      <div className="bp-req-list">
                        {blueprintReadiness.required.map((req) => (
                          <div className="bp-req-row" key={req.typeName}>
                            <div>{req.typeName}</div>
                            <div className="bp-req-reason">{req.reason}</div>
                            <div
                              className={`op-status ${
                                !req.isOwned ? "alert" : req.warningStatus ? "furnace" : "nominal"
                              }`}
                            >
                              {!req.isOwned ? "Missing" : req.warningStatus ? req.warningStatus.replace(/_/g, " ") : "Owned"}
                            </div>
                          </div>
                        ))}
                      </div>
                    </>
                  )}
                </Panel>

                <div style={{ height: 14 }} />
              </>
            )}

            <Panel title="Reserved for future sprints" keel="coolant">
              <p className="ph-mission">
                Every operation will eventually carry a production plan, an inventory view scoped to what it has
                reserved, a generated shopping list, and running cost tracking. These sections exist as
                architectural placeholders only — no implementation yet.
              </p>
              <div className="ops-reserved-grid">
                {RESERVED_SECTIONS.map((s) => (
                  <div className="ops-reserved-card" key={s}>
                    <div className="ops-reserved-label">{s}</div>
                    <div className="ops-reserved-note">reserved</div>
                  </div>
                ))}
              </div>
            </Panel>
          </div>
        )}
      </div>
    </div>
  );
}

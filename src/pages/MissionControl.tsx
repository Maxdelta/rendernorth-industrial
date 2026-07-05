import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  getMissionControl,
  listBuildTargets,
  selectBuildTarget,
  getOperationsDashboard,
  getInventoryCommitment,
  getReservationConflicts,
  healthCheck,
  formatIsk,
  formatQty,
  type MissionControl,
  type BuildTargetSummary,
  type OperationsDashboard,
  type OperationSummary,
  type InventoryCommitment,
  type ReservationConflict,
  type DbHealth,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";
import { TierGauge } from "../components/TierGauge";
import { SegGauge } from "../components/SegGauge";
import { HullSchematic } from "../components/HullSchematic";
import { TargetPicker } from "../components/TargetPicker";

function healthTone(status: string): "nominal" | "furnace" | "alert" {
  if (status === "Blocked") return "alert";
  if (status === "Degraded") return "furnace";
  return "nominal";
}

function opStatusTone(op: OperationSummary): "nominal" | "furnace" | "alert" | "coolant" {
  if (op.isBlocked) return "alert";
  if (op.status === "active") return "nominal";
  if (op.status === "planned") return "coolant";
  return "furnace";
}

export function MissionControlPage() {
  const [mission, setMission] = useState<MissionControl | null>(null);
  const [ops, setOps] = useState<OperationsDashboard | null>(null);
  const [commitment, setCommitment] = useState<InventoryCommitment | null>(null);
  const [conflicts, setConflicts] = useState<ReservationConflict[] | null>(null);
  const [targets, setTargets] = useState<BuildTargetSummary[]>([]);
  const [health, setHealth] = useState<DbHealth | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);

  useEffect(() => {
    let live = true;
    getMissionControl().then((m) => live && setMission(m));
    getOperationsDashboard().then((o) => live && setOps(o));
    getInventoryCommitment().then((c) => live && setCommitment(c));
    getReservationConflicts().then((c) => live && setConflicts(c));
    listBuildTargets().then((t) => live && setTargets(t));
    healthCheck().then((h) => live && setHealth(h));
    return () => {
      live = false;
    };
  }, []);

  async function handleSelect(projectId: number) {
    const next = await selectBuildTarget(projectId);
    setMission(next);
    setTargets(await listBuildTargets());
    setPickerOpen(false);
  }

  if (!mission) {
    return <div className="data-source">Bringing Mission Control online…</div>;
  }

  const target = mission.selectedTarget;
  const fh = mission.factoryHealth;
  const overallPct = Math.round(target.overallProgress * 100);
  const tone = healthTone(fh.status);

  return (
    <div className="dash">
      <div className="stat-strip">
        <StatCard label="Running Jobs" value={String(mission.runningJobs)} tone="coolant" keel="coolant" note="industry slots active" />
        <StatCard
          label="Idle Characters"
          value={String(mission.idleCharacters)}
          tone={mission.idleCharacters > 0 ? "furnace" : "nominal"}
          keel={mission.idleCharacters > 0 ? "furnace" : "nominal"}
          note="unassigned production capacity"
        />
        <StatCard
          label="Idle BPOs"
          value={String(mission.idleBpos)}
          tone={mission.idleBpos > 0 ? "furnace" : "nominal"}
          keel={mission.idleBpos > 0 ? "furnace" : "nominal"}
          note="researched originals not in a job"
        />
        <StatCard label="Wallet" value={formatIsk(mission.walletIsk)} tone="nominal" keel="nominal" note="liquid across all characters" />
      </div>

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
          </Panel>

          <Panel title="Priority Queue" keel="furnace" className="dash-half">
            <div className="op-simple-list">
              {ops.priorityQueue.map((op) => (
                <Link className="op-simple-row" to={`/operations?op=${op.operationId}`} key={op.operationId}>
                  <span className="op-simple-priority">P{op.priority}</span>
                  <span className="op-simple-goal">{op.goal}</span>
                  <span className={`op-status ${opStatusTone(op)}`}>{op.isBlocked ? "Blocked" : op.status}</span>
                </Link>
              ))}
            </div>
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

      <Panel title="Factory Health" keel={tone} className="dash-hero">
        <div className="health-grid">
          <div className="health-cell">
            <div className="stat-label">Health</div>
            <div className={`stat-value ${tone}`}>{Math.round(fh.health * 100)}%</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">Status</div>
            <div className={`health-status ${tone}`}>{fh.status}</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">Idle Slots</div>
            <div className={`stat-value ${fh.idleSlots > 0 ? "furnace" : "nominal"}`}>{fh.idleSlots}</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">Blocked Jobs</div>
            <div className={`stat-value ${fh.blockedJobs > 0 ? "alert" : "nominal"}`}>{fh.blockedJobs}</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">Missing Inputs</div>
            <div className={`stat-value ${fh.missingInputs > 0 ? "alert" : "nominal"}`}>{fh.missingInputs}</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">ISK Locked in Jobs</div>
            <div className="stat-value coolant">{formatIsk(fh.iskLockedInJobs)}</div>
          </div>
          <div className="health-cell">
            <div className="stat-label">Projected Finish</div>
            <div className="stat-value">{fh.projectedFinishDays}d</div>
          </div>
        </div>
      </Panel>

      <Panel
        title="Current Operation"
        keel="furnace"
        className="dash-hero"
        headerRight={
          <button className="target-select" onClick={() => setPickerOpen((v) => !v)}>
            {pickerOpen ? "Close" : "Change target"}
          </button>
        }
      >
        {pickerOpen && <TargetPicker targets={targets} onSelect={handleSelect} />}
        <div className="titan-grid">
          <div className="schematic-wrap">
            <HullSchematic progress={target.overallProgress} />
            <div className="schematic-caption">Build target · drydock elevation</div>
          </div>
          <div>
            <div className="stat-label">Selected Build Target</div>
            <div className="titan-name">{target.name}</div>
            <div className="titan-class">{target.className}</div>
            <div className="overall-row">
              <div className="overall-pct">{overallPct}%</div>
              <div className="overall-label">
                Inventory Coverage
                <div className="overall-source">via Inventory Engine</div>
              </div>
            </div>
            <SegGauge progress={target.overallProgress} />
            <div className="tiers">
              {target.tiers.map((tier) => (
                <TierGauge key={tier.label} label={tier.label} coverage={tier.coverage} />
              ))}
            </div>
          </div>
        </div>
      </Panel>

      <Panel title="Missing Inputs" keel={mission.missingMaterials.length > 0 ? "alert" : "nominal"} className="dash-half">
        {mission.missingMaterials.length === 0 ? (
          <p className="ph-mission">No missing inputs for this target. All requirements are covered.</p>
        ) : (
          <div className="mat-list">
            {mission.missingMaterials.map((mat) => (
              <div className="mat-row" key={mat.name}>
                <div className="mat-qty">{formatQty(mat.quantity)}</div>
                <div className="mat-name">{mat.name}</div>
                <div className="mat-tag">{mat.category}</div>
              </div>
            ))}
          </div>
        )}
      </Panel>

      <Panel title="Next Recommendation" keel="furnace" className="dash-half">
        <div className="rec-title">{mission.recommendation.title}</div>
        <p className="rec-reason">{mission.recommendation.reason}</p>
        <div className="rec-rule">{mission.recommendation.ruleId}</div>
        {mission.recommendation.inputs && (
          <div className="rec-inputs">
            {Object.entries(mission.recommendation.inputs).map(([k, v]) => (
              <span className="rec-input" key={k}>
                {k}={String(v)}
              </span>
            ))}
          </div>
        )}
      </Panel>

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        Data source: <b>{mission.source}</b>
        {health && (
          <>
            {" · "}db <b>{health.ok ? "connected" : "error"}</b> · schema v<b>{health.schemaVersion}</b> · {health.dbPath}
          </>
        )}
        {!health && mission.source === "mock" && <> · running in browser without the Tauri shell</>}
      </div>
    </div>
  );
}

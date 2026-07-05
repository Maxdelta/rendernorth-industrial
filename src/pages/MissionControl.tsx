import { useEffect, useState } from "react";
import {
  getMissionControl,
  listBuildTargets,
  selectBuildTarget,
  healthCheck,
  formatIsk,
  formatQty,
  type MissionControl,
  type BuildTargetSummary,
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

export function MissionControlPage() {
  const [mission, setMission] = useState<MissionControl | null>(null);
  const [targets, setTargets] = useState<BuildTargetSummary[]>([]);
  const [health, setHealth] = useState<DbHealth | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);

  useEffect(() => {
    let live = true;
    getMissionControl().then((m) => live && setMission(m));
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

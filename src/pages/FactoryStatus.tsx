import { useEffect, useState } from "react";
import {
  getFactoryStatus,
  healthCheck,
  formatIsk,
  formatQty,
  type FactoryStatus as FactoryStatusDto,
  type DbHealth,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";
import { TierGauge } from "../components/TierGauge";
import { SegGauge } from "../components/SegGauge";
import { HullSchematic } from "../components/HullSchematic";

export function FactoryStatusPage() {
  const [status, setStatus] = useState<FactoryStatusDto | null>(null);
  const [health, setHealth] = useState<DbHealth | null>(null);

  useEffect(() => {
    let live = true;
    getFactoryStatus().then((s) => live && setStatus(s));
    healthCheck().then((h) => live && setHealth(h));
    return () => {
      live = false;
    };
  }, []);

  if (!status) {
    return <div className="data-source">Bringing factory telemetry online…</div>;
  }

  const target = status.selectedTarget;
  const overallPct = Math.round(target.overallProgress * 100);

  return (
    <div className="dash">
      <div className="stat-strip">
        <StatCard label="Running Jobs" value={String(status.runningJobs)} tone="coolant" keel="coolant" note="industry slots active" />
        <StatCard
          label="Idle Characters"
          value={String(status.idleCharacters)}
          tone={status.idleCharacters > 0 ? "furnace" : "nominal"}
          keel={status.idleCharacters > 0 ? "furnace" : "nominal"}
          note="unassigned production capacity"
        />
        <StatCard
          label="Idle BPOs"
          value={String(status.idleBpos)}
          tone={status.idleBpos > 0 ? "furnace" : "nominal"}
          keel={status.idleBpos > 0 ? "furnace" : "nominal"}
          note="researched originals not in a job"
        />
        <StatCard label="Wallet" value={formatIsk(status.walletIsk)} tone="nominal" keel="nominal" note="liquid across all characters" />
      </div>

      <Panel
        title="Selected Build Target"
        keel="furnace"
        className="dash-hero"
        headerRight={
          <button className="target-select" disabled title="Target selection activates with the Build Target Engine (Sprint 004)">
            Change target
          </button>
        }
      >
        <div className="titan-grid">
          <div className="schematic-wrap">
            <HullSchematic progress={target.overallProgress} />
            <div className="schematic-caption">Hull fit-out · drydock elevation</div>
          </div>
          <div>
            <div className="titan-name">{target.name}</div>
            <div className="titan-class">{target.className}</div>
            <div className="overall-row">
              <div className="overall-pct">{overallPct}%</div>
              <div className="overall-label">Overall progress</div>
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

      <Panel title="Missing Materials" keel="alert" className="dash-half">
        <div className="mat-list">
          {status.missingMaterials.map((mat) => (
            <div className="mat-row" key={mat.name}>
              <div className="mat-qty">{formatQty(mat.quantity)}</div>
              <div className="mat-name">{mat.name}</div>
              <div className="mat-tag">{mat.category}</div>
            </div>
          ))}
        </div>
      </Panel>

      <Panel title="Next Recommendation" keel="furnace" className="dash-half">
        <div className="rec-title">{status.recommendation.title}</div>
        <p className="rec-reason">{status.recommendation.reason}</p>
        <div className="rec-rule">{status.recommendation.ruleId}</div>
      </Panel>

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        Data source: <b>{status.source}</b>
        {health && (
          <>
            {" · "}db <b>{health.ok ? "connected" : "error"}</b> · schema v<b>{health.schemaVersion}</b> · {health.dbPath}
          </>
        )}
        {!health && status.source === "mock" && <> · running in browser without the Tauri shell</>}
      </div>
    </div>
  );
}
